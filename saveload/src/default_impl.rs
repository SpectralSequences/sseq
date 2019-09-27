use std::io;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use crate::{Save, Load};

impl Save for bool {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        if *self {
            buffer.write(&[1])?;
        } else {
            buffer.write(&[0])?;
        }
        Ok(())
    }
}

impl Load for bool {
    type AuxData = ();

    fn load(buffer : &mut impl Read, _ : &()) -> io::Result<Self> {
        let mut bytes : [u8; 1] = [0; 1];
        buffer.read_exact(&mut bytes)?;
        if bytes[0] == 1 {
            Ok(true)
        } else if bytes[0] == 0 {
            Ok(false)
        } else {
            panic!("Invalid encoding of boolean")
        }
    }
}

impl Save for i64 {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        let bytes = self.to_le_bytes();
        buffer.write(&bytes)?;
        Ok(())
    }
}

impl Load for i64 {
    type AuxData = ();

    fn load(buffer : &mut impl Read, _ : &()) -> io::Result<Self> {
        let mut bytes : [u8; 8] = [0; 8];
        buffer.read_exact(&mut bytes)?;
        Ok(i64::from_le_bytes(bytes))
    }
}

impl Save for i32 {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        let bytes = self.to_le_bytes();
        buffer.write(&bytes)?;
        Ok(())
    }
}

impl Load for i32 {
    type AuxData = ();

    fn load(buffer : &mut impl Read, _ : &()) -> io::Result<Self> {
        let mut bytes : [u8; 4] = [0; 4];
        buffer.read_exact(&mut bytes)?;
        Ok(i32::from_le_bytes(bytes))
    }
}

impl Save for u32 {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        let bytes = self.to_le_bytes();
        buffer.write(&bytes)?;
        Ok(())
    }
}

impl Load for u32 {
    type AuxData = ();

    fn load(buffer : &mut impl Read, _ : &()) -> io::Result<Self> {
        let mut bytes : [u8; 4] = [0; 4];
        buffer.read_exact(&mut bytes)?;
        Ok(u32::from_le_bytes(bytes))
    }
}

impl Save for u64 {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        let bytes = self.to_le_bytes();
        buffer.write(&bytes)?;
        Ok(())
    }
}

impl Load for u64 {
    type AuxData = ();

    fn load(buffer : &mut impl Read, _ : &()) -> io::Result<Self> {
        let mut bytes : [u8; 8] = [0; 8];
        buffer.read_exact(&mut bytes)?;
        Ok(u64::from_le_bytes(bytes))
    }
}

impl Save for isize {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        (*self as i64).save(buffer)
    }
}

impl Load for isize {
    type AuxData = ();

    fn load(buffer : &mut impl Read, _ : &()) -> io::Result<Self> {
        Ok(i64::load(buffer, &())? as isize)
    }
}

impl Save for usize {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        (*self as u64).save(buffer)
    }
}

impl Load for usize {
    type AuxData = ();

    fn load(buffer : &mut impl Read, _ : &()) -> io::Result<Self> {
        Ok(u64::load(buffer, &())? as usize)
    }
}

impl<T : Save> Save for Vec<T> {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        self.len().save(buffer)?;
        for x in self.iter() {
            x.save(buffer)?;
        }
        Ok(())
    }
}

impl<T : Load> Load for Vec<T> {
    type AuxData = T::AuxData;

    fn load(buffer : &mut impl Read, data : &Self::AuxData) -> io::Result<Self> {
        let len = usize::load(buffer, &())?;

        let mut result : Vec<T> = Vec::with_capacity(len);

        for _ in 0 .. len {
            result.push(T::load(buffer, data)?);
        }
        Ok(result)
    }
}

impl<T : Save> Save for Arc<T> {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        let x : &T = &*self;
        x.save(buffer)
    }
}

impl<T : Load> Load for Arc<T> {
    type AuxData = T::AuxData;

    fn load(buffer : &mut impl Read, data : &Self::AuxData) -> io::Result<Self> {
        Ok(Arc::new(T::load(buffer, data)?))
    }
}

impl<T : Save> Save for Mutex<T> {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        let x : &T = &*self.lock().unwrap();
        x.save(buffer)
    }
}

impl<T : Load> Load for Mutex<T> {
    type AuxData = T::AuxData;

    fn load(buffer : &mut impl Read, data : &Self::AuxData) -> io::Result<Self> {
        Ok(Mutex::new(T::load(buffer, data)?))
    }
}

impl<T : Save> Save for Option<T> {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        match self {
            None => false.save(buffer),
            Some(x) => {
                true.save(buffer)?;
                x.save(buffer)
            }
        }
    }
}

impl<T : Load> Load for Option<T> {
    type AuxData = Option<T::AuxData>;

    fn load(buffer : &mut impl Read, data : &Self::AuxData) -> io::Result<Self> {
        let is_some = bool::load(buffer, &())?;
        if is_some {
            Ok(Some(T::load(buffer, data.as_ref().unwrap())?))
        } else {
            Ok(None)
        }
    }
}
