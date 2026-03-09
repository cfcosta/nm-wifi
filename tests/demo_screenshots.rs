use std::error::Error;

#[cfg(feature = "demo")]
use nm_wifi::{
    backend::DemoNetworkBackend,
    demo_screenshots::{demo_shot_apps, write_demo_svgs},
    network::demo_networks,
};
use nm_wifi::{
    backend::{BackendFuture, NetworkBackend},
    demo_screenshots::write_demo_svgs_with_backend,
    theme::CatppuccinColors,
    wifi::{WifiNetwork, WifiSecurity},
};
use ratatui::style::Color;

#[test]
fn theme_palette_exposes_expected_base_colors() {
    assert_eq!(CatppuccinColors::BASE, Color::Rgb(30, 30, 46));
    assert_eq!(CatppuccinColors::TEXT, Color::Rgb(205, 214, 244));
}

#[derive(Clone)]
struct StaticScanBackend {
    networks: Vec<WifiNetwork>,
}

impl NetworkBackend for StaticScanBackend {
    fn connected_ssid(&self) -> Result<Option<String>, Box<dyn Error>> {
        Ok(None)
    }

    fn adapter_name(&self) -> Result<Option<String>, Box<dyn Error>> {
        Ok(None)
    }

    fn scan_networks(
        &self,
    ) -> BackendFuture<'_, Result<Vec<WifiNetwork>, Box<dyn Error>>> {
        let networks = self.networks.clone();
        Box::pin(async move { Ok(networks) })
    }

    fn connect(
        &self,
        _request: nm_wifi::network::ConnectionRequest<'_>,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn disconnect(&self, _network: &WifiNetwork) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

fn network(ssid: &str, security: WifiSecurity, connected: bool) -> WifiNetwork {
    WifiNetwork {
        ssid: ssid.to_string(),
        signal_strength: 78,
        security,
        frequency: 5180,
        connected,
    }
}

#[tokio::test]
async fn backend_driven_screenshot_generation_rejects_invalid_fixture_shapes() {
    let cases = vec![
        ("empty", Vec::new(), "at least one network"),
        (
            "no-connected",
            vec![network("guest", WifiSecurity::WpaSae, false)],
            "at least one connected network",
        ),
        (
            "no-unconnected",
            vec![network("home", WifiSecurity::WpaSae, true)],
            "at least one unconnected network",
        ),
        (
            "no-secured-unconnected",
            vec![
                network("home", WifiSecurity::WpaSae, true),
                network("guest", WifiSecurity::Open, false),
            ],
            "at least one secured unconnected network",
        ),
    ];

    for (name, networks, expected_message) in cases {
        let mut output_dir = std::env::temp_dir();
        output_dir.push(format!(
            "nm-wifi-invalid-screens-{}-{}",
            std::process::id(),
            name
        ));
        let _ = std::fs::remove_dir_all(&output_dir);

        let backend = StaticScanBackend { networks };
        let error =
            write_demo_svgs_with_backend(output_dir.as_path(), &backend)
                .await
                .expect_err("invalid fixture shape should be rejected");
        assert!(error.to_string().contains(expected_message));

        let _ = std::fs::remove_dir_all(&output_dir);
    }
}

#[cfg(feature = "demo")]
#[test]
fn demo_screenshot_manifest_includes_error_and_disconnect_flows() {
    let names: Vec<_> = demo_shot_apps(&demo_networks())
        .into_iter()
        .map(|(name, _)| name)
        .collect();

    assert!(names.contains(&"disconnecting.svg"));
    assert!(names.contains(&"result-error.svg"));
}

#[cfg(feature = "demo")]
#[test]
fn screenshot_writer_creates_svg_files() {
    let mut output_dir = std::env::temp_dir();
    output_dir.push(format!("nm-wifi-screens-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&output_dir);

    write_demo_svgs(output_dir.as_path(), &demo_networks())
        .expect("screenshot generation succeeds");

    let network_list =
        std::fs::read_to_string(output_dir.join("network-list.svg"))
            .expect("network list svg exists");
    assert!(network_list.starts_with("<svg "));
    assert!(network_list.contains("font-family=\"monospace\""));

    let _ = std::fs::remove_dir_all(&output_dir);
}

#[cfg(feature = "demo")]
#[tokio::test]
async fn screenshot_writer_can_load_networks_through_the_backend_trait() {
    let mut output_dir = std::env::temp_dir();
    output_dir.push(format!("nm-wifi-screens-backend-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&output_dir);

    let backend = DemoNetworkBackend;
    write_demo_svgs_with_backend(output_dir.as_path(), &backend)
        .await
        .expect("backend-driven screenshot generation succeeds");

    let network_list =
        std::fs::read_to_string(output_dir.join("network-list.svg"))
            .expect("network list svg exists");
    assert!(network_list.starts_with("<svg "));

    let _ = std::fs::remove_dir_all(&output_dir);
}
