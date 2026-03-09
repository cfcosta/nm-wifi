use std::error::Error;

use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

use crate::{app_state::App, ui::ui};

pub const WIDTH: u16 = 120;
pub const HEIGHT: u16 = 36;

pub fn render_app(app: &App) -> Result<Buffer, Box<dyn Error>> {
    let backend = TestBackend::new(WIDTH, HEIGHT);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|frame| ui(frame, app))?;
    Ok(terminal.backend().buffer().clone())
}
