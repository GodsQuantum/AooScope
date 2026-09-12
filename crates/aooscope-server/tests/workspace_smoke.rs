#[test]
fn workspace_version_is_exposed() {
    assert_eq!(aooscope_server::APP_VERSION, "0.3.0-dev");
}
