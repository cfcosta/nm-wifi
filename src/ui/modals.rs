use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::format::get_frequency_band;
use crate::{app_state::App, theme::CatppuccinColors, wifi::WifiNetwork};

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
