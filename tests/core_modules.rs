use nm_wifi::{
    types::{App, AppState, WifiNetwork, WifiSecurity},
    ui::{format_ssid_column, get_frequency_band, keybindings_hint, ui},
};
use ratatui::{Terminal, backend::TestBackend};

fn network(ssid: &str, security: WifiSecurity, connected: bool) -> WifiNetwork {
    WifiNetwork {
        ssid: ssid.to_string(),
        signal_strength: 78,
        security,
        frequency: 5180,
        connected,
    }
}

#[test]
fn types_selection_stays_in_sync_in_integration_tests() {
    let mut app = App::new();
    app.state = AppState::NetworkList;
    app.networks = vec![
        network("guest", WifiSecurity::Open, false),
        network("home", WifiSecurity::WpaPsk, true),
    ];

    app.next();
    assert_eq!(app.selected_index, 1);
    assert_eq!(app.list_state.selected(), Some(1));

    app.previous();
    assert_eq!(app.selected_index, 0);
    assert_eq!(app.list_state.selected(), Some(0));
}

#[test]
fn ui_renderer_draws_network_list_screen() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal created");
    let mut app = App::new();
    app.state = AppState::NetworkList;
    app.networks = vec![network("CatCat", WifiSecurity::WpaSae, true)];
    app.network_count = app.networks.len();
    app.adapter_name = Some("demo-wlan0".to_string());

    terminal
        .draw(|frame| ui(frame, &app))
        .expect("render succeeds");

    let buffer = terminal.backend().buffer().clone();
    let mut text = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            text.push_str(buffer[(x, y)].symbol());
        }
        text.push('\n');
    }

    assert!(text.contains("WiFi Networks"));
    assert!(text.contains("demo-wlan0"));
}

#[test]
fn public_ui_helpers_remain_usable_from_integration_tests() {
    assert_eq!(get_frequency_band(2412), "2.4G");
    assert_eq!(get_frequency_band(5180), "5G");
    assert_eq!(keybindings_hint(&AppState::Help), "h/q/Esc Back");
    assert_eq!(format_ssid_column("abc", 5), "abc  ");
}

#[cfg(feature = "demo")]
#[tokio::test]
async fn demo_network_module_scans_and_connects_in_integration_tests() {
    use nm_wifi::network::{
        ConnectionRequest,
        connect_to_network,
        demo_networks,
        scan_wifi_networks,
    };

    let networks = scan_wifi_networks().await.expect("demo scan succeeds");
    assert!(networks.iter().any(|network| network.ssid == "CatCat"));

    let network = demo_networks()
        .into_iter()
        .find(|network| network.security == WifiSecurity::WpaSae)
        .expect("demo WPA3 network exists");

    connect_to_network(ConnectionRequest::Secured {
        network: &network,
        password: "AcerolaAcai",
    })
    .await
    .expect("demo connect succeeds");
}
