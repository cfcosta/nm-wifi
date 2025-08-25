use std::{
    collections::HashMap,
    error::Error,
    io,
    time::{Duration, Instant},
};

use crossterm::{
    event::{
        self,
        DisableMouseCapture,
        EnableMouseCapture,
        Event,
        KeyCode,
        KeyEventKind,
    },
    execute,
    terminal::{
        EnterAlternateScreen,
        LeaveAlternateScreen,
        disable_raw_mode,
        enable_raw_mode,
    },
};
use networkmanager::{
    NetworkManager,
    devices::{Device, Wireless},
};
use ratatui::{
    Frame,
    Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

// Catppuccin Mocha color palette
struct CatppuccinColors;

#[allow(dead_code)]
impl CatppuccinColors {
    const BASE: Color = Color::Rgb(30, 30, 46); // #1e1e2e
    const MANTLE: Color = Color::Rgb(24, 24, 37); // #181825
    const SURFACE0: Color = Color::Rgb(49, 50, 68); // #313244
    const SURFACE1: Color = Color::Rgb(69, 71, 90); // #45475a
    const SURFACE2: Color = Color::Rgb(88, 91, 112); // #585b70
    const TEXT: Color = Color::Rgb(205, 214, 244); // #cdd6f4
    const SUBTEXT1: Color = Color::Rgb(186, 194, 222); // #bac2de
    const SUBTEXT0: Color = Color::Rgb(166, 173, 200); // #a6adc8
    const OVERLAY2: Color = Color::Rgb(147, 153, 178); // #9399b2
    const OVERLAY1: Color = Color::Rgb(127, 132, 156); // #7f849c
    const OVERLAY0: Color = Color::Rgb(108, 112, 134); // #6c7086
    const LAVENDER: Color = Color::Rgb(180, 190, 254); // #b4befe
    const BLUE: Color = Color::Rgb(137, 180, 250); // #89b4fa
    const SAPPHIRE: Color = Color::Rgb(116, 199, 236); // #74c7ec
    const SKY: Color = Color::Rgb(137, 220, 235); // #89dceb
    const TEAL: Color = Color::Rgb(148, 226, 213); // #94e2d5
    const GREEN: Color = Color::Rgb(166, 227, 161); // #a6e3a1
    const YELLOW: Color = Color::Rgb(249, 226, 175); // #f9e2af
    const PEACH: Color = Color::Rgb(250, 179, 135); // #fab387
    const MAROON: Color = Color::Rgb(235, 160, 172); // #eba0ac
    const RED: Color = Color::Rgb(243, 139, 168); // #f38ba8
    const MAUVE: Color = Color::Rgb(203, 166, 247); // #cba6f7
    const PINK: Color = Color::Rgb(245, 194, 231); // #f5c2e7
    const FLAMINGO: Color = Color::Rgb(242, 205, 205); // #f2cdcd
    const ROSEWATER: Color = Color::Rgb(245, 224, 220); // #f5e0dc
}
use std::process::Command;

use tokio::time::sleep;

#[derive(Debug, Clone)]
struct WifiNetwork {
    ssid: String,
    signal_strength: u8,
    secured: bool,
    frequency: u32,
    connected: bool,
}

#[derive(PartialEq)]
enum AppState {
    Scanning,
    NetworkList,
    PasswordInput,
    Connecting,
    Disconnecting,
    ConnectionResult,
    Help,
    NetworkDetails,
}

struct App {
    networks: Vec<WifiNetwork>,
    selected_index: usize,
    list_state: ListState,
    state: AppState,
    password_input: String,
    selected_network: Option<WifiNetwork>,
    status_message: String,
    should_quit: bool,
    connection_success: bool,
    connection_error: Option<String>,
    is_disconnect_operation: bool,
    adapter_info: Option<String>,
    network_count: usize,
    last_scan_time: Option<std::time::Instant>,
    connection_start_time: Option<std::time::Instant>,
    password_visible: bool,
}

impl App {
    fn new() -> App {
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

    fn next(&mut self) {
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

    fn previous(&mut self) {
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

    fn select_network(&mut self) {
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

    fn add_char_to_password(&mut self, c: char) {
        self.password_input.push(c);
    }

    fn remove_char_from_password(&mut self) {
        self.password_input.pop();
    }

    fn confirm_password(&mut self) {
        self.state = AppState::Connecting;
        self.connection_start_time = Some(Instant::now());
        if let Some(network) = &self.selected_network {
            self.status_message = format!("Connecting to {}...", network.ssid);
        }
    }

    fn quit(&mut self) {
        self.should_quit = true;
    }

    fn back_to_network_list(&mut self) {
        self.state = AppState::NetworkList;
        self.connection_success = false;
        self.connection_error = None;
        self.password_input.clear();
        self.password_visible = false;
        self.is_disconnect_operation = false;
        self.connection_start_time = None;
        // Keep selected_network to preserve selection after rescan
    }

    fn update_selection_after_rescan(&mut self) {
        if let Some(selected_network) = &self.selected_network {
            // Find the network by SSID in the new list
            if let Some(new_index) = self
                .networks
                .iter()
                .position(|n| n.ssid == selected_network.ssid)
            {
                self.selected_index = new_index;
                self.list_state.select(Some(new_index));
            } else {
                // Network not found, select first network
                self.selected_index = 0;
                self.list_state.select(Some(0));
            }
        }
        // Clear selected_network after updating selection
        self.selected_network = None;
    }
}

async fn get_connected_ssid() -> Option<String> {
    let output = Command::new("nmcli")
        .args(["-t", "-f", "ACTIVE,SSID", "dev", "wifi"])
        .output()
        .ok()?;

    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if let Some(stripped) = line.strip_prefix("yes:") {
                return Some(stripped.to_string());
            }
        }
    }
    None
}

async fn get_wifi_adapter_info() -> Option<String> {
    let output = Command::new("nmcli")
        .args(["-t", "-f", "DEVICE,TYPE,STATE", "dev"])
        .output()
        .ok()?;

    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 && parts[1] == "wifi" && parts[2] == "connected"
            {
                return Some(parts[0].to_string());
            }
        }
        // If no connected adapter, find any wifi adapter
        for line in output_str.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 && parts[1] == "wifi" {
                return Some(parts[0].to_string());
            }
        }
    }
    None
}

async fn scan_wifi_networks() -> Result<Vec<WifiNetwork>, Box<dyn Error>> {
    let dbus = dbus::blocking::Connection::new_system()
        .map_err(|_| "Failed to connect to D-Bus".to_string())?;
    let nm = NetworkManager::new(&dbus);

    // Get currently connected SSID
    let connected_ssid = get_connected_ssid().await;

    let devices = nm
        .get_devices()
        .map_err(|_| "Failed to get devices".to_string())?;

    for device in devices {
        if let Device::WiFi(wifi_device) = device {
            wifi_device
                .request_scan(HashMap::new())
                .map_err(|_| "Failed to request scan".to_string())?;

            // Brief wait for scan to start
            sleep(Duration::from_millis(200)).await;

            let access_points = wifi_device
                .get_all_access_points()
                .map_err(|_| "Failed to get access points".to_string())?;

            let mut networks = Vec::new();

            for ap in access_points {
                let ssid =
                    ap.ssid().map_err(|_| "Failed to get SSID".to_string())?;
                if !ssid.is_empty() {
                    let flags = ap
                        .flags()
                        .map_err(|_| "Failed to get flags".to_string())?;
                    let wpa_flags = ap
                        .wpa_flags()
                        .map_err(|_| "Failed to get WPA flags".to_string())?;
                    let rsn_flags = ap
                        .rsn_flags()
                        .map_err(|_| "Failed to get RSN flags".to_string())?;

                    let secured =
                        rsn_flags != 0 || wpa_flags != 0 || (flags & 0x1) != 0;

                    let signal_strength = ap.strength().map_err(|_| {
                        "Failed to get signal strength".to_string()
                    })?;

                    let frequency = ap
                        .frequency()
                        .map_err(|_| "Failed to get frequency".to_string())?;

                    let connected = connected_ssid.as_ref() == Some(&ssid);

                    let network = WifiNetwork {
                        ssid,
                        signal_strength,
                        secured,
                        frequency,
                        connected,
                    };
                    networks.push(network);
                }
            }

            // Deduplicate networks by SSID, keeping the one with highest frequency
            let mut unique_networks: HashMap<String, WifiNetwork> =
                HashMap::new();
            for network in networks {
                match unique_networks.get(&network.ssid) {
                    Some(existing) => {
                        if network.frequency > existing.frequency {
                            unique_networks
                                .insert(network.ssid.clone(), network);
                        }
                    }
                    None => {
                        unique_networks.insert(network.ssid.clone(), network);
                    }
                }
            }

            let mut deduplicated_networks: Vec<WifiNetwork> =
                unique_networks.into_values().collect();

            // Sort by connection status first, then by signal strength
            deduplicated_networks.sort_by(|a, b| {
                match (a.connected, b.connected) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => b.signal_strength.cmp(&a.signal_strength),
                }
            });

            return Ok(deduplicated_networks);
        }
    }

    Ok(Vec::new())
}

async fn connect_to_network(
    network: &WifiNetwork,
    password: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    use std::process::Command;

    if network.secured && password.is_none() {
        return Err("Password required for secured network".into());
    }

    // Use nmcli command line tool for connection
    let mut cmd = Command::new("nmcli");

    if network.secured {
        // For secured networks, use the connection add approach
        cmd.args([
            "connection",
            "add",
            "type",
            "wifi",
            "con-name",
            &network.ssid,
            "ssid",
            &network.ssid,
            "wifi-sec.key-mgmt",
            "wpa-psk",
            "wifi-sec.psk",
            password.unwrap(),
        ]);

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to execute nmcli add: {}", e))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            // If connection already exists, try to modify it
            if error_msg.contains("already exists") {
                let mut modify_cmd = Command::new("nmcli");
                modify_cmd.args([
                    "connection",
                    "modify",
                    &network.ssid,
                    "wifi-sec.psk",
                    password.unwrap(),
                ]);

                let modify_output = modify_cmd.output().map_err(|e| {
                    format!("Failed to execute nmcli modify: {}", e)
                })?;

                if !modify_output.status.success() {
                    let modify_error =
                        String::from_utf8_lossy(&modify_output.stderr);
                    return Err(format!(
                        "nmcli modify failed: {}",
                        modify_error
                    )
                    .into());
                }
            } else {
                return Err(format!("nmcli add failed: {}", error_msg).into());
            }
        }

        // Now activate the connection
        let mut activate_cmd = Command::new("nmcli");
        activate_cmd.args(["connection", "up", &network.ssid]);

        let activate_output = activate_cmd
            .output()
            .map_err(|e| format!("Failed to execute nmcli up: {}", e))?;

        if activate_output.status.success() {
            Ok(())
        } else {
            let error_msg = String::from_utf8_lossy(&activate_output.stderr);
            Err(format!("nmcli activation failed: {}", error_msg).into())
        }
    } else {
        // For open networks, use the simple connect command
        cmd.args(["device", "wifi", "connect", &network.ssid]);

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to execute nmcli: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            Err(format!("nmcli failed: {}", error_msg).into())
        }
    }
}

async fn disconnect_from_network(
    network: &WifiNetwork,
) -> Result<(), Box<dyn Error>> {
    use std::process::Command;

    let output = Command::new("nmcli")
        .args(["connection", "down", &network.ssid])
        .output()
        .map_err(|e| format!("Failed to execute nmcli: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        Err(format!("nmcli disconnect failed: {}", error_msg).into())
    }
}

fn create_signal_graph(strength: u8) -> String {
    let bars = (strength as f32 / 100.0 * 20.0) as usize;
    let filled = "█".repeat(bars);
    let empty = "░".repeat(20 - bars);
    format!("{}{}", filled, empty)
}

fn get_frequency_band(frequency: u32) -> &'static str {
    if frequency >= 5000 { "5G" } else { "2.4G" }
}

fn format_signal_strength(strength: u8) -> String {
    format!("{}%", strength)
}

fn create_network_list_item<'a>(network: &WifiNetwork) -> ListItem<'a> {
    let signal_graph = create_signal_graph(network.signal_strength);
    let signal_percent = format_signal_strength(network.signal_strength);
    let frequency_band = get_frequency_band(network.frequency);
    let security_icon = if network.secured { "🔒" } else { "  " };
    let connection_icon = if network.connected { "🔗" } else { "  " };

    let signal_color = match network.signal_strength {
        80..=100 => CatppuccinColors::GREEN,
        60..=79 => CatppuccinColors::YELLOW,
        40..=59 => CatppuccinColors::PEACH,
        _ => CatppuccinColors::RED,
    };
    let ssid_color = if network.connected {
        CatppuccinColors::GREEN
    } else {
        CatppuccinColors::TEXT
    };

    ListItem::new(Line::from(vec![
        Span::styled(
            connection_icon.to_string(),
            Style::default().fg(CatppuccinColors::GREEN),
        ),
        Span::styled(
            format!("{} ", security_icon),
            Style::default().fg(CatppuccinColors::MAUVE),
        ),
        Span::styled(
            format!("{:<24}", network.ssid),
            Style::default().fg(ssid_color),
        ),
        Span::styled(
            format!("{:>4} ", frequency_band),
            Style::default().fg(CatppuccinColors::SAPPHIRE),
        ),
        Span::styled(
            format!("{:>4} ", signal_percent),
            Style::default().fg(signal_color),
        ),
        Span::styled(signal_graph, Style::default().fg(signal_color)),
    ]))
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(30),
            Constraint::Min(0),
            Constraint::Length(25),
        ])
        .split(area);

    // Left side - App title and version
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "nm-wifi",
            Style::default()
                .fg(CatppuccinColors::MAUVE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " v0.1.0",
            Style::default().fg(CatppuccinColors::SUBTEXT1),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL))
    .style(Style::default().bg(CatppuccinColors::BASE));

    // Center - Network count and scan info
    let scan_info = if let Some(scan_time) = app.last_scan_time {
        let elapsed = scan_time.elapsed().as_secs();
        format!(
            "Networks: {} | Last scan: {}s ago",
            app.network_count, elapsed
        )
    } else {
        format!("Networks: {}", app.network_count)
    };

    let info = Paragraph::new(scan_info)
        .block(Block::default().borders(Borders::ALL))
        .style(
            Style::default()
                .fg(CatppuccinColors::TEXT)
                .bg(CatppuccinColors::BASE),
        )
        .alignment(Alignment::Center);

    // Right side - Adapter info
    let adapter_text = app.adapter_info.as_deref().unwrap_or("WiFi Adapter");
    let adapter = Paragraph::new(adapter_text)
        .block(Block::default().borders(Borders::ALL))
        .style(
            Style::default()
                .fg(CatppuccinColors::BLUE)
                .bg(CatppuccinColors::BASE),
        )
        .alignment(Alignment::Center);

    f.render_widget(title, header_chunks[0]);
    f.render_widget(info, header_chunks[1]);
    f.render_widget(adapter, header_chunks[2]);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(40)])
        .split(area);

    // Main status message
    let status = Paragraph::new(app.status_message.as_str())
        .block(Block::default().borders(Borders::ALL))
        .style(
            Style::default()
                .fg(CatppuccinColors::SUBTEXT1)
                .bg(CatppuccinColors::BASE),
        )
        .alignment(Alignment::Left);

    // Keybindings hint
    let keybindings = match app.state {
        AppState::NetworkList => {
            "h:Help | i:Info | r:Rescan | c/Enter:Connect | d:Disconnect | q:Quit"
        }
        AppState::Help => "h/q/Esc:Back",
        AppState::NetworkDetails => "q/i/Esc:Back",
        AppState::PasswordInput => "Enter:Connect | Esc:Cancel",
        _ => "Esc:Cancel | q:Quit",
    };

    let hints = Paragraph::new(keybindings)
        .block(Block::default().borders(Borders::ALL))
        .style(
            Style::default()
                .fg(CatppuccinColors::OVERLAY1)
                .bg(CatppuccinColors::BASE),
        )
        .alignment(Alignment::Center);

    f.render_widget(status, status_chunks[0]);
    f.render_widget(hints, status_chunks[1]);
}

fn render_help_screen(f: &mut Frame, _app: &App, area: Rect) {
    let help_text = vec![
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default()
                .fg(CatppuccinColors::MAUVE)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("↑/k", Style::default().fg(CatppuccinColors::GREEN)),
            Span::styled(
                "        Move up",
                Style::default().fg(CatppuccinColors::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("↓/j", Style::default().fg(CatppuccinColors::GREEN)),
            Span::styled(
                "        Move down",
                Style::default().fg(CatppuccinColors::TEXT),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions",
            Style::default()
                .fg(CatppuccinColors::MAUVE)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Enter/c",
                Style::default().fg(CatppuccinColors::GREEN),
            ),
            Span::styled(
                "     Connect to network",
                Style::default().fg(CatppuccinColors::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("d", Style::default().fg(CatppuccinColors::GREEN)),
            Span::styled(
                "           Disconnect from network",
                Style::default().fg(CatppuccinColors::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("r", Style::default().fg(CatppuccinColors::GREEN)),
            Span::styled(
                "           Rescan networks",
                Style::default().fg(CatppuccinColors::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("i", Style::default().fg(CatppuccinColors::GREEN)),
            Span::styled(
                "           Show network details",
                Style::default().fg(CatppuccinColors::TEXT),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Other",
            Style::default()
                .fg(CatppuccinColors::MAUVE)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("h", Style::default().fg(CatppuccinColors::GREEN)),
            Span::styled(
                "         Show this help",
                Style::default().fg(CatppuccinColors::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("q/Esc", Style::default().fg(CatppuccinColors::GREEN)),
            Span::styled(
                "      Quit application",
                Style::default().fg(CatppuccinColors::TEXT),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Symbols",
            Style::default()
                .fg(CatppuccinColors::MAUVE)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("🔗", Style::default().fg(CatppuccinColors::GREEN)),
            Span::styled(
                "         Connected",
                Style::default().fg(CatppuccinColors::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("🔒", Style::default().fg(CatppuccinColors::MAUVE)),
            Span::styled(
                "         Secured network",
                Style::default().fg(CatppuccinColors::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "2.4G/5G",
                Style::default().fg(CatppuccinColors::SAPPHIRE),
            ),
            Span::styled(
                "     Frequency band",
                Style::default().fg(CatppuccinColors::TEXT),
            ),
        ]),
    ];

    let help_paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Help - nm-wifi")
                .title_style(
                    Style::default()
                        .fg(CatppuccinColors::BLUE)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .style(Style::default().bg(CatppuccinColors::BASE))
        .alignment(Alignment::Left);

    f.render_widget(help_paragraph, area);
}

fn render_network_details(f: &mut Frame, app: &App) {
    if let Some(network) = app.networks.get(app.selected_index) {
        let popup_area = centered_rect(60, 70, f.area());
        f.render_widget(Clear, popup_area);

        let security_type = if network.secured {
            "Secured (WPA/WPA2)"
        } else {
            "Open"
        };

        let signal_description = match network.signal_strength {
            80..=100 => "Excellent",
            60..=79 => "Good",
            40..=59 => "Fair",
            20..=39 => "Weak",
            _ => "Very Weak",
        };

        let signal_text =
            format!("{}% ({})", network.signal_strength, signal_description);
        let frequency_text = format!(
            "{} MHz ({})",
            network.frequency,
            get_frequency_band(network.frequency)
        );

        let details_text = vec![
            Line::from(vec![
                Span::styled(
                    "SSID: ",
                    Style::default()
                        .fg(CatppuccinColors::MAUVE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    &network.ssid,
                    Style::default().fg(CatppuccinColors::TEXT),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Status: ",
                    Style::default()
                        .fg(CatppuccinColors::MAUVE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    if network.connected {
                        "Connected"
                    } else {
                        "Available"
                    },
                    Style::default().fg(if network.connected {
                        CatppuccinColors::GREEN
                    } else {
                        CatppuccinColors::TEXT
                    }),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Security: ",
                    Style::default()
                        .fg(CatppuccinColors::MAUVE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    security_type,
                    Style::default().fg(CatppuccinColors::TEXT),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Signal Strength: ",
                    Style::default()
                        .fg(CatppuccinColors::MAUVE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    &signal_text,
                    Style::default().fg(match network.signal_strength {
                        80..=100 => CatppuccinColors::GREEN,
                        60..=79 => CatppuccinColors::YELLOW,
                        40..=59 => CatppuccinColors::PEACH,
                        _ => CatppuccinColors::RED,
                    }),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Frequency: ",
                    Style::default()
                        .fg(CatppuccinColors::MAUVE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    &frequency_text,
                    Style::default().fg(CatppuccinColors::SAPPHIRE),
                ),
            ]),
            Line::from(""),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Press ",
                    Style::default().fg(CatppuccinColors::SUBTEXT1),
                ),
                Span::styled(
                    "i",
                    Style::default()
                        .fg(CatppuccinColors::GREEN)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " or ",
                    Style::default().fg(CatppuccinColors::SUBTEXT1),
                ),
                Span::styled(
                    "Esc",
                    Style::default()
                        .fg(CatppuccinColors::GREEN)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " to close",
                    Style::default().fg(CatppuccinColors::SUBTEXT1),
                ),
            ]),
        ];

        let details_paragraph = Paragraph::new(details_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Network Details")
                    .title_style(
                        Style::default()
                            .fg(CatppuccinColors::BLUE)
                            .add_modifier(Modifier::BOLD),
                    ),
            )
            .style(Style::default().bg(CatppuccinColors::BASE))
            .alignment(Alignment::Left);

        f.render_widget(details_paragraph, popup_area);
    }
}

fn create_progress_bar(progress: f32, width: usize) -> String {
    let filled = ((progress * width as f32) as usize).min(width);
    let empty = width - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

fn get_connection_animation_frame(elapsed_ms: u128) -> char {
    let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    frames[(elapsed_ms / 100) as usize % frames.len()]
}

fn render_enhanced_password_modal(f: &mut Frame, app: &App) {
    if let Some(network) = &app.selected_network {
        let popup_area = centered_rect(70, 40, f.area());
        f.render_widget(Clear, popup_area);

        // Create border with shadow effect
        let shadow_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width,
            height: popup_area.height,
        };
        f.render_widget(
            Block::default()
                .style(Style::default().bg(CatppuccinColors::SURFACE0)),
            shadow_area,
        );

        let security_type = if network.secured {
            "🔒 WPA/WPA2"
        } else {
            "🔓 Open"
        };
        let signal_strength = format!("📶 {}%", network.signal_strength);
        let frequency_band =
            format!("📡 {}", get_frequency_band(network.frequency));

        let password_display = if app.password_visible {
            app.password_input.clone()
        } else {
            "•".repeat(app.password_input.len())
        };
        let password_field = format!("{:<38}", password_display);

        let password_text = vec![
            Line::from(vec![
                Span::styled(
                    "🔗 ",
                    Style::default().fg(CatppuccinColors::BLUE),
                ),
                Span::styled(
                    "Connect to Network",
                    Style::default()
                        .fg(CatppuccinColors::TEXT)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Network: ",
                    Style::default()
                        .fg(CatppuccinColors::MAUVE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    &network.ssid,
                    Style::default().fg(CatppuccinColors::TEXT),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Security: ",
                    Style::default()
                        .fg(CatppuccinColors::MAUVE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    security_type,
                    Style::default().fg(CatppuccinColors::YELLOW),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Signal: ",
                    Style::default()
                        .fg(CatppuccinColors::MAUVE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    &signal_strength,
                    Style::default().fg(match network.signal_strength {
                        80..=100 => CatppuccinColors::GREEN,
                        60..=79 => CatppuccinColors::YELLOW,
                        40..=59 => CatppuccinColors::PEACH,
                        _ => CatppuccinColors::RED,
                    }),
                ),
                Span::styled("  ", Style::default()),
                Span::styled(
                    &frequency_band,
                    Style::default().fg(CatppuccinColors::SAPPHIRE),
                ),
            ]),
            Line::from(vec![Span::styled(
                "Password: ",
                Style::default()
                    .fg(CatppuccinColors::MAUVE)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "┌",
                    Style::default().fg(CatppuccinColors::SURFACE2),
                ),
                Span::styled(
                    "─".repeat(40),
                    Style::default().fg(CatppuccinColors::SURFACE2),
                ),
                Span::styled(
                    "┐",
                    Style::default().fg(CatppuccinColors::SURFACE2),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "│ ",
                    Style::default().fg(CatppuccinColors::SURFACE2),
                ),
                Span::styled(
                    &password_field,
                    Style::default()
                        .fg(CatppuccinColors::TEXT)
                        .bg(CatppuccinColors::SURFACE0),
                ),
                Span::styled(
                    " │",
                    Style::default().fg(CatppuccinColors::SURFACE2),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "└",
                    Style::default().fg(CatppuccinColors::SURFACE2),
                ),
                Span::styled(
                    "─".repeat(40),
                    Style::default().fg(CatppuccinColors::SURFACE2),
                ),
                Span::styled(
                    "┘",
                    Style::default().fg(CatppuccinColors::SURFACE2),
                ),
            ]),
            Line::from(""),
            Line::from(""),
            Line::from(vec![Span::styled(
                "💡 Tips:",
                Style::default()
                    .fg(CatppuccinColors::YELLOW)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                "  • Type your WiFi password",
                Style::default().fg(CatppuccinColors::SUBTEXT1),
            )]),
            Line::from(vec![
                Span::styled(
                    "  • Press ",
                    Style::default().fg(CatppuccinColors::SUBTEXT1),
                ),
                Span::styled(
                    "Tab",
                    Style::default()
                        .fg(CatppuccinColors::GREEN)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " to toggle password visibility",
                    Style::default().fg(CatppuccinColors::SUBTEXT1),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Press ",
                    Style::default().fg(CatppuccinColors::SUBTEXT1),
                ),
                Span::styled(
                    "Enter",
                    Style::default()
                        .fg(CatppuccinColors::GREEN)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " to connect or ",
                    Style::default().fg(CatppuccinColors::SUBTEXT1),
                ),
                Span::styled(
                    "Esc",
                    Style::default()
                        .fg(CatppuccinColors::RED)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " to cancel",
                    Style::default().fg(CatppuccinColors::SUBTEXT1),
                ),
            ]),
        ];

        let password_modal = Paragraph::new(password_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("🔑 Enter Network Password")
                    .title_style(
                        Style::default()
                            .fg(CatppuccinColors::BLUE)
                            .add_modifier(Modifier::BOLD),
                    )
                    .border_style(Style::default().fg(CatppuccinColors::BLUE)),
            )
            .style(Style::default().bg(CatppuccinColors::BASE))
            .alignment(Alignment::Left);

        f.render_widget(password_modal, popup_area);
    }
}

fn render_enhanced_connecting_modal(f: &mut Frame, app: &App) {
    if let Some(network) = &app.selected_network {
        let popup_area = centered_rect(70, 35, f.area());
        f.render_widget(Clear, popup_area);

        // Create border with shadow effect
        let shadow_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width,
            height: popup_area.height,
        };
        f.render_widget(
            Block::default()
                .style(Style::default().bg(CatppuccinColors::SURFACE0)),
            shadow_area,
        );

        let elapsed = app
            .connection_start_time
            .map(|start| start.elapsed().as_millis())
            .unwrap_or(0);

        let spinner = get_connection_animation_frame(elapsed);
        let progress = (elapsed as f32 / 5000.0).min(0.9); // Max 90% during connection
        let progress_bar = create_progress_bar(progress, 30);

        let elapsed_secs = elapsed / 1000;
        let status_text = format!("{} Establishing connection...", spinner);
        let progress_percent = format!(" {}%", (progress * 100.0) as u8);
        let elapsed_text = format!("Elapsed time: {}s", elapsed_secs);
        let connecting_step = format!("  {} ", spinner);

        let connecting_text = vec![
            Line::from(vec![
                Span::styled(
                    "🔗 ",
                    Style::default().fg(CatppuccinColors::BLUE),
                ),
                Span::styled(
                    "Connecting to Network",
                    Style::default()
                        .fg(CatppuccinColors::TEXT)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Network: ",
                    Style::default()
                        .fg(CatppuccinColors::MAUVE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    &network.ssid,
                    Style::default().fg(CatppuccinColors::TEXT),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Status: ",
                    Style::default()
                        .fg(CatppuccinColors::MAUVE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    &status_text,
                    Style::default().fg(CatppuccinColors::YELLOW),
                ),
            ]),
            Line::from(vec![Span::styled(
                "Progress:",
                Style::default()
                    .fg(CatppuccinColors::MAUVE)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    &progress_bar,
                    Style::default().fg(CatppuccinColors::BLUE),
                ),
                Span::styled(
                    &progress_percent,
                    Style::default().fg(CatppuccinColors::TEXT),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                &elapsed_text,
                Style::default().fg(CatppuccinColors::SUBTEXT1),
            )]),
            Line::from(""),
            Line::from(""),
            Line::from(vec![Span::styled(
                "📋 Connection Steps:",
                Style::default()
                    .fg(CatppuccinColors::YELLOW)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled(
                    "  ✓ ",
                    Style::default().fg(CatppuccinColors::GREEN),
                ),
                Span::styled(
                    "Network found",
                    Style::default().fg(CatppuccinColors::SUBTEXT1),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  ✓ ",
                    Style::default().fg(CatppuccinColors::GREEN),
                ),
                Span::styled(
                    "Credentials verified",
                    Style::default().fg(CatppuccinColors::SUBTEXT1),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    &connecting_step,
                    Style::default().fg(CatppuccinColors::YELLOW),
                ),
                Span::styled(
                    "Establishing connection",
                    Style::default().fg(CatppuccinColors::YELLOW),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  ○ ",
                    Style::default().fg(CatppuccinColors::SURFACE2),
                ),
                Span::styled(
                    "Obtaining IP address",
                    Style::default().fg(CatppuccinColors::SURFACE2),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Press ",
                    Style::default().fg(CatppuccinColors::SUBTEXT1),
                ),
                Span::styled(
                    "Esc",
                    Style::default()
                        .fg(CatppuccinColors::RED)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " to cancel",
                    Style::default().fg(CatppuccinColors::SUBTEXT1),
                ),
            ]),
        ];

        let connecting_modal = Paragraph::new(connecting_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("⚡ Connecting...")
                    .title_style(
                        Style::default()
                            .fg(CatppuccinColors::YELLOW)
                            .add_modifier(Modifier::BOLD),
                    )
                    .border_style(
                        Style::default().fg(CatppuccinColors::YELLOW),
                    ),
            )
            .style(Style::default().bg(CatppuccinColors::BASE))
            .alignment(Alignment::Left);

        f.render_widget(connecting_modal, popup_area);
    }
}

fn render_enhanced_disconnecting_modal(f: &mut Frame, app: &App) {
    if let Some(network) = &app.selected_network {
        let popup_area = centered_rect(70, 25, f.area());
        f.render_widget(Clear, popup_area);

        // Create border with shadow effect
        let shadow_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width,
            height: popup_area.height,
        };
        f.render_widget(
            Block::default()
                .style(Style::default().bg(CatppuccinColors::SURFACE0)),
            shadow_area,
        );

        let elapsed = app
            .connection_start_time
            .map(|start| start.elapsed().as_millis())
            .unwrap_or(0);

        let spinner = get_connection_animation_frame(elapsed);
        let status_text = format!("{} Terminating connection...", spinner);
        let releasing_step = format!("  {} ", spinner);
        let terminating_step = format!("  {} ", spinner);

        let disconnecting_text = vec![
            Line::from(vec![
                Span::styled(
                    "🔌 ",
                    Style::default().fg(CatppuccinColors::PEACH),
                ),
                Span::styled(
                    "Disconnecting from Network",
                    Style::default()
                        .fg(CatppuccinColors::TEXT)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Network: ",
                    Style::default()
                        .fg(CatppuccinColors::MAUVE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    &network.ssid,
                    Style::default().fg(CatppuccinColors::TEXT),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Status: ",
                    Style::default()
                        .fg(CatppuccinColors::MAUVE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    &status_text,
                    Style::default().fg(CatppuccinColors::PEACH),
                ),
            ]),
            Line::from(vec![Span::styled(
                "📋 Disconnection Steps:",
                Style::default()
                    .fg(CatppuccinColors::PEACH)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled(
                    &releasing_step,
                    Style::default().fg(CatppuccinColors::PEACH),
                ),
                Span::styled(
                    "Releasing IP address",
                    Style::default().fg(CatppuccinColors::PEACH),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    &terminating_step,
                    Style::default().fg(CatppuccinColors::PEACH),
                ),
                Span::styled(
                    "Terminating connection",
                    Style::default().fg(CatppuccinColors::PEACH),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "This will only take a moment...",
                Style::default().fg(CatppuccinColors::SUBTEXT1),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Press ",
                    Style::default().fg(CatppuccinColors::SUBTEXT1),
                ),
                Span::styled(
                    "Esc",
                    Style::default()
                        .fg(CatppuccinColors::RED)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " to cancel",
                    Style::default().fg(CatppuccinColors::SUBTEXT1),
                ),
            ]),
        ];

        let disconnecting_modal = Paragraph::new(disconnecting_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("🔻 Disconnecting...")
                    .title_style(
                        Style::default()
                            .fg(CatppuccinColors::PEACH)
                            .add_modifier(Modifier::BOLD),
                    )
                    .border_style(Style::default().fg(CatppuccinColors::PEACH)),
            )
            .style(Style::default().bg(CatppuccinColors::BASE))
            .alignment(Alignment::Left);

        f.render_widget(disconnecting_modal, popup_area);
    }
}

fn render_enhanced_result_modal(f: &mut Frame, app: &App) {
    let popup_area = centered_rect(70, 45, f.area());
    f.render_widget(Clear, popup_area);

    // Create border with shadow effect
    let shadow_area = Rect {
        x: popup_area.x + 1,
        y: popup_area.y + 1,
        width: popup_area.width,
        height: popup_area.height,
    };
    f.render_widget(
        Block::default().style(Style::default().bg(CatppuccinColors::SURFACE0)),
        shadow_area,
    );

    let network_name = app
        .selected_network
        .as_ref()
        .map(|n| n.ssid.as_str())
        .unwrap_or("Unknown");

    let (icon, title, color, main_message) = if app.connection_success {
        if app.is_disconnect_operation {
            (
                "✅",
                "Disconnection Successful",
                CatppuccinColors::GREEN,
                format!("Successfully disconnected from {}!", network_name),
            )
        } else {
            (
                "🎉",
                "Connection Successful",
                CatppuccinColors::GREEN,
                format!("Successfully connected to {}!", network_name),
            )
        }
    } else if app.is_disconnect_operation {
        (
            "❌",
            "Disconnection Failed",
            CatppuccinColors::RED,
            "Failed to disconnect from network".to_string(),
        )
    } else {
        (
            "❌",
            "Connection Failed",
            CatppuccinColors::RED,
            "Failed to connect to network".to_string(),
        )
    };

    let icon_text = format!("{} ", icon);
    let title_text = format!("{} {}", icon, title);

    let mut result_text = vec![
        Line::from(vec![
            Span::styled(&icon_text, Style::default().fg(color)),
            Span::styled(
                title,
                Style::default()
                    .fg(CatppuccinColors::TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            &main_message,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )]),
    ];

    // Add network details if available
    if let Some(network) = &app.selected_network {
        result_text.extend(vec![
            Line::from(vec![Span::styled(
                "📊 Network Details:",
                Style::default()
                    .fg(CatppuccinColors::MAUVE)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "  Network: ",
                    Style::default().fg(CatppuccinColors::SUBTEXT1),
                ),
                Span::styled(
                    &network.ssid,
                    Style::default().fg(CatppuccinColors::TEXT),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  Security: ",
                    Style::default().fg(CatppuccinColors::SUBTEXT1),
                ),
                Span::styled(
                    if network.secured {
                        "🔒 Secured"
                    } else {
                        "🔓 Open"
                    },
                    Style::default().fg(CatppuccinColors::TEXT),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  Signal: ",
                    Style::default().fg(CatppuccinColors::SUBTEXT1),
                ),
                Span::styled(
                    format!(
                        "{}% ({})",
                        network.signal_strength,
                        get_frequency_band(network.frequency)
                    ),
                    Style::default().fg(CatppuccinColors::TEXT),
                ),
            ]),
            Line::from(""),
        ]);
    }

    result_text.extend(vec![
        Line::from(vec![Span::styled(
            "ℹ️  Information:",
            Style::default()
                .fg(CatppuccinColors::BLUE)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ]);

    // Add info based on success/failure
    if app.connection_success {
        if app.is_disconnect_operation {
            result_text.push(Line::from(vec![Span::styled(
                "  🔌 Connection terminated",
                Style::default().fg(CatppuccinColors::SUBTEXT1),
            )]));
            result_text.push(Line::from(vec![Span::styled(
                "  📡 Network interface released",
                Style::default().fg(CatppuccinColors::SUBTEXT1),
            )]));
            result_text.push(Line::from(vec![Span::styled(
                "  🌐 You are now offline from this network",
                Style::default().fg(CatppuccinColors::SUBTEXT1),
            )]));
        } else {
            result_text.push(Line::from(vec![Span::styled(
                "  🔗 Connection established",
                Style::default().fg(CatppuccinColors::SUBTEXT1),
            )]));
            result_text.push(Line::from(vec![Span::styled(
                "  📡 Signal strength is good",
                Style::default().fg(CatppuccinColors::SUBTEXT1),
            )]));
            result_text.push(Line::from(vec![Span::styled(
                "  🌐 Internet access available",
                Style::default().fg(CatppuccinColors::SUBTEXT1),
            )]));
        }
    } else {
        let error_msg =
            app.connection_error.as_deref().unwrap_or("Unknown error");
        if app.is_disconnect_operation {
            result_text.push(Line::from(vec![Span::styled(
                "  🔧 Try disconnecting manually",
                Style::default().fg(CatppuccinColors::SUBTEXT1),
            )]));
            result_text.push(Line::from(vec![Span::styled(
                "  📡 Check network manager status",
                Style::default().fg(CatppuccinColors::SUBTEXT1),
            )]));
        } else {
            result_text.push(Line::from(vec![Span::styled(
                "  🔐 Check your password",
                Style::default().fg(CatppuccinColors::SUBTEXT1),
            )]));
            result_text.push(Line::from(vec![Span::styled(
                "  📡 Verify network availability",
                Style::default().fg(CatppuccinColors::SUBTEXT1),
            )]));
        }
        result_text.push(Line::from(vec![Span::styled(
            format!("  ⚠️  Error: {}", error_msg),
            Style::default().fg(CatppuccinColors::SUBTEXT1),
        )]));
    }

    result_text.extend(vec![
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Press ",
                Style::default().fg(CatppuccinColors::SUBTEXT1),
            ),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(CatppuccinColors::GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " to continue or ",
                Style::default().fg(CatppuccinColors::SUBTEXT1),
            ),
            Span::styled(
                "q/Esc",
                Style::default()
                    .fg(CatppuccinColors::RED)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " to quit",
                Style::default().fg(CatppuccinColors::SUBTEXT1),
            ),
        ]),
    ]);

    let result_modal = Paragraph::new(result_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title_text)
                .title_style(
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )
                .border_style(Style::default().fg(color)),
        )
        .style(Style::default().bg(CatppuccinColors::BASE))
        .alignment(Alignment::Left);

    f.render_widget(result_modal, popup_area);
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(
            [
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Status bar
            ]
            .as_ref(),
        )
        .split(f.area());

    // Render header
    render_header(f, app, chunks[0]);

    match app.state {
        AppState::Scanning => {
            if app.networks.is_empty() {
                let popup_area = centered_rect(50, 20, f.area());
                f.render_widget(Clear, popup_area);

                let scanning_modal = Paragraph::new(
                    "Scanning for WiFi networks...\n\nPlease wait...",
                )
                .block(Block::default().borders(Borders::ALL).title("Scanning"))
                .style(
                    Style::default()
                        .fg(CatppuccinColors::BLUE)
                        .bg(CatppuccinColors::BASE),
                )
                .alignment(Alignment::Center);

                f.render_widget(scanning_modal, popup_area);
            } else {
                // Show networks as they appear during scanning
                let items: Vec<ListItem> =
                    app.networks.iter().map(create_network_list_item).collect();

                let scanning_title = Line::from(vec![
                    Span::styled(
                        "🔍 ",
                        Style::default().fg(CatppuccinColors::YELLOW),
                    ),
                    Span::styled(
                        "Scanning...",
                        Style::default()
                            .fg(CatppuccinColors::YELLOW)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]);

                let list = List::new(items)
                    .block(
                        Block::default()
                            .style(Style::default().bg(CatppuccinColors::BASE))
                            .title(scanning_title)
                            .borders(Borders::ALL),
                    )
                    .highlight_style(
                        Style::default()
                            .bg(CatppuccinColors::SURFACE0)
                            .fg(CatppuccinColors::TEXT)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("► ");

                f.render_stateful_widget(
                    list,
                    chunks[1],
                    &mut app.list_state.clone(),
                );
            }
        }
        AppState::NetworkList => {
            let items: Vec<ListItem> =
                app.networks.iter().map(create_network_list_item).collect();

            let list_title = Line::from(vec![
                Span::styled(
                    "📶 ",
                    Style::default().fg(CatppuccinColors::BLUE),
                ),
                Span::styled(
                    "WiFi Networks",
                    Style::default()
                        .fg(CatppuccinColors::TEXT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " | ",
                    Style::default().fg(CatppuccinColors::SUBTEXT1),
                ),
                Span::styled(
                    "🔗:Connected ",
                    Style::default().fg(CatppuccinColors::GREEN),
                ),
                Span::styled(
                    "🔒:Secured ",
                    Style::default().fg(CatppuccinColors::MAUVE),
                ),
                Span::styled(
                    "2.4G/5G:Band",
                    Style::default().fg(CatppuccinColors::SAPPHIRE),
                ),
            ]);

            let list = List::new(items)
                .block(
                    Block::default()
                        .title(list_title)
                        .borders(Borders::ALL)
                        .style(Style::default().bg(CatppuccinColors::BASE)),
                )
                .highlight_style(
                    Style::default()
                        .bg(CatppuccinColors::SURFACE0)
                        .fg(CatppuccinColors::TEXT)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("► ");

            f.render_stateful_widget(
                list,
                chunks[1],
                &mut app.list_state.clone(),
            );
        }
        AppState::Help => {
            render_help_screen(f, app, chunks[1]);
        }
        AppState::NetworkDetails => {
            let items: Vec<ListItem> =
                app.networks.iter().map(create_network_list_item).collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .style(Style::default().bg(CatppuccinColors::BASE)),
                )
                .highlight_style(
                    Style::default()
                        .bg(CatppuccinColors::SURFACE0)
                        .fg(CatppuccinColors::TEXT)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("► ");

            f.render_stateful_widget(
                list,
                chunks[1],
                &mut app.list_state.clone(),
            );

            render_network_details(f, app);
        }
        AppState::PasswordInput => {
            let items: Vec<ListItem> =
                app.networks.iter().map(create_network_list_item).collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .style(Style::default().bg(CatppuccinColors::BASE)),
                )
                .highlight_style(
                    Style::default()
                        .bg(CatppuccinColors::SURFACE0)
                        .fg(CatppuccinColors::TEXT)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("► ");

            f.render_stateful_widget(
                list,
                chunks[1],
                &mut app.list_state.clone(),
            );

            render_enhanced_password_modal(f, app);
        }
        AppState::Connecting => {
            let items: Vec<ListItem> =
                app.networks.iter().map(create_network_list_item).collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .style(Style::default().bg(CatppuccinColors::BASE)),
                )
                .highlight_style(
                    Style::default()
                        .bg(CatppuccinColors::SURFACE0)
                        .fg(CatppuccinColors::TEXT)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("► ");

            f.render_stateful_widget(
                list,
                chunks[1],
                &mut app.list_state.clone(),
            );

            render_enhanced_connecting_modal(f, app);
        }
        AppState::Disconnecting => {
            let items: Vec<ListItem> =
                app.networks.iter().map(create_network_list_item).collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .style(Style::default().bg(CatppuccinColors::BASE)),
                )
                .highlight_style(
                    Style::default()
                        .bg(CatppuccinColors::SURFACE0)
                        .fg(CatppuccinColors::TEXT)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("► ");

            f.render_stateful_widget(
                list,
                chunks[1],
                &mut app.list_state.clone(),
            );

            render_enhanced_disconnecting_modal(f, app);
        }
        AppState::ConnectionResult => {
            let items: Vec<ListItem> =
                app.networks.iter().map(create_network_list_item).collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .style(Style::default().bg(CatppuccinColors::BASE)),
                )
                .highlight_style(
                    Style::default()
                        .bg(CatppuccinColors::SURFACE0)
                        .fg(CatppuccinColors::TEXT)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("► ");

            f.render_stateful_widget(
                list,
                chunks[1],
                &mut app.list_state.clone(),
            );

            render_enhanced_result_modal(f, app);
        }
    }

    render_status_bar(f, app, chunks[2]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> Result<(), Box<dyn Error>> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if app.should_quit {
            break;
        }

        if app.state == AppState::Scanning {
            // Process events during scanning to allow UI updates and handle input
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()?
                    && key.kind == KeyEventKind::Press
                {
                    match key.code {
                        KeyCode::Esc => {
                            app.quit();
                            continue;
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            if !app.networks.is_empty() {
                                app.next();
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if !app.networks.is_empty() {
                                app.previous();
                            }
                        }
                        KeyCode::Enter | KeyCode::Char('c') => {
                            if !app.networks.is_empty() {
                                app.select_network();
                                continue;
                            }
                        }
                        _ => {}
                    }
                }
                // Continue to redraw with any new events
                continue;
            }

            // Perform incremental scan
            let networks = scan_wifi_networks().await?;
            let previous_count = app.networks.len();
            app.networks = networks;
            app.network_count = app.networks.len();
            app.last_scan_time = Some(Instant::now());

            // Get adapter info on first scan
            if app.adapter_info.is_none() {
                app.adapter_info = get_wifi_adapter_info().await;
            }

            // Update selection when first networks appear or preserve selection
            if previous_count == 0 && !app.networks.is_empty() {
                if app.selected_network.is_some() {
                    app.update_selection_after_rescan();
                } else {
                    app.list_state.select(Some(0));
                }
            }

            // Check if we should finish scanning (after reasonable time or enough networks)
            if !app.networks.is_empty() {
                app.status_message = format!(
                    "Found {} network(s). Ready to connect!",
                    app.networks.len()
                );
                app.state = AppState::NetworkList;
            } else {
                app.status_message =
                    "Scanning for WiFi networks...".to_string();
            }

            continue;
        }

        if app.state == AppState::Connecting {
            if event::poll(Duration::from_millis(100))?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
                && key.code == KeyCode::Esc
            {
                app.quit();
                continue;
            }

            let password = if app.selected_network.as_ref().unwrap().secured {
                Some(app.password_input.as_str())
            } else {
                None
            };

            match connect_to_network(
                app.selected_network.as_ref().unwrap(),
                password,
            )
            .await
            {
                Ok(_) => {
                    app.connection_success = true;
                    app.connection_error = None;
                    app.status_message = "Connected successfully!".to_string();
                }
                Err(e) => {
                    app.connection_success = false;
                    app.connection_error = Some(e.to_string());
                    app.status_message = "Connection failed".to_string();
                }
            }
            app.state = AppState::ConnectionResult;
            continue;
        }

        if app.state == AppState::Disconnecting {
            if event::poll(Duration::from_millis(100))?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
                && key.code == KeyCode::Esc
            {
                app.quit();
                continue;
            }

            match disconnect_from_network(
                app.selected_network.as_ref().unwrap(),
            )
            .await
            {
                Ok(_) => {
                    app.connection_success = true;
                    app.connection_error = None;
                    app.status_message =
                        "Disconnected successfully!".to_string();
                }
                Err(e) => {
                    app.connection_success = false;
                    app.connection_error = Some(e.to_string());
                    app.status_message = "Disconnection failed".to_string();
                }
            }
            app.state = AppState::ConnectionResult;
            continue;
        }

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match app.state {
                AppState::Scanning => {
                    // Handled above in the scanning loop
                }
                AppState::NetworkList => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => app.quit(),
                    KeyCode::Char('j') | KeyCode::Down => app.next(),
                    KeyCode::Char('k') | KeyCode::Up => app.previous(),
                    KeyCode::Enter | KeyCode::Char('c') => app.select_network(),
                    KeyCode::Char('d') => {
                        if let Some(network) = app
                            .networks
                            .get(app.selected_index)
                            .filter(|n| n.connected)
                            .cloned()
                        {
                            app.is_disconnect_operation = true;
                            app.state = AppState::Disconnecting;
                            app.connection_start_time = Some(Instant::now());
                            app.status_message = format!(
                                "Disconnecting from {}...",
                                network.ssid
                            );

                            app.selected_network = Some(network);
                        }
                    }
                    KeyCode::Char('r') => {
                        app.state = AppState::Scanning;
                        app.status_message =
                            "Scanning for networks...".to_string();
                        app.networks.clear();
                    }
                    KeyCode::Char('h') => {
                        app.state = AppState::Help;
                    }
                    KeyCode::Char('i') => {
                        if !app.networks.is_empty() {
                            app.state = AppState::NetworkDetails;
                        }
                    }
                    _ => {}
                },
                AppState::Help => match key.code {
                    KeyCode::Esc | KeyCode::Char('h') | KeyCode::Char('q') => {
                        app.state = AppState::NetworkList;
                    }
                    _ => {}
                },
                AppState::NetworkDetails => match key.code {
                    KeyCode::Esc | KeyCode::Char('i') | KeyCode::Char('q') => {
                        app.state = AppState::NetworkList;
                    }
                    _ => {}
                },
                AppState::PasswordInput => match key.code {
                    KeyCode::Esc => {
                        app.state = AppState::NetworkList;
                        app.password_input.clear();
                        app.password_visible = false;
                    }
                    KeyCode::Enter => app.confirm_password(),
                    KeyCode::Backspace => app.remove_char_from_password(),
                    KeyCode::Tab => {
                        app.password_visible = !app.password_visible;
                    }
                    KeyCode::Char(c) => app.add_char_to_password(c),
                    _ => {}
                },
                AppState::Connecting => {
                    // Handled above in the connecting loop
                }
                AppState::Disconnecting => {
                    // Handled above in the disconnecting loop
                }
                AppState::ConnectionResult => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => app.quit(),
                    KeyCode::Enter => {
                        // Always return to network list after connection result
                        app.back_to_network_list();
                        // Rescan to update connection status
                        app.state = AppState::Scanning;
                        app.status_message =
                            "Scanning for networks...".to_string();
                        app.networks.clear();
                    }
                    _ => {}
                },
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new();
    let res = run_app(&mut terminal, app).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}
