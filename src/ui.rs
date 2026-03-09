use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::{
    app_state::{App, AppState},
    theme::CatppuccinColors,
    wifi::WifiNetwork,
};

pub fn create_signal_graph(strength: u8) -> String {
    let bars = (strength as f32 / 100.0 * 20.0) as usize;
    let filled = "█".repeat(bars);
    let empty = "░".repeat(20 - bars);
    format!("{}{}", filled, empty)
}

pub fn get_frequency_band(frequency: u32) -> &'static str {
    match frequency {
        5925.. => "6G",
        5000.. => "5G",
        _ => "2.4G",
    }
}

pub fn format_signal_strength(strength: u8) -> String {
    format!("{}%", strength)
}

pub fn format_ssid_column(ssid: &str, width: usize) -> String {
    let mut formatted = String::new();
    let mut current_width = 0;

    for ch in ssid.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_width + ch_width > width {
            break;
        }

        formatted.push(ch);
        current_width += ch_width;
    }

    let padding =
        width.saturating_sub(UnicodeWidthStr::width(formatted.as_str()));
    formatted.push_str(&" ".repeat(padding));
    formatted
}

pub fn keybindings_hint(state: &AppState) -> &'static str {
    match state {
        AppState::NetworkList => {
            "↑↓/jk Move  Enter Connect  d Disconnect  r Rescan  i Info  h Help  q Quit"
        }
        AppState::Help => "h/q/Esc Back",
        AppState::NetworkDetails => "q/i/Esc Back",
        AppState::PasswordInput => "Enter Connect  Tab Show/Hide  Esc Cancel",
        AppState::Connecting | AppState::Disconnecting => "Esc Quit",
        AppState::Scanning => "Scanning  Esc Quit",
        AppState::ConnectionResult => "Enter Return  q/Esc Quit",
    }
}

pub fn create_network_list_item<'a>(network: &WifiNetwork) -> ListItem<'a> {
    let signal_graph = create_signal_graph(network.signal_strength);
    let signal_percent = format_signal_strength(network.signal_strength);
    let frequency_band = get_frequency_band(network.frequency);
    let security_icon = if network.is_secured() { "🔒" } else { "  " };
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
            format_ssid_column(&network.ssid, 24),
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

fn render_network_list_background(
    f: &mut Frame,
    app: &App,
    area: Rect,
    title: Option<Line<'static>>,
) {
    let items: Vec<ListItem> =
        app.networks.iter().map(create_network_list_item).collect();

    let mut block =
        Block::default().style(Style::default().bg(CatppuccinColors::BASE));
    if let Some(title) = title {
        block = block.title(title);
    }
    block = block.borders(Borders::ALL);

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(CatppuccinColors::SURFACE0)
                .fg(CatppuccinColors::TEXT)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("► ");

    let mut list_state = ListState::default();
    if !app.networks.is_empty() {
        list_state.select(Some(app.selected_index.min(app.networks.len() - 1)));
    }

    f.render_stateful_widget(list, area, &mut list_state);
}

pub fn render_header(f: &mut Frame, app: &App, area: Rect) {
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
            concat!(" v", env!("CARGO_PKG_VERSION")),
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
    let adapter_text = app.adapter_name.as_deref().unwrap_or("WiFi Adapter");
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

pub fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
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
    let hints = Paragraph::new(keybindings_hint(&app.state))
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

pub fn render_help_screen(f: &mut Frame, _app: &App, area: Rect) {
    let help_text = vec![
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default()
                .fg(CatppuccinColors::MAUVE)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from("↑/k        Move up"),
        Line::from("↓/j        Move down"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions",
            Style::default()
                .fg(CatppuccinColors::MAUVE)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from("Enter/c    Connect or disconnect selection"),
        Line::from("d          Disconnect selected active network"),
        Line::from("r          Rescan networks"),
        Line::from("i          Show network details"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Other",
            Style::default()
                .fg(CatppuccinColors::MAUVE)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from("h          Show help"),
        Line::from("q/Esc      Quit application"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Markers",
            Style::default()
                .fg(CatppuccinColors::MAUVE)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from("Link icon   Connected network"),
        Line::from("Lock icon   Protected network"),
        Line::from("2.4G/5G     Frequency band"),
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

pub fn render_network_details(f: &mut Frame, app: &App) {
    if let Some(network) = app.selected_network_in_list() {
        let popup_area = centered_rect(60, 70, f.area());
        f.render_widget(Clear, popup_area);

        let security_type = network.security.display_name();

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

fn modal_shadow_area(popup_area: Rect) -> Rect {
    Rect {
        x: popup_area.x + 1,
        y: popup_area.y + 1,
        width: popup_area.width,
        height: popup_area.height,
    }
}

fn render_modal_shell(f: &mut Frame, popup_area: Rect) {
    f.render_widget(Clear, popup_area);
    f.render_widget(
        Block::default().style(Style::default().bg(CatppuccinColors::SURFACE0)),
        modal_shadow_area(popup_area),
    );
}

fn modal_block<'a>(title: &'a str, border_color: Color) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(
            Style::default()
                .fg(border_color)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(border_color))
}

fn render_modal(
    f: &mut Frame,
    popup_area: Rect,
    title: &str,
    border_color: Color,
    lines: Vec<Line<'static>>,
) {
    render_modal_shell(f, popup_area);
    let modal = Paragraph::new(lines)
        .block(modal_block(title, border_color))
        .style(Style::default().bg(CatppuccinColors::BASE))
        .alignment(Alignment::Left);

    f.render_widget(modal, popup_area);
}

fn network_summary_lines(
    network: &WifiNetwork,
    include_signal: bool,
) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(format!("Network: {}", network.ssid)),
        Line::from(format!("Security: {}", network.security.display_name())),
    ];

    if include_signal {
        lines.push(Line::from(format!(
            "Signal: {}% ({})",
            network.signal_strength,
            get_frequency_band(network.frequency)
        )));
    }

    lines
}

pub fn render_enhanced_password_modal(f: &mut Frame, app: &App) {
    if let Some(network) = &app.selected_network {
        let popup_area = centered_rect(64, 28, f.area());
        let password_display = if app.password_visible {
            app.password_input.clone()
        } else {
            "•".repeat(app.password_input.len())
        };
        let password_field = format!("{:<38}", password_display);

        let mut password_text = network_summary_lines(network, false);
        password_text.extend([
            Line::from(""),
            Line::from("Password:"),
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
                    password_field,
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
            Line::from("Enter: connect"),
            Line::from("Tab: show or hide password"),
            Line::from("Esc: cancel"),
        ]);

        render_modal(
            f,
            popup_area,
            "Password",
            CatppuccinColors::BLUE,
            password_text,
        );
    }
}

pub fn render_enhanced_connecting_modal(f: &mut Frame, app: &App) {
    if let Some(network) = &app.selected_network {
        let popup_area = centered_rect(64, 28, f.area());
        let mut connecting_text = network_summary_lines(network, true);
        connecting_text.extend([
            Line::from(""),
            Line::from("Activating connection via NetworkManager..."),
            Line::from("Press Esc to quit the application."),
        ]);

        render_modal(
            f,
            popup_area,
            "Connecting",
            CatppuccinColors::YELLOW,
            connecting_text,
        );
    }
}

pub fn render_enhanced_disconnecting_modal(f: &mut Frame, app: &App) {
    if let Some(network) = &app.selected_network {
        let popup_area = centered_rect(64, 24, f.area());
        let mut disconnecting_text = network_summary_lines(network, false);
        disconnecting_text.extend([
            Line::from("Disconnecting via NetworkManager..."),
            Line::from("Press Esc to quit the application."),
        ]);

        render_modal(
            f,
            popup_area,
            "Disconnecting",
            CatppuccinColors::PEACH,
            disconnecting_text,
        );
    }
}

pub fn render_enhanced_result_modal(f: &mut Frame, app: &App) {
    let popup_area = centered_rect(68, 38, f.area());

    let (title, color) = if app.connection_success {
        if app.is_disconnect_operation {
            ("Disconnection complete", CatppuccinColors::GREEN)
        } else {
            ("Connection complete", CatppuccinColors::GREEN)
        }
    } else if app.is_disconnect_operation {
        ("Disconnection failed", CatppuccinColors::RED)
    } else {
        ("Connection failed", CatppuccinColors::RED)
    };

    let mut result_text = vec![];

    if let Some(network) = &app.selected_network {
        result_text.extend(network_summary_lines(network, true));
    } else {
        result_text.push(Line::from("Network: Unknown"));
    }

    if let Some(interface_name) = app.adapter_name.as_deref() {
        result_text.push(Line::from(format!("Interface: {}", interface_name)));
    }

    result_text.push(Line::from(""));

    if app.connection_success {
        result_text
            .push(Line::from("Status: NetworkManager reported success."));
    } else {
        let error_msg =
            app.connection_error.as_deref().unwrap_or("Unknown error");
        result_text.push(Line::from(vec![
            Span::styled(
                "Error: ",
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                error_msg.to_string(),
                Style::default().fg(CatppuccinColors::TEXT),
            ),
        ]));
    }

    result_text.extend([
        Line::from(""),
        Line::from("Enter: return to the network list"),
        Line::from("q/Esc: quit"),
    ]);

    render_modal(f, popup_area, title, color, result_text);
}

pub fn ui(f: &mut Frame, app: &App) {
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

                render_network_list_background(
                    f,
                    app,
                    chunks[1],
                    Some(scanning_title),
                );
            }
        }
        AppState::NetworkList => {
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

            render_network_list_background(f, app, chunks[1], Some(list_title));
        }
        AppState::Help => {
            render_help_screen(f, app, chunks[1]);
        }
        AppState::NetworkDetails => {
            render_network_list_background(f, app, chunks[1], None);
            render_network_details(f, app);
        }
        AppState::PasswordInput => {
            render_network_list_background(f, app, chunks[1], None);
            render_enhanced_password_modal(f, app);
        }
        AppState::Connecting => {
            render_network_list_background(f, app, chunks[1], None);
            render_enhanced_connecting_modal(f, app);
        }
        AppState::Disconnecting => {
            render_network_list_background(f, app, chunks[1], None);
            render_enhanced_disconnecting_modal(f, app);
        }
        AppState::ConnectionResult => {
            render_network_list_background(f, app, chunks[1], None);
            render_enhanced_result_modal(f, app);
        }
    }

    render_status_bar(f, app, chunks[2]);
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
