use etas_std::migration::StdMigrationBaseline;
use std::{error::Error, fs, io};

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["export", path] => {
            let baseline = StdMigrationBaseline::from_registry(&etas_std::standard_registry());
            serde_json::to_writer_pretty(fs::File::create(path)?, &baseline)?;
        }
        ["compare", expected, actual] => {
            let expected: StdMigrationBaseline = serde_json::from_slice(&fs::read(expected)?)?;
            let actual: StdMigrationBaseline = serde_json::from_slice(&fs::read(actual)?)?;
            let report = expected.compare(&actual);
            serde_json::to_writer_pretty(io::stdout().lock(), &report)?;
            if !report.matches() {
                std::process::exit(1);
            }
        }
        _ => {
            return Err(
                "usage: std_migration_baseline export <file> | compare <baseline> <candidate>"
                    .into(),
            );
        }
    }
    Ok(())
}
