use std::{
    error::Error,
    fmt::{Debug, Display, Formatter},
};

pub mod atom;
pub mod combine;

#[cfg(feature = "jni")]
pub mod jni_export;

#[cfg(feature = "clib")]
pub mod c_export;

#[macro_export]
macro_rules! magic {
    ($a:literal, $b:literal, $c:literal, $d:literal) => {
        u32::from_be_bytes([$a as u8, $b as u8, $c as u8, $d as u8])
    };
}

pub const MAGIC_FTYP: u32 = magic!('f', 't', 'y', 'p');
pub const MAGIC_MOOV: u32 = magic!('m', 'o', 'o', 'v');
pub const MAGIC_MOOF: u32 = magic!('m', 'o', 'o', 'f');
pub const MAGIC_MFHD: u32 = magic!('m', 'f', 'h', 'd');
pub const MAGIC_TRAF: u32 = magic!('t', 'r', 'a', 'f');
pub const MAGIC_TFHD: u32 = magic!('t', 'f', 'h', 'd');
pub const MAGIC_TRUN: u32 = magic!('t', 'r', 'u', 'n');
pub const MAGIC_MDAT: u32 = magic!('m', 'd', 'a', 't');

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct Mp4CombineError<D>(D);

impl<D> Display for Mp4CombineError<D>
where
    D: Display,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mp4CombineError[{}]", self.0)
    }
}

impl<D> Error for Mp4CombineError<D> where D: Debug + Display {}
