use std::time::Instant;

use crate::wifi::WifiNetwork;

#[derive(PartialEq)]
pub enum AppState {
    Scanning,
    NetworkList,
    PasswordInput,
    Connecting,
    Disconnecting,
    ConnectionResult,
    Help,
    NetworkDetails,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Connect,
    Disconnect,
}

pub struct App {
    pub networks: Vec<WifiNetwork>,
    pub selected_index: usize,
    pub state: AppState,
    pub password_input: String,
    pub selected_network: Option<WifiNetwork>,
    pub status_message: String,
    pub should_quit: bool,
    pub connection_success: bool,
    pub connection_error: Option<String>,
    pub is_disconnect_operation: bool,
    pub adapter_name: Option<String>,
    pub network_count: usize,
    pub last_scan_time: Option<Instant>,
    pub connection_start_time: Option<Instant>,
    pub password_visible: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    fn set_selected_index(&mut self, index: usize) {
        self.selected_index = index;
    }

    pub fn new() -> App {
        App {
            networks: Vec::new(),
            selected_index: 0,
            state: AppState::Scanning,
            password_input: String::new(),
            selected_network: None,
            status_message: "Scanning for networks...".to_string(),
            should_quit: false,
            connection_success: false,
            connection_error: None,
            is_disconnect_operation: false,
            adapter_name: None,
            network_count: 0,
            last_scan_time: None,
            connection_start_time: None,
            password_visible: false,
        }
    }

    pub fn next(&mut self) {
        if !self.networks.is_empty() {
            let i = if self.selected_index >= self.networks.len() - 1 {
                0
            } else {
                self.selected_index + 1
            };
            self.set_selected_index(i);
        }
    }

    pub fn previous(&mut self) {
        if !self.networks.is_empty() {
            let i = if self.selected_index == 0 {
                self.networks.len() - 1
            } else {
                self.selected_index - 1
            };
            self.set_selected_index(i);
        }
    }

    pub fn selected_network_in_list(&self) -> Option<&WifiNetwork> {
        self.networks.get(self.selected_index)
    }

    pub fn begin_operation(
        &mut self,
        network: WifiNetwork,
        operation: OperationKind,
    ) {
        self.selected_network = Some(network.clone());
        self.is_disconnect_operation = operation == OperationKind::Disconnect;
        self.connection_start_time = Some(Instant::now());
        self.state = match operation {
            OperationKind::Connect => AppState::Connecting,
            OperationKind::Disconnect => AppState::Disconnecting,
        };
        self.status_message = match operation {
            OperationKind::Connect => {
                format!("Connecting to {}...", network.ssid)
            }
            OperationKind::Disconnect => {
                format!("Disconnecting from {}...", network.ssid)
            }
        };
    }

    pub fn activate_selected_network(&mut self) {
        let network = self.selected_network_in_list().cloned();

        match network {
            Some(network) if network.connected => {
                self.begin_operation(network, OperationKind::Disconnect);
            }
            Some(network) if network.is_secured() => {
                self.state = AppState::PasswordInput;
                self.password_input.clear();
                self.selected_network = Some(network);
            }
            Some(network) => {
                self.begin_operation(network, OperationKind::Connect);
            }
            None => {}
        }
    }

    pub fn add_char_to_password(&mut self, c: char) {
        self.password_input.push(c);
    }

    pub fn remove_char_from_password(&mut self) {
        self.password_input.pop();
    }

    pub fn confirm_password(&mut self) {
        if let Some(network) = self.selected_network.clone() {
            self.begin_operation(network, OperationKind::Connect);
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn finish_operation(&mut self, succeeded: bool, error: Option<String>) {
        self.connection_success = succeeded;
        self.connection_error = error;
        self.status_message = match (self.is_disconnect_operation, succeeded) {
            (true, true) => "Disconnected successfully!".to_string(),
            (true, false) => "Disconnection failed".to_string(),
            (false, true) => "Connected successfully!".to_string(),
            (false, false) => "Connection failed".to_string(),
        };
        self.state = AppState::ConnectionResult;
    }

    pub fn back_to_network_list(&mut self) {
        self.state = AppState::NetworkList;
        self.connection_success = false;
        self.connection_error = None;
        self.password_input.clear();
        self.password_visible = false;
        self.is_disconnect_operation = false;
        self.connection_start_time = None;
    }

    pub fn start_scan(&mut self) {
        self.state = AppState::Scanning;
        self.status_message = "Scanning for networks...".to_string();
        self.networks.clear();
        self.network_count = 0;
        self.last_scan_time = None;
        self.set_selected_index(0);
    }

    pub fn handle_scan_error(&mut self, error: impl std::fmt::Display) {
        self.state = AppState::NetworkList;
        self.network_count = self.networks.len();
        self.last_scan_time = None;
        self.status_message =
            format!("Scan failed: {}. Press r to retry.", error);
    }

    pub fn update_selection_after_rescan(&mut self) {
        if let Some(selected_network) = &self.selected_network {
            if let Some(new_index) = self
                .networks
                .iter()
                .position(|n| n.ssid == selected_network.ssid)
            {
                self.set_selected_index(new_index);
            } else {
                self.set_selected_index(0);
            }
        }
        self.selected_network = None;
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::{App, AppState};
    use crate::wifi::{WifiNetwork, WifiSecurity};

    fn network(
        ssid: &str,
        security: WifiSecurity,
        connected: bool,
    ) -> WifiNetwork {
        WifiNetwork {
            ssid: ssid.to_string(),
            signal_strength: 80,
            security,
            frequency: 5180,
            connected,
        }
    }

    fn connected_network(ssid: &str) -> WifiNetwork {
        network(ssid, WifiSecurity::WpaPsk, true)
    }

    #[test]
    fn next_wraps_and_keeps_selection_state_in_sync() {
        let mut app = App::new();
        app.networks =
            vec![connected_network("home"), connected_network("guest")];
        app.selected_index = 1;

        app.next();

        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn previous_wraps_and_keeps_selection_state_in_sync() {
        let mut app = App::new();
        app.networks =
            vec![connected_network("home"), connected_network("guest")];
        app.selected_index = 0;

        app.previous();

        assert_eq!(app.selected_index, 1);
    }

    #[test]
    fn selecting_a_connected_network_starts_disconnect_timing() {
        let mut app = App::new();
        app.state = AppState::NetworkList;
        app.networks = vec![connected_network("home")];

        app.activate_selected_network();

        assert!(matches!(app.state, AppState::Disconnecting));
        assert!(app.connection_start_time.is_some());
    }

    #[test]
    fn activate_selected_network_uses_current_selection_not_just_index_zero() {
        let mut app = App::new();
        app.state = AppState::NetworkList;
        app.networks = vec![
            network("cafe", WifiSecurity::Open, false),
            network("office", WifiSecurity::WpaPsk, false),
        ];
        app.selected_index = 1;

        app.activate_selected_network();

        assert!(matches!(app.state, AppState::PasswordInput));
        assert_eq!(
            app.selected_network
                .as_ref()
                .map(|network| network.ssid.as_str()),
            Some("office")
        );
    }

    #[test]
    fn starting_a_scan_clears_stale_scan_metadata() {
        let mut app = App::new();
        app.state = AppState::NetworkList;
        app.networks = vec![connected_network("home")];
        app.network_count = 3;
        app.last_scan_time = Some(Instant::now());
        app.selected_index = 0;

        app.start_scan();

        assert!(matches!(app.state, AppState::Scanning));
        assert!(app.networks.is_empty());
        assert_eq!(app.network_count, 0);
        assert!(app.last_scan_time.is_none());
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn start_scan_resets_selection_fields_together() {
        let mut app = App::new();
        app.networks =
            vec![connected_network("home"), connected_network("guest")];
        app.selected_index = 1;

        app.start_scan();

        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn update_selection_after_rescan_restores_matching_ssid() {
        let mut app = App::new();
        app.networks =
            vec![connected_network("guest"), connected_network("home")];
        app.selected_network = Some(connected_network("home"));

        app.update_selection_after_rescan();

        assert_eq!(app.selected_index, 1);
        assert!(app.selected_network.is_none());
    }

    #[test]
    fn update_selection_after_rescan_resets_to_first_when_selected_ssid_disappears()
     {
        let mut app = App::new();
        app.selected_index = 1;
        app.networks =
            vec![connected_network("guest"), connected_network("cafe")];
        app.selected_network = Some(connected_network("home"));

        app.update_selection_after_rescan();

        assert_eq!(app.selected_index, 0);
        assert!(app.selected_network.is_none());
    }

    #[test]
    fn scan_failures_keep_the_app_running_with_a_retry_message() {
        let mut app = App::new();
        app.state = AppState::Scanning;

        app.handle_scan_error("dbus unavailable");

        assert!(matches!(app.state, AppState::NetworkList));
        assert_eq!(
            app.status_message,
            "Scan failed: dbus unavailable. Press r to retry."
        );
    }
}
