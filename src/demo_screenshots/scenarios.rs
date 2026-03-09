use std::time::Instant;

use crate::{
    app_state::{App, AppState},
    wifi::{WifiNetwork, WifiSecurity},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoScreen {
    Scanning,
    NetworkList,
    Help,
    Details,
    Password,
    Connecting,
    Disconnecting,
    ResultSuccess,
    ResultError,
}

impl DemoScreen {
    pub fn file_name(self) -> &'static str {
        match self {
            Self::Scanning => "scanning.svg",
            Self::NetworkList => "network-list.svg",
            Self::Help => "help.svg",
            Self::Details => "details.svg",
            Self::Password => "password.svg",
            Self::Connecting => "connecting.svg",
            Self::Disconnecting => "disconnecting.svg",
            Self::ResultSuccess => "result-success.svg",
            Self::ResultError => "result-error.svg",
        }
    }
}

pub fn build_demo_screen(screen: DemoScreen, networks: &[WifiNetwork]) -> App {
    match screen {
        DemoScreen::Scanning => scanning_app(),
        DemoScreen::NetworkList => network_list_app(networks),
        DemoScreen::Help => help_app(networks),
        DemoScreen::Details => details_app(networks),
        DemoScreen::Password => password_app(networks),
        DemoScreen::Connecting => connecting_app(networks),
        DemoScreen::Disconnecting => disconnecting_app(networks),
        DemoScreen::ResultSuccess => result_success_app(networks),
        DemoScreen::ResultError => result_error_app(networks),
    }
}

pub fn demo_shot_apps(networks: &[WifiNetwork]) -> Vec<(&'static str, App)> {
    [
        DemoScreen::Scanning,
        DemoScreen::NetworkList,
        DemoScreen::Help,
        DemoScreen::Details,
        DemoScreen::Password,
        DemoScreen::Connecting,
        DemoScreen::Disconnecting,
        DemoScreen::ResultSuccess,
        DemoScreen::ResultError,
    ]
    .into_iter()
    .map(|screen| (screen.file_name(), build_demo_screen(screen, networks)))
    .collect()
}

fn base_app(networks: &[WifiNetwork]) -> App {
    let mut app = App::new();
    app.networks = networks.to_vec();
    app.network_count = app.networks.len();
    app.adapter_name = Some("demo-wlan0".to_string());
    app.selected_index = 0;
    app.status_message = if networks.is_empty() {
        "Scanning for WiFi networks...".to_string()
    } else {
        format!("Found {} network(s). Ready to connect!", networks.len())
    };
    app
}

fn scanning_app() -> App {
    App::new()
}

fn network_list_app(networks: &[WifiNetwork]) -> App {
    let mut app = base_app(networks);
    app.state = AppState::NetworkList;
    app
}

fn help_app(networks: &[WifiNetwork]) -> App {
    let mut app = base_app(networks);
    app.state = AppState::Help;
    app
}

fn details_app(networks: &[WifiNetwork]) -> App {
    let mut app = base_app(networks);
    app.state = AppState::NetworkDetails;
    app.selected_index = 1;
    app
}

fn password_app(networks: &[WifiNetwork]) -> App {
    let mut app = base_app(networks);
    let network = networks
        .iter()
        .find(|network| network.is_secured() && !network.connected)
        .cloned()
        .expect("demo secure network exists");
    app.state = AppState::PasswordInput;
    app.selected_network = Some(network);
    app.password_input = "hunter2".to_string();
    app.password_visible = false;
    app
}

fn connecting_app(networks: &[WifiNetwork]) -> App {
    let mut app = base_app(networks);
    let network = networks
        .iter()
        .find(|network| !network.connected)
        .cloned()
        .expect("demo network exists");
    app.state = AppState::Connecting;
    app.selected_network = Some(network.clone());
    app.status_message = format!("Connecting to {}...", network.ssid);
    app.connection_start_time = Some(Instant::now());
    app
}

fn disconnecting_app(networks: &[WifiNetwork]) -> App {
    let mut app = base_app(networks);
    let network = networks
        .iter()
        .find(|network| network.connected)
        .cloned()
        .expect("demo connected network exists");
    app.state = AppState::Disconnecting;
    app.selected_network = Some(network.clone());
    app.is_disconnect_operation = true;
    app.status_message = format!("Disconnecting from {}...", network.ssid);
    app.connection_start_time = Some(Instant::now());
    app
}

fn result_success_app(networks: &[WifiNetwork]) -> App {
    let mut app = base_app(networks);
    let network = networks
        .iter()
        .find(|network| network.connected)
        .cloned()
        .expect("demo connected network exists");
    app.state = AppState::ConnectionResult;
    app.selected_network = Some(network);
    app.connection_success = true;
    app.status_message = "Connected successfully!".to_string();
    app
}

fn result_error_app(networks: &[WifiNetwork]) -> App {
    let mut app = base_app(networks);
    let network = networks
        .iter()
        .find(|network| network.security == WifiSecurity::WpaSae)
        .cloned()
        .unwrap_or_else(|| networks[0].clone());
    app.state = AppState::ConnectionResult;
    app.selected_network = Some(network);
    app.connection_success = false;
    app.connection_error =
        Some("Failed to find WiFi device in NetworkManager".to_string());
    app.status_message = "Connection failed".to_string();
    app
}
