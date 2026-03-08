use std::{error::Error, path::Path};

use nm_wifi::{demo_screenshots::write_demo_svgs, network::scan_wifi_networks};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let output_dir = Path::new("docs/screenshots");
    let networks = scan_wifi_networks().await?;
    write_demo_svgs(output_dir, &networks)?;
    println!("Generated demo screenshots in {}", output_dir.display());
    Ok(())
}
