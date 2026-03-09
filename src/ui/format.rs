use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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
