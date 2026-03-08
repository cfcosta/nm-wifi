use std::{error::Error, fs, path::Path, time::Instant};

use nm_wifi::{
    network::scan_wifi_networks,
    theme::CatppuccinColors,
    types::{App, AppState, WifiNetwork},
    ui::ui,
};
use ratatui::{
    Terminal,
    backend::TestBackend,
    buffer::Buffer,
    style::{Color, Modifier},
};

const WIDTH: u16 = 120;
const HEIGHT: u16 = 36;
const CELL_WIDTH: u32 = 10;
const CELL_HEIGHT: u32 = 20;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let output_dir = Path::new("docs/screenshots");
    fs::create_dir_all(output_dir)?;

    let networks = scan_wifi_networks().await?;

    let shots = vec![
        ("scanning.svg", scanning_app()),
        ("network-list.svg", network_list_app(&networks)),
        ("help.svg", help_app(&networks)),
        ("details.svg", details_app(&networks)),
        ("password.svg", password_app(&networks)),
        ("connecting.svg", connecting_app(&networks)),
        ("disconnecting.svg", disconnecting_app(&networks)),
        ("result-success.svg", result_success_app(&networks)),
        ("result-error.svg", result_error_app(&networks)),
    ];

    for (file_name, app) in shots {
        let buffer = render_app(&app)?;
        let svg = buffer_to_svg(&buffer);
        fs::write(output_dir.join(file_name), svg)?;
    }

    println!("Generated demo screenshots in {}", output_dir.display());
    Ok(())
}

fn render_app(app: &App) -> Result<Buffer, Box<dyn Error>> {
    let backend = TestBackend::new(WIDTH, HEIGHT);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|frame| ui(frame, app))?;
    Ok(terminal.backend().buffer().clone())
}

fn base_app(networks: &[WifiNetwork]) -> App {
    let mut app = App::new();
    app.networks = networks.to_vec();
    app.network_count = app.networks.len();
    app.adapter_info = Some("demo-wlan0".to_string());
    app.list_state.select(Some(0));
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
    app.list_state.select(Some(1));
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
        .find(|network| network.security.display_name() == "WPA3 Personal")
        .cloned()
        .unwrap_or_else(|| networks[0].clone());
    app.state = AppState::ConnectionResult;
    app.selected_network = Some(network);
    app.connection_success = false;
    app.connection_error = Some("Failed to find WiFi device in NetworkManager".to_string());
    app.status_message = "Connection failed".to_string();
    app
}

fn buffer_to_svg(buffer: &Buffer) -> String {
    let width = u32::from(buffer.area.width) * CELL_WIDTH;
    let height = u32::from(buffer.area.height) * CELL_HEIGHT;
    let mut svg = String::new();

    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#
    ));
    svg.push_str(r##"<rect width="100%" height="100%" fill="#1e1e2e"/>"##);
    svg.push_str(r#"<g font-family="monospace" font-size="15">"#);

    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            let px = u32::from(x) * CELL_WIDTH;
            let py = u32::from(y) * CELL_HEIGHT;
            let bg = color_to_hex(cell.bg, CatppuccinColors::BASE);
            let fg = color_to_hex(cell.fg, CatppuccinColors::TEXT);

            svg.push_str(&format!(
                r#"<rect x="{px}" y="{py}" width="{CELL_WIDTH}" height="{CELL_HEIGHT}" fill="{bg}"/>"#
            ));

            if cell.symbol().trim().is_empty() {
                continue;
            }

            let weight = if cell.modifier.contains(Modifier::BOLD) {
                "700"
            } else {
                "400"
            };
            let text = escape_xml(cell.symbol());
            let text_x = px + 1;
            let text_y = py + CELL_HEIGHT - 5;
            svg.push_str(&format!(
                r#"<text x="{text_x}" y="{text_y}" fill="{fg}" font-weight="{weight}">{text}</text>"#
            ));
        }
    }

    svg.push_str("</g></svg>");
    svg
}

fn color_to_hex(color: Color, reset: Color) -> String {
    let normalized = match color {
        Color::Reset => reset,
        other => other,
    };

    match normalized {
        Color::Reset => color_to_hex(reset, reset),
        Color::Black => "#000000".to_string(),
        Color::Red => "#ff5555".to_string(),
        Color::Green => "#50fa7b".to_string(),
        Color::Yellow => "#f1fa8c".to_string(),
        Color::Blue => "#8be9fd".to_string(),
        Color::Magenta => "#ff79c6".to_string(),
        Color::Cyan => "#8be9fd".to_string(),
        Color::Gray => "#bfbfbf".to_string(),
        Color::DarkGray => "#666666".to_string(),
        Color::LightRed => "#ff6e6e".to_string(),
        Color::LightGreen => "#69ff94".to_string(),
        Color::LightYellow => "#ffffa5".to_string(),
        Color::LightBlue => "#d6acff".to_string(),
        Color::LightMagenta => "#ff92df".to_string(),
        Color::LightCyan => "#a4ffff".to_string(),
        Color::White => "#ffffff".to_string(),
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Indexed(idx) => ansi_index_to_hex(idx),
    }
}

fn ansi_index_to_hex(idx: u8) -> String {
    const BASIC: [&str; 16] = [
        "#000000", "#800000", "#008000", "#808000", "#000080", "#800080", "#008080", "#c0c0c0",
        "#808080", "#ff0000", "#00ff00", "#ffff00", "#0000ff", "#ff00ff", "#00ffff", "#ffffff",
    ];

    if idx < 16 {
        return BASIC[idx as usize].to_string();
    }

    if idx <= 231 {
        let i = idx - 16;
        let r = i / 36;
        let g = (i / 6) % 6;
        let b = i % 6;
        let conv = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
        return format!("#{:02x}{:02x}{:02x}", conv(r), conv(g), conv(b));
    }

    let gray = 8 + (idx - 232) * 10;
    format!("#{gray:02x}{gray:02x}{gray:02x}")
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
