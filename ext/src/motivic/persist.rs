//! Disk cache for the Phase 2 lift: the motivic weights and the lifted $A_C$
//! differentials, keyed by the `(max, compute)` box so a stale cache is never
//! loaded. Written alongside the mod-$\tau$ resolution's own save directory (see
//! [`MotivicResolution::with_module`]).

use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
    sync::Arc,
};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use super::{Gen, MotivicResolution};

impl MotivicResolution {
    /// Path of the weights + lifted-differential cache within `save_dir`.
    fn lift_cache_path(save_dir: &Option<PathBuf>) -> Option<PathBuf> {
        save_dir.as_ref().map(|d| d.join("motivic-lift.bin"))
    }

    /// Save the weights and lifted differentials to `save_dir/motivic-lift.bin`,
    /// tagged with the compute box so a stale cache is never loaded. No-op if
    /// `save_dir` is `None`.
    pub(super) fn save_lift(&self, save_dir: &Option<PathBuf>) {
        let Some(path) = Self::lift_cache_path(save_dir) else {
            return;
        };
        let Ok(file) = std::fs::File::create(&path) else {
            return;
        };
        let mut w = std::io::BufWriter::new(file);
        let write = |w: &mut std::io::BufWriter<std::fs::File>| -> std::io::Result<()> {
            // Header: magic + the (max, compute) box this lift was computed for.
            w.write_u32::<LittleEndian>(0x004D_0004)?;
            for v in [
                self.max.n(),
                self.max.s(),
                self.compute.n(),
                self.compute.s(),
            ] {
                w.write_i32::<LittleEndian>(v)?;
            }
            // Weights: (s, t, idx, weight).
            w.write_u64::<LittleEndian>(self.weights.len() as u64)?;
            for (g, &wt) in self.weights.iter() {
                w.write_i32::<LittleEndian>(g.s)?;
                w.write_i32::<LittleEndian>(g.t)?;
                w.write_u64::<LittleEndian>(g.idx as u64)?;
                w.write_i32::<LittleEndian>(wt)?;
            }
            // Lifted differentials: (s, t, idx, len, [indices]).
            w.write_u64::<LittleEndian>(self.lifted.len() as u64)?;
            for (g, support) in &self.lifted {
                w.write_i32::<LittleEndian>(g.s)?;
                w.write_i32::<LittleEndian>(g.t)?;
                w.write_u64::<LittleEndian>(g.idx as u64)?;
                w.write_u64::<LittleEndian>(support.len() as u64)?;
                for &b in support {
                    w.write_u64::<LittleEndian>(b as u64)?;
                }
            }
            Ok(())
        };
        let _ = write(&mut w);
    }

    /// Load the weights and lifted differentials from the cache, returning whether a
    /// valid cache for this exact `(max, compute)` box was found and read.
    pub(super) fn load_lift(&mut self, save_dir: &Option<PathBuf>) -> bool {
        let Some(path) = Self::lift_cache_path(save_dir) else {
            return false;
        };
        let Ok(file) = std::fs::File::open(&path) else {
            return false;
        };
        let mut r = std::io::BufReader::new(file);
        let read = |r: &mut std::io::BufReader<std::fs::File>| -> std::io::Result<
            Option<(HashMap<Gen, i32>, HashMap<Gen, BTreeSet<usize>>)>,
        > {
            if r.read_u32::<LittleEndian>()? != 0x004D_0004 {
                return Ok(None);
            }
            let hdr = [
                r.read_i32::<LittleEndian>()?,
                r.read_i32::<LittleEndian>()?,
                r.read_i32::<LittleEndian>()?,
                r.read_i32::<LittleEndian>()?,
            ];
            if hdr
                != [
                    self.max.n(),
                    self.max.s(),
                    self.compute.n(),
                    self.compute.s(),
                ]
            {
                return Ok(None); // cache is for a different box
            }
            let mut weights = HashMap::new();
            for _ in 0..r.read_u64::<LittleEndian>()? {
                let g = Gen {
                    s: r.read_i32::<LittleEndian>()?,
                    t: r.read_i32::<LittleEndian>()?,
                    idx: r.read_u64::<LittleEndian>()? as usize,
                };
                weights.insert(g, r.read_i32::<LittleEndian>()?);
            }
            let mut lifted = HashMap::new();
            for _ in 0..r.read_u64::<LittleEndian>()? {
                let g = Gen {
                    s: r.read_i32::<LittleEndian>()?,
                    t: r.read_i32::<LittleEndian>()?,
                    idx: r.read_u64::<LittleEndian>()? as usize,
                };
                let len = r.read_u64::<LittleEndian>()?;
                let mut support = BTreeSet::new();
                for _ in 0..len {
                    support.insert(r.read_u64::<LittleEndian>()? as usize);
                }
                lifted.insert(g, support);
            }
            Ok(Some((weights, lifted)))
        };
        match read(&mut r) {
            Ok(Some((weights, lifted))) => {
                self.weights = Arc::new(weights);
                self.lifted = lifted;
                true
            }
            _ => false,
        }
    }
}
