use cadiswave_core::{
    effects::FxSettings,
    model::Preferences,
    voice_presets::{BuiltinPreset, VoicePresets},
};
use serde_json::json;

#[test]
fn built_in_presets_have_valid_distinct_effects() {
    let presets = [
        BuiltinPreset::Meeting,
        BuiltinPreset::Podcast,
        BuiltinPreset::Streaming,
    ];
    let settings: Vec<_> = presets.iter().map(|preset| preset.settings()).collect();
    for value in &settings {
        assert_eq!(value.validated().unwrap(), *value);
        assert!(value.active());
        assert_eq!(value.delay_ms, 0.0);
    }
    assert_ne!(settings[0], settings[1]);
    assert_ne!(settings[1], settings[2]);
}

#[test]
fn user_presets_round_trip_without_changing_other_preferences() {
    let mut prefs =
        Preferences::from_value(json!({"language":"id", "custom":{"keep":true}})).unwrap();
    let settings = BuiltinPreset::Podcast.settings();
    prefs
        .voice_presets
        .save("Suara saya", settings.clone())
        .unwrap();
    let value = serde_json::to_value(&prefs).unwrap();
    let restored = Preferences::from_value(value).unwrap();
    assert_eq!(restored.voice_presets.get("Suara saya"), Some(&settings));
    assert_eq!(restored.extra["custom"], json!({"keep":true}));
    assert_eq!(restored.language, prefs.language);
}

#[test]
fn invalid_names_and_effects_cannot_change_the_preset_store() {
    let mut presets = VoicePresets::default();
    presets.save("Voice", FxSettings::default()).unwrap();
    let before = presets.clone();
    for name in ["", "   ", " Voice", "Voice ", "bad\nname", &"a".repeat(49)] {
        assert!(
            presets.save(name, FxSettings::default()).is_err(),
            "{name:?}"
        );
        assert_eq!(presets, before);
    }
    let invalid = FxSettings {
        lowcut: 100,
        ..Default::default()
    };
    assert!(presets.save("Invalid", invalid).is_err());
    assert_eq!(presets, before);
    assert!(presets.delete("missing").is_err());
    assert_eq!(presets, before);
    presets.delete("Voice").unwrap();
    assert_eq!(presets.len(), 0);
}

#[test]
fn malformed_presets_fail_preferences_decode() {
    for data in [
        json!([]),
        json!({"bad\nname":{}}),
        json!({"Voice":{"lowcut":100}}),
        json!({"Voice":false}),
    ] {
        assert!(Preferences::from_value(json!({"voice_presets": data})).is_err());
    }
    let mut presets = VoicePresets::default();
    for index in 0..32 {
        presets
            .save(&format!("Voice {index}"), FxSettings::default())
            .unwrap();
    }
    assert!(presets.save("One more", FxSettings::default()).is_err());
    presets
        .save("Voice 0", BuiltinPreset::Meeting.settings())
        .unwrap();
    assert_eq!(presets.len(), 32);
}
