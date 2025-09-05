use std::time::Instant;

use ratatui::widgets::ListState;

#[derive(Debug, Clone)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal_strength: u8,
    pub secured: bool,
    pub frequency: u32,
    pub connected: bool,
}

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

pub struct App {
    pub networks: Vec<WifiNetwork>,
    pub selected_index: usize,
    pub list_state: ListState,
    pub state: AppState,
    pub password_input: String,
    pub selected_network: Option<WifiNetwork>,
    pub status_message: String,
    pub should_quit: bool,
    pub connection_success: bool,
    pub connection_error: Option<String>,
    pub is_disconnect_operation: bool,
    pub adapter_info: Option<String>,
    pub network_count: usize,
    pub last_scan_time: Option<Instant>,
    pub connection_start_time: Option<Instant>,
    pub password_visible: bool,
}

impl App {
    pub fn new() -> App {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        App {
            networks: Vec::new(),
            selected_index: 0,
            list_state,
            state: AppState::Scanning,
            password_input: String::new(),
            selected_network: None,
            status_message: "Scanning for networks...".to_string(),
            should_quit: false,
            connection_success: false,
            connection_error: None,
            is_disconnect_operation: false,
            adapter_info: None,
            network_count: 0,
            last_scan_time: None,
            connection_start_time: None,
            password_visible: false,
        }
    }

    pub fn next(&mut self) {
        if !self.networks.is_empty() {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i >= self.networks.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
            self.selected_index = i;
        }
    }

    pub fn previous(&mut self) {
        if !self.networks.is_empty() {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        self.networks.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
            self.selected_index = i;
        }
    }

    pub fn select_network(&mut self) {
        let network = self.networks.get(self.selected_index).cloned();

        match &network {
            Some(network) if network.connected => {
                self.state = AppState::Disconnecting;
                self.status_message =
                    format!("Disconnecting from {}...", network.ssid);
            }
            Some(network) if network.secured => {
                self.state = AppState::PasswordInput;
                self.password_input.clear();
            }
            Some(network) => {
                self.state = AppState::Connecting;
                self.connection_start_time = Some(Instant::now());
                self.status_message =
                    format!("Connecting to {}...", network.ssid);
            }
            None => {}
        }

        self.is_disconnect_operation = self.state == AppState::Disconnecting;

        if network.is_some() {
            self.selected_network = network;
        }
    }

    pub fn add_char_to_password(&mut self, c: char) {
        self.password_input.push(c);
    }

    pub fn remove_char_from_password(&mut self) {
        self.password_input.pop();
    }

    pub fn confirm_password(&mut self) {
        self.state = AppState::Connecting;
        self.connection_start_time = Some(Instant::now());
        if let Some(network) = &self.selected_network {
            self.status_message = format!("Connecting to {}...", network.ssid);
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
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

    pub fn update_selection_after_rescan(&mut self) {
        if let Some(selected_network) = &self.selected_network {
            if let Some(new_index) = self
                .networks
                .iter()
                .position(|n| n.ssid == selected_network.ssid)
            {
                self.selected_index = new_index;
                self.list_state.select(Some(new_index));
            } else {
                self.selected_index = 0;
                self.list_state.select(Some(0));
            }
        }
        self.selected_network = None;
    }
}
