use cadiswave_runtime::paths;
#[test]
fn application_paths_use_cadiswave() {
    assert_eq!(
        paths::config_dir().unwrap().file_name().unwrap(),
        "cadiswave"
    );
    assert_eq!(paths::data_dir().unwrap().file_name().unwrap(), "cadiswave");
}
