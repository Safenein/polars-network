use pyo3::prelude::*;

pub mod expressions;

/// A Polars plugin for network-related computations implemented in Rust.
#[pymodule]
fn polars_network(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register submodules
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    expressions::register(m)?;

    Ok(())
}
