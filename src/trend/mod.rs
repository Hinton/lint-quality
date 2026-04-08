mod export;
pub mod loader;
mod server;

use anyhow::Result;
use std::path::PathBuf;

pub fn run(
    paths: Vec<PathBuf>,
    port: u16,
    no_open: bool,
    export_dir: Option<PathBuf>,
) -> Result<()> {
    let reports = loader::load_reports(&paths)?;
    let count = reports.len();
    let oldest = &reports.first().unwrap().metadata.timestamp;
    let newest = &reports.last().unwrap().metadata.timestamp;
    eprintln!(
        "loaded {} report(s) ({} to {})",
        count,
        oldest.format("%Y-%m-%d"),
        newest.format("%Y-%m-%d"),
    );

    if let Some(out_dir) = export_dir {
        export::export(&reports, &out_dir)?;
    } else {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(server::serve(reports, port, no_open))?;
    }
    Ok(())
}
