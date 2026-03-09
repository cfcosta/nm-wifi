use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::{
    app_state::{App, AppState},
    theme::CatppuccinColors,
};

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

pub fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(30),
            Constraint::Min(0),
            Constraint::Length(25),
        ])
        .split(area);

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

    let status = Paragraph::new(app.status_message.as_str())
        .block(Block::default().borders(Borders::ALL))
        .style(
            Style::default()
                .fg(CatppuccinColors::SUBTEXT1)
                .bg(CatppuccinColors::BASE),
        )
        .alignment(Alignment::Left);

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
