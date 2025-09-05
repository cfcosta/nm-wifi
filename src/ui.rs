use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use crate::{
    theme::CatppuccinColors,
    types::{App, AppState, WifiNetwork},
};

pub fn create_signal_graph(strength: u8) -> String {
    let bars = (strength as f32 / 100.0 * 20.0) as usize;
    let filled = "█".repeat(bars);
    let empty = "░".repeat(20 - bars);
    format!("{}{}", filled, empty)
}

pub fn get_frequency_band(frequency: u32) -> &'static str {
    if frequency >= 5000 { "5G" } else { "2.4G" }
}

pub fn format_signal_strength(strength: u8) -> String {
    format!("{}%", strength)
}

pub fn create_network_list_item<'a>(network: &WifiNetwork) -> ListItem<'a> {
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

pub fn render_help_screen(f: &mut Frame, _app: &App, area: Rect) {
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

pub fn render_network_details(f: &mut Frame, app: &App) {
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

pub fn create_progress_bar(progress: f32, width: usize) -> String {
    let filled = ((progress * width as f32) as usize).min(width);
    let empty = width - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

pub fn get_connection_animation_frame(elapsed_ms: u128) -> char {
    let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    frames[(elapsed_ms / 100) as usize % frames.len()]
}

pub fn render_enhanced_password_modal(f: &mut Frame, app: &App) {
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

pub fn render_enhanced_connecting_modal(f: &mut Frame, app: &App) {
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

pub fn render_enhanced_disconnecting_modal(f: &mut Frame, app: &App) {
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

pub fn render_enhanced_result_modal(f: &mut Frame, app: &App) {
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
