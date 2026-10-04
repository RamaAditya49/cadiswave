use cadiswave_core::{capabilities::for_profile, profiles::ProfileId};
use cadiswave_core::{
    model::{DeviceSetting, ErrorCode},
    protocol::{ConfigBuffer, KnobMode, validate_setting},
};

#[test]
fn original_xlr_monitor_uses_only_its_verified_word_and_mode() {
    let profile = ProfileId::WaveXlr;
    assert!(for_profile(profile).monitor_mix);
    assert!(!for_profile(ProfileId::WaveXlrMk2).monitor_mix);
    let legacy = profile.profile().legacy.unwrap();
    assert_eq!(
        (legacy.config_len, legacy.wvalue_config, legacy.windex),
        (34, 0, 0x3303)
    );
    let mut original = [0xa5; 34];
    original[12..14].copy_from_slice(&0x3200_u16.to_le_bytes());
    original[14] = 3;
    let mut config = ConfigBuffer::decode(profile, &original).unwrap();
    assert_eq!(config.state().monitor_mix, Some(0x3200));
    assert_eq!(config.state().knob_mode, KnobMode::MonitorMix);
    for (requested, expected) in [
        (0, 0_u16),
        (0x3201, 0x3201),
        (0x6400, 0x6400),
        (u16::MAX, 0x6400),
    ] {
        config.apply(DeviceSetting::MonitorMix(requested)).unwrap();
        let mut bytes = original;
        bytes[12..14].copy_from_slice(&expected.to_le_bytes());
        assert_eq!(config.as_bytes(), bytes);
        assert_eq!(config.state().monitor_mix, Some(expected));
    }
    let mut unknown = ConfigBuffer::decode(ProfileId::WaveXlrMk2, &original).unwrap();
    assert_eq!(unknown.state().monitor_mix, None);
    assert_eq!(unknown.state().knob_mode, KnobMode::None);
    assert_eq!(
        unknown
            .apply(DeviceSetting::MonitorMix(0x3200))
            .unwrap_err()
            .code,
        ErrorCode::Unsupported
    );
    assert_eq!(unknown.as_bytes(), original);
}

#[test]
fn hardware_processing_is_available_only_on_mapped_profiles() {
    for (profile, clipguard, low_cut) in [
        (ProfileId::WaveXlr, false, false),
        (ProfileId::WaveXlrMk2, false, false),
        (ProfileId::Wave3, true, false),
        (ProfileId::XlrDockMk2, true, true),
    ] {
        let capabilities = for_profile(profile);
        assert_eq!(capabilities.clipguard, clipguard, "{profile}");
        assert_eq!(capabilities.hardware_low_cut, low_cut, "{profile}");
        assert!(!capabilities.led);
        assert!(!capabilities.persistence);
    }
}

#[test]
fn wave3_clipguard_requires_a_reviewed_api() {
    for api in ["5.3", "5.4"] {
        assert!(cadiswave_core::capabilities::clipguard_available(
            ProfileId::Wave3,
            api
        ));
    }
    for api in [
        "",
        "Unavailable",
        "1.0",
        "5.2",
        "5.5",
        "6.3",
        "5.03",
        "5.3 ",
    ] {
        assert!(!cadiswave_core::capabilities::clipguard_available(
            ProfileId::Wave3,
            api
        ));
        assert!(cadiswave_core::capabilities::clipguard_available(
            ProfileId::XlrDockMk2,
            api
        ));
    }
    for profile in [ProfileId::WaveXlr, ProfileId::WaveXlrMk2] {
        assert!(!cadiswave_core::capabilities::clipguard_available(
            profile, "5.3"
        ));
    }
}

#[test]
fn dock_processing_preserves_other_bits_and_blocks() {
    let original = [0xa7; 38];
    let headphones = [47, 0xef];
    let monitor = [150, 11, 22, 33, 44, 55];
    let mut config = ConfigBuffer::decode_dock(&original, &headphones, &monitor).unwrap();
    for (clipguard, low_cut, flags, clip_flags) in
        [(true, true, 0xb7, 0xa3), (false, false, 0xa7, 0xa7)]
    {
        config.apply(DeviceSetting::Clipguard(clipguard)).unwrap();
        config
            .apply(DeviceSetting::HardwareLowCut(low_cut))
            .unwrap();
        let mut expected = original;
        expected[1] = flags;
        expected[2] = clip_flags;
        assert_eq!(
            config.blocks().collect::<Vec<_>>(),
            vec![
                (4, expected.as_slice()),
                (5, headphones.as_slice()),
                (1, monitor.as_slice())
            ]
        );
        assert_eq!(config.state().clipguard, Some(clipguard));
        assert_eq!(config.state().hardware_low_cut, Some(low_cut));
    }
}

#[test]
fn wave3_clipguard_preserves_monitor_and_reserved_bytes() {
    let original = [0x55; 16];
    let mut config = ConfigBuffer::decode(ProfileId::Wave3, &original).unwrap();
    for (enabled, byte) in [(true, 1), (false, 0)] {
        config.apply(DeviceSetting::Clipguard(enabled)).unwrap();
        let mut expected = original;
        expected[5] = byte;
        assert_eq!(config.as_bytes(), expected);
        assert_eq!(config.state().clipguard, Some(enabled));
        assert_eq!(config.state().hardware_low_cut, None);
    }
}

#[test]
fn unmapped_processing_is_rejected_before_any_config_change() {
    for profile in [ProfileId::WaveXlr, ProfileId::WaveXlrMk2, ProfileId::Wave3] {
        let bytes = vec![0x67; profile.profile().legacy.unwrap().config_len];
        let mut config = ConfigBuffer::decode(profile, &bytes).unwrap();
        for enabled in [false, true] {
            let mut settings = vec![DeviceSetting::HardwareLowCut(enabled)];
            if profile != ProfileId::Wave3 {
                settings.push(DeviceSetting::Clipguard(enabled));
            }
            for setting in settings {
                assert_eq!(
                    validate_setting(profile, setting).unwrap_err().code,
                    ErrorCode::Unsupported
                );
                assert_eq!(
                    config.apply(setting).unwrap_err().code,
                    ErrorCode::Unsupported
                );
                assert_eq!(config.as_bytes(), bytes);
            }
        }
        if profile != ProfileId::Wave3 {
            assert_eq!(config.state().clipguard, None);
        }
        assert_eq!(config.state().hardware_low_cut, None);
    }
}
