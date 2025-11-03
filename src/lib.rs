use pyo3::prelude::*;

pub mod expressions;

/// A Polars plugin for network-related computations implemented in Rust.
///
/// This module provides high-performance network analysis utilities including:
/// - Graph construction and manipulation
/// - Network algorithms (centrality, pathfinding, etc.)
/// - Network metrics and statistics
#[pymodule]
fn polars_network(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register submodules
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    expressions::register(m)?;

    Ok(())
}
