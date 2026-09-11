use super::*;

#[test]
fn compares_release_versions_with_optional_v_prefix() {
    assert!(is_newer_version("0.7.0", "0.6.0").unwrap());
    assert!(is_newer_version("v0.7.0", "0.6.0").unwrap());
    assert!(!is_newer_version("0.6.0", "v0.6.0").unwrap());
    assert!(!is_newer_version("0.5.9", "0.6.0").unwrap());
}

#[test]
fn uses_proxy_only_for_download_assets() {
    let original = "https://github.com/XiaoPb/combridge/releases/download/v0.7.0/ComBridge_0.7.0_x64_en-US.msi";
    assert_eq!(
        api_url(),
        "https://api.github.com/repos/XiaoPb/combridge/releases/latest"
    );
    assert_eq!(
        proxy_urls(original, &["https://v4.gh-proxy.org".to_string()])[0],
        format!("https://v4.gh-proxy.org/{original}")
    );
    assert!(!api_url().contains("gh-proxy"));
}

#[test]
fn selects_platform_asset() {
    let assets = vec![
        ReleaseAsset {
            name: "ComBridge_0.7.0_x64_en-US.exe".into(),
            browser_download_url: "https://example.com/exe".into(),
            size: 10,
        },
        ReleaseAsset {
            name: "ComBridge_0.7.0_x64_en-US.msi".into(),
            browser_download_url: "https://example.com/msi".into(),
            size: 20,
        },
        ReleaseAsset {
            name: "ComBridge_0.7.0_arm64.msi".into(),
            browser_download_url: "https://example.com/arm".into(),
            size: 30,
        },
    ];
    let selected = select_asset(&assets, Platform::WindowsX64, &[]).unwrap();
    assert_eq!(selected.name, "ComBridge_0.7.0_x64_en-US.msi");
    assert!(select_asset(&assets, Platform::LinuxX64, &[]).is_err());
}

#[test]
fn accepts_single_platform_asset_without_architecture_token() {
    let assets = vec![ReleaseAsset {
        name: "ComBridge_0.7.0.dmg".into(),
        browser_download_url: "https://example.com/mac".into(),
        size: 40,
    }];
    let selected = select_asset(&assets, Platform::MacOsX64, &[]).unwrap();
    assert_eq!(selected.name, "ComBridge_0.7.0.dmg");
}

#[test]
fn rejects_unsafe_asset_names() {
    assert!(validate_asset_name("ComBridge.msi").is_ok());
    assert!(validate_asset_name("../ComBridge.msi").is_err());
    assert!(validate_asset_name("nested/ComBridge.msi").is_err());
}

#[test]
fn loads_proxy_nodes_from_yaml_and_normalizes_trailing_slashes() {
    let yaml = r#"
proxies:
  - https://custom-one.example/
  - https://custom-two.example
"#;
    let nodes = proxy_nodes_from_yaml(yaml).unwrap();
    assert_eq!(
        nodes,
        vec![
            "https://custom-one.example".to_string(),
            "https://custom-two.example".to_string()
        ]
    );
}

#[test]
fn proxy_urls_follow_yaml_order_before_direct_url() {
    let nodes = vec![
        "https://first.example".to_string(),
        "https://second.example".to_string(),
    ];
    let urls = proxy_urls(
        "https://github.com/XiaoPb/combridge/releases/download/v0.7.0/file.msi",
        &nodes,
    );
    assert_eq!(urls[0], "https://first.example/https://github.com/XiaoPb/combridge/releases/download/v0.7.0/file.msi");
    assert_eq!(urls[1], "https://second.example/https://github.com/XiaoPb/combridge/releases/download/v0.7.0/file.msi");
}

#[test]
fn default_proxy_nodes_are_stable_and_ordered() {
    assert_eq!(
        DEFAULT_PROXY_NODES,
        &[
            "https://v4.gh-proxy.org",
            "https://gh-proxy.org",
            "https://v6.gh-proxy.org",
        ]
    );
}

#[test]
fn empty_proxy_config_falls_back_to_default_nodes() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join(PROXY_CONFIG_FILE_NAME);
    std::fs::write(&path, "proxies: []\n").unwrap();

    let nodes = load_proxy_nodes(&path).unwrap();
    assert_eq!(
        nodes,
        DEFAULT_PROXY_NODES
            .iter()
            .map(|node| node.to_string())
            .collect::<Vec<_>>()
    );
}

#[test]
fn invalid_proxy_config_is_reported() {
    let error = proxy_nodes_from_yaml("proxies: [").unwrap_err();
    assert!(error.contains("invalid update proxy config"));
}
