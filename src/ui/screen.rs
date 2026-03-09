use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::{
    header_footer::{render_header, render_status_bar},
    list::render_network_list_background,
    modals::{
        centered_rect,
        render_enhanced_connecting_modal,
        render_enhanced_disconnecting_modal,
        render_enhanced_password_modal,
        render_enhanced_result_modal,
        render_help_screen,
        render_network_details,
    },
};
use crate::{
    app_state::{App, AppState},
    theme::CatppuccinColors,
};

pub fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.area());

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
