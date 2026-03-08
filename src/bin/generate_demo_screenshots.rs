use std::{error::Error, path::Path};

use nm_wifi::{
    backend::default_backend,
    demo_screenshots::write_demo_svgs_with_backend,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let output_dir = Path::new("docs/screenshots");
    let backend = default_backend();
    write_demo_svgs_with_backend(output_dir, backend.as_ref()).await?;
    println!("Generated demo screenshots in {}", output_dir.display());
    Ok(())
}
