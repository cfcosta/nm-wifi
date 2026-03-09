use std::{error::Error, fs, io, path::Path};

use super::{
    render::render_app,
    scenarios::demo_shot_apps,
    svg::buffer_to_svg,
};
use crate::{backend::NetworkBackend, wifi::WifiNetwork};

fn validate_demo_screenshot_networks(
    networks: &[WifiNetwork],
) -> Result<(), Box<dyn Error>> {
    if networks.is_empty() {
        return Err(io::Error::other(
            "demo screenshots require at least one network",
        )
        .into());
    }

    if !networks.iter().any(|network| network.connected) {
        return Err(io::Error::other(
            "demo screenshots require at least one connected network",
        )
        .into());
    }

    if !networks.iter().any(|network| !network.connected) {
        return Err(io::Error::other(
            "demo screenshots require at least one unconnected network",
        )
        .into());
    }

    if !networks
        .iter()
        .any(|network| network.is_secured() && !network.connected)
    {
        return Err(io::Error::other(
            "demo screenshots require at least one secured unconnected network",
        )
        .into());
    }

    Ok(())
}

pub fn write_demo_svgs(
    output_dir: &Path,
    networks: &[WifiNetwork],
) -> Result<(), Box<dyn Error>> {
    validate_demo_screenshot_networks(networks)?;
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
