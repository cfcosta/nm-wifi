use nm_wifi::{
    app::{CleanupGuard, begin_disconnect_for_selected_network},
    app_state::{App, AppState},
    wifi::{WifiNetwork, WifiSecurity},
};

fn network(ssid: &str, connected: bool) -> WifiNetwork {
    WifiNetwork {
        ssid: ssid.to_string(),
        signal_strength: 80,
        security: WifiSecurity::WpaPsk,
        frequency: 5180,
        connected,
    }
}

#[test]
fn cleanup_guard_runs_cleanup_on_drop() {
    let cleaned = std::cell::Cell::new(false);

    {
        let cleaned = &cleaned;
        let _guard = CleanupGuard::new(|| {
            cleaned.set(true);
        });
    }

    assert!(cleaned.get());
}

#[test]
fn disconnect_shortcut_uses_current_selected_connected_network() {
    let mut app = App::new();
    app.state = AppState::NetworkList;
    app.networks = vec![network("guest", false), network("home", true)];
    app.selected_index = 1;

    begin_disconnect_for_selected_network(&mut app);

    assert!(matches!(app.state, AppState::Disconnecting));
    assert!(app.is_disconnect_operation);
    assert!(app.connection_start_time.is_some());
    assert_eq!(
        app.selected_network
            .as_ref()
            .map(|network| network.ssid.as_str()),
        Some("home")
    );
}

#[test]
fn disconnect_shortcut_ignores_unconnected_selected_network() {
    let mut app = App::new();
    app.state = AppState::NetworkList;
    app.networks = vec![network("guest", false), network("home", true)];
    app.selected_index = 0;

    begin_disconnect_for_selected_network(&mut app);

    assert!(matches!(app.state, AppState::NetworkList));
    assert!(app.selected_network.is_none());
}
