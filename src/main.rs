use std::{collections::HashMap, error::Error, io, time::Duration};

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
use tokio::time::sleep;

#[derive(Debug, Clone)]
struct WifiNetwork {
    ssid: String,
    signal_strength: u8,
    secured: bool,
    frequency: u32,
}

#[derive(PartialEq)]
enum AppState {
    Scanning,
    NetworkList,
    PasswordInput,
    Connecting,
    ConnectionResult,
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
        if let Some(network) = self.networks.get(self.selected_index).cloned() {
            self.selected_network = Some(network.clone());
            if network.secured {
                self.state = AppState::PasswordInput;
                self.password_input.clear();
            } else {
                self.state = AppState::Connecting;
                self.status_message =
                    format!("Connecting to {}...", network.ssid);
            }
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
        self.selected_network = None;
        self.password_input.clear();
    }
}

async fn scan_wifi_networks() -> Result<Vec<WifiNetwork>, Box<dyn Error>> {
    let dbus = dbus::blocking::Connection::new_system()
        .map_err(|_| "Failed to connect to D-Bus".to_string())?;
    let nm = NetworkManager::new(&dbus);

    let devices = nm
        .get_devices()
        .map_err(|_| "Failed to get devices".to_string())?;

    for device in devices {
        if let Device::WiFi(wifi_device) = device {
            wifi_device
                .request_scan(HashMap::new())
                .map_err(|_| "Failed to request scan".to_string())?;

            sleep(Duration::from_secs(3)).await;

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

                    let network = WifiNetwork {
                        ssid,
                        signal_strength,
                        secured,
                        frequency,
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
            deduplicated_networks
                .sort_by(|a, b| b.signal_strength.cmp(&a.signal_strength));
            return Ok(deduplicated_networks);
        }
    }

    Ok(Vec::new())
}

async fn connect_to_network(
    network: &WifiNetwork,
    password: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    use tokio::process::Command;

    // First, try to connect to existing connection
    let mut cmd = Command::new("nmcli");
    cmd.args(&["connection", "up", &network.ssid]);

    let output = cmd.output().await?;

    if output.status.success() {
        return Ok(());
    }

    // If that fails, create a new connection
    let mut cmd = Command::new("nmcli");
    cmd.args(&["device", "wifi", "connect", &network.ssid]);

    if let Some(pwd) = password {
        cmd.args(&["password", pwd]);
    }

    let output = cmd.output().await?;

    if output.status.success() {
        Ok(())
    } else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        Err(format!("Connection failed: {}", error_msg.trim()).into())
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(f.area());

    match app.state {
        AppState::Scanning => {
            let popup_area = centered_rect(50, 20, f.area());
            f.render_widget(Clear, popup_area);

            let scanning_modal = Paragraph::new(
                "Scanning for WiFi networks...\n\nPlease wait...",
            )
            .block(Block::default().borders(Borders::ALL).title("Scanning"))
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center);

            f.render_widget(scanning_modal, popup_area);
        }
        AppState::NetworkList => {
            let items: Vec<ListItem> = app
                .networks
                .iter()
                .map(|network| {
                    let signal_bars = "█"
                        .repeat((network.signal_strength / 25).max(1) as usize);
                    let security_icon =
                        if network.secured { "🔒" } else { "  " };

                    ListItem::new(Line::from(vec![Span::styled(
                        format!(
                            "{} {} {}",
                            security_icon, network.ssid, signal_bars
                        ),
                        Style::default(),
                    )]))
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("WiFi Networks"),
                )
                .highlight_style(
                    Style::default().add_modifier(Modifier::REVERSED),
                )
                .highlight_symbol("► ");

            f.render_stateful_widget(
                list,
                chunks[0],
                &mut app.list_state.clone(),
            );
        }
        AppState::PasswordInput => {
            let items: Vec<ListItem> = app
                .networks
                .iter()
                .map(|network| {
                    let signal_bars = "█"
                        .repeat((network.signal_strength / 25).max(1) as usize);
                    let security_icon =
                        if network.secured { "🔒" } else { "  " };

                    ListItem::new(Line::from(vec![Span::styled(
                        format!(
                            "{} {} {}",
                            security_icon, network.ssid, signal_bars
                        ),
                        Style::default(),
                    )]))
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("WiFi Networks"),
                )
                .highlight_style(
                    Style::default().add_modifier(Modifier::REVERSED),
                )
                .highlight_symbol("► ");

            f.render_stateful_widget(
                list,
                chunks[0],
                &mut app.list_state.clone(),
            );

            let popup_area = centered_rect(50, 20, f.area());
            f.render_widget(Clear, popup_area);

            let password_input = Paragraph::new(format!(
                "Password: {}",
                "*".repeat(app.password_input.len())
            ))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Enter Password"),
            )
            .style(Style::default().fg(Color::Yellow));

            f.render_widget(password_input, popup_area);
        }
        AppState::Connecting => {
            let items: Vec<ListItem> = app
                .networks
                .iter()
                .map(|network| {
                    let signal_bars = "█"
                        .repeat((network.signal_strength / 25).max(1) as usize);
                    let security_icon =
                        if network.secured { "🔒" } else { "  " };

                    ListItem::new(Line::from(vec![Span::styled(
                        format!(
                            "{} {} {}",
                            security_icon, network.ssid, signal_bars
                        ),
                        Style::default(),
                    )]))
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("WiFi Networks"),
                )
                .highlight_style(
                    Style::default().add_modifier(Modifier::REVERSED),
                )
                .highlight_symbol("► ");

            f.render_stateful_widget(
                list,
                chunks[0],
                &mut app.list_state.clone(),
            );

            let popup_area = centered_rect(50, 20, f.area());
            f.render_widget(Clear, popup_area);

            let network_name = app
                .selected_network
                .as_ref()
                .map(|n| n.ssid.as_str())
                .unwrap_or("Unknown");

            let connecting_modal = Paragraph::new(format!(
                "Connecting to {}...\n\nPlease wait...",
                network_name
            ))
            .block(Block::default().borders(Borders::ALL).title("Connecting"))
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);

            f.render_widget(connecting_modal, popup_area);
        }
        AppState::ConnectionResult => {
            let items: Vec<ListItem> = app
                .networks
                .iter()
                .map(|network| {
                    let signal_bars = "█"
                        .repeat((network.signal_strength / 25).max(1) as usize);
                    let security_icon =
                        if network.secured { "🔒" } else { "  " };

                    ListItem::new(Line::from(vec![Span::styled(
                        format!(
                            "{} {} {}",
                            security_icon, network.ssid, signal_bars
                        ),
                        Style::default(),
                    )]))
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("WiFi Networks"),
                )
                .highlight_style(
                    Style::default().add_modifier(Modifier::REVERSED),
                )
                .highlight_symbol("► ");

            f.render_stateful_widget(
                list,
                chunks[0],
                &mut app.list_state.clone(),
            );

            let popup_area = centered_rect(60, 25, f.area());
            f.render_widget(Clear, popup_area);

            let (message, color, title) = if app.connection_success {
                let network_name = app
                    .selected_network
                    .as_ref()
                    .map(|n| n.ssid.as_str())
                    .unwrap_or("Unknown");
                (
                    format!(
                        "Successfully connected to {}!\n\nPress Enter to continue or Esc to quit",
                        network_name
                    ),
                    Color::Green,
                    "Connection Successful",
                )
            } else {
                let error_msg =
                    app.connection_error.as_deref().unwrap_or("Unknown error");
                (
                    format!(
                        "Failed to connect to network.\n\nError: {}\n\nPress Enter to try again or Esc to quit",
                        error_msg
                    ),
                    Color::Red,
                    "Connection Failed",
                )
            };

            let result_modal = Paragraph::new(message)
                .block(Block::default().borders(Borders::ALL).title(title))
                .style(Style::default().fg(color))
                .alignment(Alignment::Center);

            f.render_widget(result_modal, popup_area);
        }
    }

    let status = Paragraph::new(app.status_message.as_str())
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .alignment(Alignment::Center);
    f.render_widget(status, chunks[1]);
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
            if event::poll(Duration::from_millis(100))?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
                && key.code == KeyCode::Esc
            {
                app.quit();
                continue;
            }

            let networks = scan_wifi_networks().await?;
            app.networks = networks;

            if app.networks.is_empty() {
                app.status_message =
                    "No networks found. Press 'r' to rescan or Esc to quit"
                        .to_string();
            } else {
                app.status_message =
                    "Use ↑/↓ or j/k to navigate, Enter to select, Esc to quit"
                        .to_string();
                app.list_state.select(Some(0));
            }

            app.state = AppState::NetworkList;
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
                    KeyCode::Enter => app.select_network(),
                    KeyCode::Char('r') => {
                        app.state = AppState::Scanning;
                        app.status_message =
                            "Scanning for networks...".to_string();
                        app.networks.clear();
                    }
                    _ => {}
                },
                AppState::PasswordInput => match key.code {
                    KeyCode::Esc => {
                        app.state = AppState::NetworkList;
                        app.password_input.clear();
                    }
                    KeyCode::Enter => app.confirm_password(),
                    KeyCode::Backspace => app.remove_char_from_password(),
                    KeyCode::Char(c) => app.add_char_to_password(c),
                    _ => {}
                },
                AppState::Connecting => {
                    // Handled above in the connecting loop
                }
                AppState::ConnectionResult => match key.code {
                    KeyCode::Esc => app.quit(),
                    KeyCode::Enter => {
                        if app.connection_success {
                            break; // Exit the app on successful connection
                        } else {
                            app.back_to_network_list();
                        }
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
