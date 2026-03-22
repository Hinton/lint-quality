//! Scan domain: pattern compilation, file discovery, and violation detection.

mod model;
mod patterns;
mod scanner;

#[cfg(test)]
pub use model::Violation;
pub use model::{FileReport, ScanResult};
pub use patterns::compile_patterns;
pub use scanner::scan_paths;
