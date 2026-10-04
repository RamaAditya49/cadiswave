mod process {
    pub use cadiswave_runtime::process::*;
}
#[allow(dead_code, reason = "This fixture compiles private production seams.")]
mod device {
    include!("../src/device.rs");
    struct Metadata(Vec<u8>);
    impl Transport for Metadata {
        fn read(&mut self, _: u16, bytes: &mut [u8]) -> Result<usize> {
            bytes.copy_from_slice(&self.0);
            Ok(bytes.len())
        }
        fn write(&mut self, _: u16, _: &[u8]) -> Result<usize> {
            panic!("Metadata reads must not write hardware")
        }
        fn unresponsive(&self) -> bool {
            false
        }
    }
    fn vendor(serial: Option<&str>) -> VendorDevice {
        let profile = ProfileId::WaveXlr;
        let p = profile.profile().legacy.unwrap();
        let mut bytes = vec![0; p.devinfo_len];
        bytes[p.devinfo_api[0]] = 1;
        bytes[p.devinfo_api[1]] = 4;
        for (index, value) in p.devinfo_fw.into_iter().zip([3, 7, 3]) {
            bytes[index] = value;
        }
        bytes[p.devinfo_serial.0..p.devinfo_serial.0 + 9].copy_from_slice(b"vendor-id");
        VendorDevice {
            unit: UnitId {
                profile,
                bus: 1,
                address: 2,
                incarnation: 1,
            },
            transport: Box::new(Metadata(bytes)),
            usb_info: None,
            usb_serial: serial.map(str::to_owned),
        }
    }
    #[test]
    fn legacy_capture_identity_uses_the_usb_descriptor_and_retains_versions() {
        let info = vendor(Some("fixture-usb-serial")).read_info().unwrap();
        assert_eq!(info.serial, "fixture-usb-serial");
        assert_eq!(info.api, "1.4");
        assert_eq!(info.firmware, "3.7.3");
    }
    #[test]
    fn missing_descriptor_cannot_authorize_capture_identity_from_vendor_bytes() {
        assert!(vendor(None).read_info().unwrap().serial.is_empty());
    }
}
