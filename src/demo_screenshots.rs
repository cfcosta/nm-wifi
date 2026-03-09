mod render;
mod scenarios;
mod svg;
mod writer;

pub use render::{HEIGHT, WIDTH, render_app};
pub use scenarios::{DemoScreen, build_demo_screen, demo_shot_apps};
pub use svg::buffer_to_svg;
pub use writer::{write_demo_svgs, write_demo_svgs_with_backend};

#[cfg(all(test, feature = "demo"))]
mod tests {
    use super::{buffer_to_svg, demo_shot_apps, render_app};
    use crate::network::demo_networks;

    fn buffer_text(buffer: &ratatui::buffer::Buffer) -> String {
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
    fn demo_shot_apps_cover_all_documented_screens() {
        let names: Vec<_> = demo_shot_apps(&demo_networks())
            .into_iter()
            .map(|(name, _)| name)
            .collect();

        assert_eq!(
            names,
            vec![
                "scanning.svg",
                "network-list.svg",
                "help.svg",
                "details.svg",
                "password.svg",
                "connecting.svg",
                "disconnecting.svg",
                "result-success.svg",
                "result-error.svg",
            ]
        );
    }

    #[test]
    fn rendered_demo_screens_export_valid_svg_shell() {
        let (_, app) = demo_shot_apps(&demo_networks())
            .into_iter()
            .find(|(name, _)| *name == "result-error.svg")
            .expect("result error screen exists");

        let buffer = render_app(&app).expect("render succeeds");
        let svg = buffer_to_svg(&buffer);
        let text = buffer_text(&buffer);

        assert!(svg.starts_with("<svg "));
        assert!(text.contains("Failed to find WiFi device in NetworkManager"));
        assert!(text.contains("CatCat"));
    }
}
