use std::{error::Error, fs, path::Path};

use super::{
    render::render_app,
    scenarios::demo_shot_apps,
    svg::buffer_to_svg,
};
use crate::{backend::NetworkBackend, wifi::WifiNetwork};

pub fn write_demo_svgs(
    output_dir: &Path,
    networks: &[WifiNetwork],
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(output_dir)?;

    for (file_name, app) in demo_shot_apps(networks) {
        let buffer = render_app(&app)?;
        let svg = buffer_to_svg(&buffer);
        fs::write(output_dir.join(file_name), svg)?;
    }

    Ok(())
}

pub async fn write_demo_svgs_with_backend(
    output_dir: &Path,
    backend: &dyn NetworkBackend,
) -> Result<(), Box<dyn Error>> {
    let networks = backend.scan_networks().await?;
    write_demo_svgs(output_dir, &networks)
}
