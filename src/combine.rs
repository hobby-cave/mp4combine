use std::cmp::min;
use std::fs::File;
use std::io::{Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};

use crate::atom::{Atom, BytesAtomInput, FileAtomInput};
use crate::{
    Mp4CombineError, MAGIC_FTYP, MAGIC_MDAT, MAGIC_MFHD, MAGIC_MOOF, MAGIC_MOOV, MAGIC_TFHD,
    MAGIC_TRAF, MAGIC_TRUN,
};
use std::path::Path;

pub fn combine_mp4<InitPath, PartPath, OutPath>(
    init_path: InitPath,
    part_path: PartPath,
    out_path: OutPath,
) -> Result<()>
where
    InitPath: AsRef<Path>,
    PartPath: AsRef<Path>,
    OutPath: AsRef<Path>,
{
    let mut output = File::create(out_path)?;
    Init::open(init_path)?.write(&mut output)?;
    output.seek(SeekFrom::End(0))?;

    let mut part = Part::open(part_path)?;
    part.process(output.stream_position()?)?;
    part.write(&mut output)?;

    Ok(())
}

fn find_atom(atoms: &[Atom], magic: u32) -> Option<Atom> {
    for atom in atoms {
        if atom.magic == magic {
            return Some(*atom);
        }
    }
    None
}

fn require_atom(atoms: &[Atom], magic: u32) -> Result<Atom> {
    find_atom(atoms, magic).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            Mp4CombineError(format!(
                "missing atom {}",
                String::from_utf8(magic.to_be_bytes().to_vec()).unwrap()
            )),
        )
    })
}

fn copy<I, O>(src: &mut I, dst: &mut O, size: u32) -> Result<()>
where
    I: Read,
    O: Write,
{
    let mut tmp = [0; 4096];
    for pos in (0..size as usize).step_by(tmp.len()) {
        let size = min(tmp.len(), size as usize - pos);
        src.read_exact(&mut tmp[..size])?;
        dst.write_all(&tmp[..size])?;
    }

    Ok(())
}

fn seek_copy<I, O>(
    src: &mut I,
    src_pos: SeekFrom,
    dst: &mut O,
    dst_pos: SeekFrom,
    size: u32,
) -> Result<()>
where
    I: Read + Seek,
    O: Write + Seek,
{
    src.seek(src_pos)?;
    dst.seek(dst_pos)?;
    copy(src, dst, size)?;
    Ok(())
}

fn fix_offset(moof: Atom, mdat: Atom, mut offset: u64, output_pos: u64, base_moof: bool) -> u64 {
    if !base_moof {
        offset = offset + output_pos - moof.offset;
    }
    if mdat.offset != moof.offset + moof.size as u64 {
        offset = offset + moof.offset + moof.size as u64 - mdat.offset;
    }
    offset
}

struct Init {
    input: FileAtomInput,
    ftyp: Atom,
    moov: Atom,
}

impl Init {
    fn open<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut input = FileAtomInput::open(path)?;
        let atoms = Atom::list(&mut input)?;
        let ftyp = require_atom(&atoms, MAGIC_FTYP)?;
        let moov = require_atom(&atoms, MAGIC_MOOV)?;
        Ok(Self { input, ftyp, moov })
    }

    fn write<O>(&mut self, output: &mut O) -> Result<()>
    where
        O: Write + Seek,
    {
        seek_copy(
            self.input.as_mut(),
            SeekFrom::Start(self.ftyp.offset),
            output,
            SeekFrom::End(0),
            self.ftyp.size,
        )?;

        seek_copy(
            self.input.as_mut(),
            SeekFrom::Start(self.moov.offset),
            output,
            SeekFrom::End(0),
            self.moov.size,
        )?;

        Ok(())
    }
}

struct Part {
    input: FileAtomInput,
    moof: Atom,
    mdat: Atom,

    moof_content: Vec<u8>,
    mfhd: Atom,
    traf: Vec<Atom>,
}

impl Part {
    fn open<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut input = FileAtomInput::open(path)?;
        let atoms = Atom::list(&mut input)?;
        let moof = require_atom(&atoms, MAGIC_MOOF)?;
        let mdat = require_atom(&atoms, MAGIC_MDAT)?;

        let mut moof_content = vec![0; moof.size as usize];
        input.as_mut().seek(SeekFrom::Start(moof.offset))?;
        input.as_mut().read_exact(&mut moof_content)?;

        let atoms = Atom::list(&mut BytesAtomInput::new(&moof_content[8..]))?;
        let mfhd = require_atom(&atoms, MAGIC_MFHD)?;
        let mut traf = vec![];
        for atom in atoms {
            if atom.magic == MAGIC_TRAF {
                traf.push(atom);
            }
        }

        Ok(Self {
            input,
            moof,
            mdat,
            moof_content,
            mfhd,
            traf,
        })
    }

    fn process(&mut self, output_pos: u64) -> Result<()> {
        // seq num
        let seq_num_start = 8 + self.mfhd.offset as usize + 12;
        self.moof_content[seq_num_start..seq_num_start + 4].copy_from_slice(&[0, 0, 0, 1]);

        // each traf
        for traf in &self.traf {
            let traf_content = &mut self.moof_content
                [8 + traf.offset as usize..8 + traf.offset as usize + traf.size as usize];

            let traf_atoms = Atom::list(&mut BytesAtomInput::new(&traf_content[8..]))?;

            let tfhd = require_atom(&traf_atoms, MAGIC_TFHD)?;
            let base_is_moof = (traf_content[8 + tfhd.offset as usize + 9] & 0x02) != 0;
            let traf_has_offset = (traf_content[8 + tfhd.offset as usize + 11] & 0x01) != 0;
            if traf_has_offset {
                // fix tfhd offset
                let offset_start = 8 + tfhd.offset as usize + 12;
                let offset_end = offset_start + 8;
                let mut offset = [0; 8];
                offset.copy_from_slice(&traf_content[offset_start..offset_end]);
                let offset = u64::from_be_bytes(offset);
                let offset = fix_offset(self.moof, self.mdat, offset, output_pos, base_is_moof);
                traf_content[offset_start..offset_end].copy_from_slice(&offset.to_be_bytes());
            } else if let Some(trun) = find_atom(&traf_atoms, MAGIC_TRUN) {
                let trun_has_offset = (traf_content[8 + trun.offset as usize + 11] & 0x01) != 0;
                if trun_has_offset {
                    // fix trun offset
                    let offset_start = 8 + trun.offset as usize + 16;
                    let offset_end = offset_start + 4;
                    let mut offset = [0; 4];
                    offset.copy_from_slice(&traf_content[offset_start..offset_end]);
                    let offset = u32::from_be_bytes(offset);
                    let offset = fix_offset(
                        self.moof,
                        self.mdat,
                        offset as u64,
                        output_pos,
                        base_is_moof,
                    ) as u32;
                    traf_content[offset_start..offset_end].copy_from_slice(&offset.to_be_bytes());
                }
            }
        }

        Ok(())
    }

    fn write<O>(&mut self, output: &mut O) -> Result<()>
    where
        O: Write + Seek,
    {
        output.seek(SeekFrom::End(0))?;
        output.write_all(&self.moof_content)?;
        seek_copy(
            self.input.as_mut(),
            SeekFrom::Start(self.mdat.offset),
            output,
            SeekFrom::End(0),
            self.mdat.size,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_combine() {
        super::combine_mp4("init.mp4", "part1.m4s", "output.mp4").expect("combine failed");
    }
}
