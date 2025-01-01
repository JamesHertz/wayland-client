use std::{fs, str, slice::Iter};

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Error {
    InvalidBufferOffset { offset: usize, buffer_len: usize },
    InvalidMagicNumber,
    MissingField(&'static str),
    InvalidUtf8(std::str::Utf8Error),
    IoError(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(value: std::str::Utf8Error) -> Self {
        Self::InvalidUtf8(value)
    }
}

impl From<&'static str> for Error {
    fn from(value: &'static str) -> Self {
        Self::MissingField(value)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}
type Result<T> = std::result::Result<T, Error>;
//
struct BinaryParser {
    buffer: Vec<u8>,
    offset: usize,
}

impl BinaryParser {
    fn new(buffer: Vec<u8>) -> Self {
        Self { buffer, offset: 0 }
    }

    fn set_offset(&mut self, offset: usize) -> Result<()> {
        if offset > self.buffer.len() {
            Err(Error::InvalidBufferOffset {
                offset,
                buffer_len: self.buffer.len(),
            })
        } else {
            self.offset = offset;
            Ok(())
        }
    }

    fn read_u32(&mut self) -> Option<u32> {
        let bytes = self.advance_by(4)?;
        Some(u32::from_be_bytes(bytes.try_into().unwrap()))
    }

    fn read_u16(&mut self) -> Option<u16> {
        let bytes = self.advance_by(2)?;
        Some(u16::from_be_bytes(bytes.try_into().unwrap()))
    }

    fn skip(&mut self, n: usize) -> Option<()> {
        if n > self.buffer.len() - self.offset {
            None
        } else {
            self.offset += n;
            Some(())
        }
    }

    fn advance_by(&mut self, n: usize) -> Option<&[u8]> {
        let start = self.offset;
        self.skip(n)?;
        Some(&self.buffer[start..self.offset])
    }
}

macro_rules! read_field {
    ($v : expr, $msg : tt) => {
        $v.ok_or($msg)
    };
}

#[derive(Debug, Clone)]
pub struct DirectoryTableEntry {
    tag: String,
    checksum: u32,
    offset: u32,
    length: u32,
}

pub struct TrueTypeFont {
    parser: BinaryParser,
    directory: Vec<DirectoryTableEntry>,
}

#[derive(Debug)]
struct CmapSubtable {
    platform_id: u16,
    platform_specific_id: u16,
    offset: u32,
}

impl TrueTypeFont {
    const TTF_MAGIC_NR: [u32; 2] = [0x1000, 0x74727565];
    const UNICODE_PLATFORM_ID: u16 = 0;

    pub fn from_file(filename: &str) -> Result<Self> {
        let mut parser = BinaryParser::new(fs::read(filename)?);

        let scalar_type = read_field!(parser.read_u32(), "scalar type")?;
        if Self::TTF_MAGIC_NR.contains(&scalar_type) {
            return Err(Error::InvalidMagicNumber);
        }

        let num_tables = read_field!(parser.read_u16(), "numTables")?;
        dbg!(num_tables);

        read_field!(parser.skip(3 * 2), "searchRange, entrySelector and rangeShift")?;

        let mut directory = Vec::new();
        for i in 0..num_tables {
            let tag = read_field!(parser.read_u32(), "tag")?;
            let tag = str::from_utf8(&tag.to_be_bytes())?.to_string();

            let checksum = read_field!(parser.read_u32(), "checksum")?;
            let offset = read_field!(parser.read_u32(), "offset")?;
            let length = read_field!(parser.read_u32(), "length")?;

            directory.push(DirectoryTableEntry {
                tag,
                checksum,
                offset,
                length,
            });
        }

        Ok(Self { parser, directory })
    }

    fn find_dir_entry(&self, tag: &str) -> Option<DirectoryTableEntry> {
        for entry in self.directory.iter() {
            if entry.tag == tag {
                return Some(entry.clone());
            }
        }
        None
    }

    pub fn load_glyphs(&mut self) -> Result<()> {
        let cmap = self.find_dir_entry("cmap").unwrap();

        let parser = &mut self.parser;
        parser.set_offset(cmap.offset as usize)?;

        let version = read_field!(parser.read_u16(), "cmap.version")?;
        let subtable_nr = read_field!(parser.read_u16(), "cmap.subtable_nr")?;

        dbg!((version, subtable_nr));

        let mut subtables = Vec::new();
        for _ in 0..subtable_nr {
            let platform_id = read_field!(parser.read_u16(), "cmap.platform_id")?;
            if platform_id != Self::UNICODE_PLATFORM_ID {
                read_field!(parser.skip(2 + 4), "cmap.platform_specific_id and cmap.offset")?;
            } else {
                let platform_specific_id = read_field!(parser.read_u16(), "cmap.platform_specific_id")?;
                let offset = read_field!(parser.read_u32(), "cmap.offset")?;
                subtables.push(CmapSubtable {
                    platform_id,
                    platform_specific_id,
                    offset,
                })
            }
        }

        dbg!(&subtables);
        assert!(!subtables.is_empty(), "No subtables for Unicode found (todo: support other codes)");
        parser.set_offset((cmap.offset + subtables[0].offset) as usize)?;

        let format = read_field!(parser.read_u16(), "cmap.subtable.format")?;
        dbg!(format);

        Ok(())
    }

    pub fn iter_table_entries(&self) -> Iter<'_, DirectoryTableEntry> {
        self.directory.iter()
    }
}
