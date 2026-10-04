// Exercise the production owner without opening USB or the host audio graph.
mod process {
    pub use cadiswave_runtime::process::*;
}
mod device {
    include!("../src/device.rs");
    fn fixture_lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
        match mutex.lock() {
            Ok(guard) => guard,
            Err(_) => panic!("Dock fixture mutex poisoned"),
        }
    }

    #[test]
    fn bank_selection_requires_every_block_and_never_masks_transport_failure() {
        for preferred_failure in [None, Some(4), Some(5), Some(1)] {
            let mut calls = Vec::new();
            let bank = detect_dock_bank(|bank, selector, bytes| {
                calls.push((bank, selector));
                if bank == 0x0103 && preferred_failure == Some(selector) {
                    return Ok(bytes.len() - 1);
                }
                Ok(bytes.len())
            })
            .unwrap();
            assert_eq!(
                bank,
                if preferred_failure.is_some() {
                    0x0203
                } else {
                    0x0103
                }
            );
            assert_eq!(
                calls
                    .iter()
                    .filter(|(b, _)| *b == bank)
                    .map(|(_, s)| *s)
                    .collect::<Vec<_>>(),
                [4, 5, 1]
            );
        }
        assert_eq!(
            detect_dock_bank(|bank, _, bytes| {
                if bank == 0x0103 {
                    Err(rusb::Error::Pipe)
                } else {
                    Ok(bytes.len())
                }
            })
            .unwrap(),
            0x0203
        );
        assert!(detect_dock_bank(|_, _, _| Err(rusb::Error::Pipe)).is_err());
        assert!(detect_dock_bank(|_, _, bytes| Ok(bytes.len() - 1)).is_err());
        for error in [
            rusb::Error::Io,
            rusb::Error::Timeout,
            rusb::Error::NoDevice,
            rusb::Error::Access,
        ] {
            let mut calls = 0;
            assert!(
                detect_dock_bank(|bank, _, _| {
                    assert_eq!(bank, 0x0103);
                    calls += 1;
                    Err(error)
                })
                .is_err()
            );
            assert_eq!(calls, if error == rusb::Error::Io { 2 } else { 1 });
        }
    }

    #[test]
    fn transient_read_error_retries_only_the_same_bank_and_block() {
        let mut first = true;
        let mut calls = Vec::new();
        assert_eq!(
            detect_dock_bank(|bank, block, bytes| {
                calls.push((bank, block));
                if first {
                    first = false;
                    Err(rusb::Error::Io)
                } else {
                    Ok(bytes.len())
                }
            })
            .unwrap(),
            0x0103
        );
        assert_eq!(calls, [(0x0103, 4), (0x0103, 4), (0x0103, 5), (0x0103, 1)]);
    }

    struct DockMemory {
        blocks: HashMap<u16, Vec<u8>>,
        reads: Vec<u16>,
        writes: Vec<u16>,
        short: Option<u16>,
        fail_write: Option<u16>,
    }
    struct DockTransport(Arc<Mutex<DockMemory>>);
    impl Transport for DockTransport {
        fn read(&mut self, selector: u16, bytes: &mut [u8]) -> Result<usize> {
            let mut memory = fixture_lock(&self.0);
            memory.reads.push(selector);
            let block = &memory.blocks[&selector];
            let length = block.len().min(bytes.len()) - usize::from(memory.short == Some(selector));
            bytes[..length].copy_from_slice(&block[..length]);
            Ok(length)
        }
        fn write(&mut self, selector: u16, bytes: &[u8]) -> Result<usize> {
            let mut memory = fixture_lock(&self.0);
            memory.writes.push(selector);
            if memory.fail_write == Some(selector) {
                return Err(OperationError::unavailable("simulated write failure"));
            }
            memory
                .blocks
                .get_mut(&selector)
                .unwrap()
                .copy_from_slice(bytes);
            Ok(bytes.len())
        }
        fn unresponsive(&self) -> bool {
            false
        }
    }
    fn fixture() -> (SyncedDevice, Arc<Mutex<DockMemory>>) {
        let mut settings = vec![0xa5; 38];
        settings[0] = 30;
        settings[1] = 0xba; // Phantom on, mute off, unrelated DSP flags preserved.
        let memory = Arc::new(Mutex::new(DockMemory {
            blocks: [
                (4, settings),
                (5, vec![48, 0xf5]),
                (1, vec![100, 11, 22, 33, 44, 55]),
            ]
            .into(),
            reads: Vec::new(),
            writes: Vec::new(),
            short: None,
            fail_write: None,
        }));
        let vendor = VendorDevice {
            unit: UnitId {
                profile: ProfileId::XlrDockMk2,
                bus: 3,
                address: 8,
                incarnation: 1,
            },
            transport: Box::new(DockTransport(memory.clone())),
            usb_info: Some(DeviceInfo {
                serial: "FIXTURE".into(),
                api: "Unavailable".into(),
                firmware: "Unavailable".into(),
            }),
        };
        (
            SyncedDevice {
                vendor,
                alsa: None,
                mirror: Mirror::default(),
            },
            memory,
        )
    }

    #[test]
    fn dock_poll_is_read_only_and_controls_preserve_other_blocks_and_phantom() {
        let (mut backend, memory) = fixture();
        let (state, issues) = backend.poll().unwrap();
        assert!(issues.is_empty());
        assert_eq!(
            (state.gain_raw, state.muted, state.hp_volume_db),
            (30, false, -12.0)
        );
        assert_eq!(state.phantom, Some(true));
        assert!(fixture_lock(&memory).writes.is_empty());
        let original = fixture_lock(&memory).blocks.clone();
        let state = backend
            .apply(&[DeviceSetting::GainRaw(31), DeviceSetting::Mute(true)])
            .unwrap();
        assert_eq!(
            (state.gain_raw, state.muted, state.phantom),
            (31, true, Some(true))
        );
        {
            let m = fixture_lock(&memory);
            assert_eq!(m.writes, [4]);
            assert_eq!(&m.blocks[&4][2..], &original[&4][2..]);
            assert_eq!(m.blocks[&4][1], 0xbb);
            assert_eq!(m.blocks[&5], original[&5]);
            assert_eq!(m.blocks[&1], original[&1]);
        }
        let state = backend
            .apply(&[
                DeviceSetting::HeadphoneDb(-12.375),
                DeviceSetting::LowImpedance(true),
            ])
            .unwrap();
        assert_eq!(state.hp_volume_db, -12.5);
        assert_eq!(state.low_impedance, Some(true));
        backend.apply(&[DeviceSetting::MonitorMix(150)]).unwrap();
        let m = fixture_lock(&memory);
        assert_eq!(m.writes, [4, 5, 1]);
        assert_eq!(m.blocks[&5], [50, 0xf7]);
        assert_eq!(m.blocks[&1], [150, 11, 22, 33, 44, 55]);
        assert_eq!(m.blocks[&4][1] & 2, 2);
    }

    #[test]
    fn incomplete_or_invalid_config_cannot_write_and_failed_write_is_not_replayed() {
        for selector in [4, 5, 1] {
            let (mut backend, memory) = fixture();
            fixture_lock(&memory).short = Some(selector);
            assert!(backend.apply(&[DeviceSetting::Mute(true)]).is_err());
            assert!(fixture_lock(&memory).writes.is_empty());
        }
        let (mut backend, memory) = fixture();
        assert!(
            backend
                .apply(&[
                    DeviceSetting::Mute(true),
                    DeviceSetting::HeadphoneDb(f64::NAN)
                ])
                .is_err()
        );
        assert!(fixture_lock(&memory).reads.is_empty());
        assert!(backend.vendor.read_raw(3, &mut [0; 38]).is_err());
        assert!(backend.vendor.read_meters().is_err());
        assert!(fixture_lock(&memory).reads.is_empty());
        fixture_lock(&memory).fail_write = Some(5);
        assert!(
            backend
                .apply(&[
                    DeviceSetting::HeadphoneDb(-20.0),
                    DeviceSetting::MonitorMix(200)
                ])
                .is_err()
        );
        let m = fixture_lock(&memory);
        assert_eq!(m.writes, [5]);
        assert_eq!(m.blocks[&5], [48, 0xf5]);
        assert_eq!(m.blocks[&1][0], 100);
    }
}
