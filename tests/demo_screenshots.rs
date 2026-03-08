use nm_wifi::theme::CatppuccinColors;
#[cfg(feature = "demo")]
use nm_wifi::{
    demo_screenshots::{demo_shot_apps, write_demo_svgs},
    network::demo_networks,
};
use ratatui::style::Color;

#[test]
fn theme_palette_exposes_expected_base_colors() {
    assert_eq!(CatppuccinColors::BASE, Color::Rgb(30, 30, 46));
    assert_eq!(CatppuccinColors::TEXT, Color::Rgb(205, 214, 244));
}

#[cfg(feature = "demo")]
#[test]
fn demo_screenshot_manifest_includes_error_and_disconnect_flows() {
    let names: Vec<_> = demo_shot_apps(&demo_networks())
        .into_iter()
        .map(|(name, _)| name)
        .collect();

    assert!(names.contains(&"disconnecting.svg"));
    assert!(names.contains(&"result-error.svg"));
}

#[cfg(feature = "demo")]
#[test]
fn screenshot_writer_creates_svg_files() {
    let mut output_dir = std::env::temp_dir();
    output_dir.push(format!("nm-wifi-screens-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&output_dir);

    write_demo_svgs(output_dir.as_path(), &demo_networks()).expect("screenshot generation succeeds");

    let network_list = std::fs::read_to_string(output_dir.join("network-list.svg"))
        .expect("network list svg exists");
    assert!(network_list.starts_with("<svg "));
    assert!(network_list.contains("font-family=\"monospace\""));

    let _ = std::fs::remove_dir_all(&output_dir);
}
