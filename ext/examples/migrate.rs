use ext::resolution::Resolution;
use saveload::{Load, Save};
use std::fs::File;
use std::io::{BufReader, BufWriter};

fn main() -> std::io::Result<()> {
    loop {
        let complex = ext::utils::construct_s_2::<&str>("milnor", None).complex();

        let save_file: String = query::query("Save file", Ok);
        let f = File::open(&save_file).unwrap();
        let mut f = BufReader::new(f);
        let resolution = Resolution::load(&mut f, &complex)?;

        drop(f);

        let f = File::create(format!("new_{}", save_file)).unwrap();
        let mut f = BufWriter::new(f);
        resolution.save(&mut f)?;
    }
}
