use ratatui::{
    buffer::Buffer,
    style::{Color, Modifier},
};

use crate::theme::CatppuccinColors;

const CELL_WIDTH: u32 = 10;
const CELL_HEIGHT: u32 = 20;

pub fn buffer_to_svg(buffer: &Buffer) -> String {
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
        "#000000", "#800000", "#008000", "#808000", "#000080", "#800080",
        "#008080", "#c0c0c0", "#808080", "#ff0000", "#00ff00", "#ffff00",
        "#0000ff", "#ff00ff", "#00ffff", "#ffffff",
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

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use super::{color_to_hex, escape_xml};
    use crate::theme::CatppuccinColors;

    #[test]
    fn escape_xml_escapes_svg_metacharacters() {
        assert_eq!(escape_xml("<&>\"'"), "&lt;&amp;&gt;&quot;&apos;");
    }

    #[test]
    fn ansi_and_reset_colors_serialize_stably() {
        assert_eq!(
            color_to_hex(Color::Reset, CatppuccinColors::BASE),
            "#1e1e2e"
        );
        assert_eq!(
            color_to_hex(Color::Indexed(196), CatppuccinColors::BASE),
            "#ff0000"
        );
        assert_eq!(
            color_to_hex(Color::Rgb(205, 214, 244), CatppuccinColors::BASE),
            "#cdd6f4"
        );
    }
}
