use cadiswave_core::{locale::LanguageChoice, model::Preferences};
use serde_json::json;
#[test]
fn system_language_has_a_deterministic_fallback() {
    for (locale, expected) in [
        ("id_ID.UTF-8", "id"),
        ("id-ID", "id"),
        ("en_US.UTF-8", "en"),
        ("de_DE.UTF-8", "en"),
        ("C", "en"),
    ] {
        assert_eq!(LanguageChoice::System.resolve(locale), expected);
    }
    assert_eq!(LanguageChoice::English.resolve("id_ID.UTF-8"), "en");
    assert_eq!(LanguageChoice::Indonesian.resolve("en_US.UTF-8"), "id");
}
#[test]
fn legacy_preferences_default_to_system_and_keep_extensions() {
    let preferences = Preferences::from_value(json!({"custom":{"keep":true}})).unwrap();
    assert_eq!(preferences.language, LanguageChoice::System);
    assert_eq!(
        serde_json::to_value(preferences).unwrap()["custom"],
        json!({"keep":true})
    );
    for (key, expected) in [
        ("system", LanguageChoice::System),
        ("en", LanguageChoice::English),
        ("id", LanguageChoice::Indonesian),
    ] {
        let prefs = Preferences::from_value(json!({"language":key})).unwrap();
        assert_eq!(prefs.language, expected);
        assert_eq!(serde_json::to_value(prefs).unwrap()["language"], key);
    }
    assert!(Preferences::from_value(json!({"language":"broken"})).is_err());
}
