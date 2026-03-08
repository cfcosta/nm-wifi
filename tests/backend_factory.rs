#[cfg(not(feature = "demo"))]
use nm_wifi::backend::NetworkBackend;
use nm_wifi::backend::default_backend;

#[cfg(feature = "demo")]
#[tokio::test]
async fn default_backend_uses_demo_implementation_when_demo_feature_is_enabled()
{
    let backend = default_backend();

    assert_eq!(
        backend.adapter_name().expect("adapter lookup works"),
        Some("demo-wlan0".to_string())
    );

    let networks = backend.scan_networks().await.expect("scan works");
    assert!(networks.iter().any(|network| network.ssid == "CatCat"));
}

#[cfg(not(feature = "demo"))]
#[test]
fn default_backend_factory_is_available_in_non_demo_builds() {
    let _backend: Box<dyn NetworkBackend> = default_backend();
}
