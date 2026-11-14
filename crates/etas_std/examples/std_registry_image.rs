use std::{error::Error, path::Path};

use etas_std::StdRegistryImage;

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let [operation, path] = arguments.as_slice() else {
        return Err("usage: std_registry_image <export-native|read> <path>".into());
    };
    match operation.as_str() {
        "export-native" => {
            let image =
                StdRegistryImage::capture(&etas_std::standard_registry().native_bootstrap())?;
            std::fs::write(path, serde_json::to_vec(&image)?)?;
        }
        "read" => {
            // This process builds its query view only from the image. It never
            // constructs the handwritten registry or fills absent declarations.
            let image: StdRegistryImage = serde_json::from_slice(&std::fs::read(Path::new(path))?)?;
            let registry = image.into_registry()?;
            println!(
                "Loaded {} modules, {} declarations, {} prelude bindings",
                registry.modules().count(),
                registry.symbols().count(),
                registry.prelude().symbols().count()
            );
        }
        _ => return Err("expected export-native or read".into()),
    }
    Ok(())
}
