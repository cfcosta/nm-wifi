mod format;
mod header_footer;
mod list;
mod modals;
mod screen;

pub use format::{
    create_signal_graph,
    format_signal_strength,
    format_ssid_column,
    get_frequency_band,
};
pub use header_footer::{keybindings_hint, render_header, render_status_bar};
pub use list::create_network_list_item;
pub use modals::{
    centered_rect,
    render_enhanced_connecting_modal,
    render_enhanced_disconnecting_modal,
    render_enhanced_password_modal,
    render_enhanced_result_modal,
    render_help_screen,
    render_network_details,
};
pub use screen::ui;

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};
    use unicode_width::UnicodeWidthStr;

    use super::{format_ssid_column, get_frequency_band, keybindings_hint, ui};
    use crate::{
        app_state::{App, AppState},
        wifi::{WifiNetwork, WifiSecurity},
    };

    fn network(
        ssid: &str,
        security: WifiSecurity,
        connected: bool,
    ) -> WifiNetwork {
        WifiNetwork {
            ssid: ssid.to_string(),
            signal_strength: 78,
            security,
            frequency: 5180,
            connected,
        }
    }

    fn render_text(app: &App) -> String {
        let backend = TestBackend::new(120, 36);
        let mut terminal = Terminal::new(backend).expect("terminal created");
        terminal
            .draw(|frame| ui(frame, app))
            .expect("render succeeds");

        let buffer = terminal.backend().buffer().clone();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }
        text
    }

    #[test]
    fn connecting_and_disconnecting_hints_show_only_quit_action() {
        assert_eq!(keybindings_hint(&AppState::Connecting), "Esc Quit");
        assert_eq!(keybindings_hint(&AppState::Disconnecting), "Esc Quit");
    }

    #[test]
    fn connection_result_hint_matches_available_actions() {
        assert_eq!(
            keybindings_hint(&AppState::ConnectionResult),
            "Enter Return  q/Esc Quit"
        );
    }

    #[test]
    fn network_list_hint_matches_connect_and_disconnect_behavior() {
        assert_eq!(
            keybindings_hint(&AppState::NetworkList),
            "↑↓/jk Move  Enter Connect  d Disconnect  r Rescan  i Info  h Help  q Quit"
        );
    }

    #[test]
    fn six_ghz_networks_are_labeled_correctly() {
        assert_eq!(get_frequency_band(5975), "6G");
    }

    #[test]
    fn ssid_column_uses_terminal_display_width() {
        let formatted = format_ssid_column("網😊", 6);
        assert_eq!(UnicodeWidthStr::width(formatted.as_str()), 6);
    }

    #[test]
    fn password_modal_masks_and_reveals_input() {
        let mut hidden_app = App::new();
        hidden_app.state = AppState::PasswordInput;
        hidden_app.networks =
            vec![network("CatCat", WifiSecurity::WpaSae, false)];
        hidden_app.selected_network =
            Some(network("CatCat", WifiSecurity::WpaSae, false));
        hidden_app.password_input = "hunter2".to_string();
        hidden_app.password_visible = false;

        let hidden_text = render_text(&hidden_app);
        assert!(hidden_text.contains("Password"));
        assert!(hidden_text.contains("•••••••"));
        assert!(!hidden_text.contains("hunter2"));

        hidden_app.password_visible = true;
        let visible_text = render_text(&hidden_app);
        assert!(visible_text.contains("hunter2"));
    }

    #[test]
    fn operation_modals_render_titles_and_network_summary() {
        let mut app = App::new();
        let network = network("CatCat", WifiSecurity::WpaSae, false);
        app.networks = vec![network.clone()];
        app.selected_network = Some(network.clone());

        app.state = AppState::Connecting;
        let connecting_text = render_text(&app);
        assert!(connecting_text.contains("Connecting"));
        assert!(connecting_text.contains("Network: CatCat"));
        assert!(connecting_text.contains("Security: WPA3 Personal"));
        assert!(connecting_text.contains("Signal: 78% (5G)"));

        app.state = AppState::Disconnecting;
        let disconnecting_text = render_text(&app);
        assert!(disconnecting_text.contains("Disconnecting"));
        assert!(disconnecting_text.contains("Network: CatCat"));
        assert!(disconnecting_text.contains("Security: WPA3 Personal"));
        assert!(
            disconnecting_text.contains("Disconnecting via NetworkManager...")
        );
    }

    #[test]
    fn result_modal_renders_backend_error_and_interface() {
        let mut app = App::new();
        app.state = AppState::ConnectionResult;
        app.selected_network =
            Some(network("CatCat", WifiSecurity::WpaSae, false));
        app.connection_error =
            Some("Failed to find WiFi device in NetworkManager".to_string());
        app.adapter_name = Some("demo-wlan0".to_string());

        let text = render_text(&app);
        assert!(text.contains("Connection failed"));
        assert!(text.contains("Network: CatCat"));
        assert!(text.contains("Interface: demo-wlan0"));
        assert!(text.contains("Failed to find WiFi device in NetworkManager"));
    }
}
