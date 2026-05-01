//! Bindings for charting an `Sseq` to SVG.
//!
//! We only expose the SVG backend, which is sufficient for the `chart`
//! example. Because `sseq::Product<2>` is not `Clone`, we cannot accept a
//! Python list of `Product` objects directly. Instead, the user passes the
//! source `Resolution`, and we compute the products internally.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use ext::chain_complex::{ChainComplex, FreeChainComplex};
use pyo3::prelude::*;
use sseq::charting::SvgBackend;

use crate::resolution::Resolution;

/// A `Write` adapter that appends to a shared `Arc<Mutex<Vec<u8>>>`.
struct SharedBuf(Arc<Mutex<Vec<u8>>>);

impl Write for SharedBuf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Compute the spectral sequence of `resolution` and write its E_r page as an
/// SVG. If `with_filtration_one_products` is true, also draw structure lines
/// for the algebra's default filtration-one products (`h_0, h_1, ...`).
///
/// If `path` is `None`, return the SVG as a `bytes`. Otherwise write to that
/// file and return `None`.
#[pyfunction]
#[pyo3(signature = (resolution, r=2, differentials=false, with_filtration_one_products=true, path=None))]
pub fn write_sseq_svg(
    resolution: &Resolution,
    r: i32,
    differentials: bool,
    with_filtration_one_products: bool,
    path: Option<PathBuf>,
) -> PyResult<Option<Vec<u8>>> {
    use algebra::Algebra;

    let res = resolution.arc();

    let sseq = res.to_sseq();

    let products: Vec<(String, sseq::Product<2>)> = if with_filtration_one_products {
        res.algebra()
            .default_filtration_one_products()
            .into_iter()
            .map(|(name, op_deg, op_idx)| {
                (name, res.filtration_one_products(op_deg, op_idx))
            })
            .collect()
    } else {
        Vec::new()
    };

    if let Some(path) = path {
        let f = BufWriter::new(File::create(&path).map_err(io_to_py)?);
        let backend = SvgBackend::new(f);
        sseq.write_to_graph(backend, r, differentials, products.iter(), |_| Ok(()))
            .map_err(io_to_py)?;
        Ok(None)
    } else {
        let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
        let backend = SvgBackend::new(SharedBuf(Arc::clone(&buf)));
        sseq.write_to_graph(backend, r, differentials, products.iter(), |_| Ok(()))
            .map_err(io_to_py)?;
        // `backend` goes out of scope here, which writes the trailing
        // `</svg>` to our shared buffer.
        let bytes = std::mem::take(&mut *buf.lock().unwrap());
        Ok(Some(bytes))
    }
}

fn io_to_py(e: io::Error) -> PyErr {
    pyo3::exceptions::PyIOError::new_err(e.to_string())
}
