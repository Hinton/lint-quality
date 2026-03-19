//! Scan domain: pattern compilation, file discovery, and violation detection.

mod model;
mod patterns;
mod scanner;

pub use model::{FileReport, ScanResult};
// Re-exported for use in tests across the crate.
#[cfg(test)]
pub use model::Violation;
pub use patterns::compile_patterns;
pub use scanner::scan_paths;
