use std::io;
use std::io::{Read, Write};

mod default_impl;

pub trait Save {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()>;
}

pub trait Load : Sized {
    type AuxData;

    fn load(buffer : &mut impl Read, data : &Self::AuxData) -> io::Result<Self>;
}
