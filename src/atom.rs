use std::{
    fs::File,
    io::{ErrorKind, Read, Result, Seek, SeekFrom},
    path::Path,
};

#[derive(Copy, Clone)]
pub struct Atom {
    pub offset: u64,
    pub size: u32,
    pub magic: u32,
}

impl Atom {
    pub fn list<I>(input: &mut I) -> Result<Vec<Atom>>
    where
        I: AtomInput,
    {
        let mut atoms = vec![];
        let mut offset = 0;
        while offset < input.len() {
            let size = input.read_u32(offset)?;
            if size < 12 || offset + size as u64 > input.len() {
                return Err(ErrorKind::UnexpectedEof.into());
            }
            let magic = input.read_u32(offset + 4)?;
            atoms.push(Atom {
                offset,
                size,
                magic,
            });
            offset += size as u64;
        }

        // return
        Ok(atoms)
    }

    #[inline]
    pub fn content_offset(&self) -> u64 {
        self.offset + 8
    }

    #[inline]
    pub fn content_size(&self) -> u32 {
        self.size - 8
    }
}

pub trait AtomInput {
    fn len(&mut self) -> u64;
    fn read(&mut self, pos: u64, buf: &mut [u8]) -> Result<()>;

    fn read_u32(&mut self, pos: u64) -> Result<u32> {
        let mut buf = [0; 4];
        self.read(pos, &mut buf)?;
        Ok(u32::from_be_bytes(buf))
    }
}

pub struct FileAtomInput {
    file: File,
    size: u64,
}

impl FileAtomInput {
    #[inline]
    pub fn open<P>(path: P) -> Result<FileAtomInput>
    where
        P: AsRef<Path>,
    {
        let file = File::open(path)?;
        let size = file.metadata()?.len();
        Ok(Self { file, size })
    }
}

impl AsRef<File> for FileAtomInput {
    #[inline]
    fn as_ref(&self) -> &File {
        &self.file
    }
}

impl AsMut<File> for FileAtomInput {
    #[inline]
    fn as_mut(&mut self) -> &mut File {
        &mut self.file
    }
}

impl AtomInput for FileAtomInput {
    fn len(&mut self) -> u64 {
        self.size
    }

    fn read(&mut self, pos: u64, buf: &mut [u8]) -> Result<()> {
        self.file.seek(SeekFrom::Start(pos))?;
        self.file.read_exact(buf)?;
        Ok(())
    }
}

pub struct BytesAtomInput<'a> {
    bytes: &'a [u8],
}

impl<'a> BytesAtomInput<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }
}

impl<'a> AtomInput for BytesAtomInput<'a> {
    fn len(&mut self) -> u64 {
        self.bytes.len() as u64
    }

    fn read(&mut self, pos: u64, buf: &mut [u8]) -> Result<()> {
        let pos = pos as usize;
        buf.copy_from_slice(&self.bytes[pos..pos + buf.len()]);
        Ok(())
    }
}

pub struct SubAtomInput<'a, A> {
    parent: &'a mut A,
    offset: u64,
    size: u64,
}

impl<'a, A> SubAtomInput<'a, A>
where
    A: AtomInput,
{
    pub fn new(atom: &'a mut A, offset: u64, size: u64) -> Self {
        assert!(offset + size < atom.len());
        Self {
            parent: atom,
            offset,
            size,
        }
    }
}

impl<'a, A> AtomInput for SubAtomInput<'a, A>
where
    A: AtomInput,
{
    fn len(&mut self) -> u64 {
        self.size
    }

    #[inline]
    fn read(&mut self, pos: u64, buf: &mut [u8]) -> Result<()> {
        self.parent.read(pos + self.offset, buf)
    }
}
