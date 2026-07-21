use std::path::Path;

use super::model::ProfileReport;

pub fn write_profile_report(path: &Path, report: &ProfileReport) -> Result<(), std::io::Error> {
    let bytes = serde_json::to_vec_pretty(report).expect("profile report serializes");
    std::fs::write(path, bytes)
}
