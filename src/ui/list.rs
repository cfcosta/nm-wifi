use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};

use super::format::{
    create_signal_graph,
    format_signal_strength,
    format_ssid_column,
    get_frequency_band,
};
use crate::{app_state::App, theme::CatppuccinColors, wifi::WifiNetwork};

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

pub(crate) fn render_network_list_background(
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
