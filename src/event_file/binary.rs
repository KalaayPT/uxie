//! Binary event file structures matching pokeplatinum and DSPRE

use binrw::{BinRead, BinWrite};
use std::io::{self, Read, Seek, Write};

/// Binary background/spawnable event (20 bytes)
#[derive(Debug, Clone, PartialEq, Eq, BinRead, BinWrite)]
#[brw(little)]
pub struct BgEventBinary {
    pub script: u16,
    pub event_type: u16,
    pub x: i32,
    pub z: i32,
    pub y: i32,
    #[brw(pad_after = 2)]
    pub player_facing_dir: u16,
}

impl BgEventBinary {
    pub fn read<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        Self::read_le(reader).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn write<W: Write + Seek>(&self, writer: &mut W) -> io::Result<()> {
        self.write_le(writer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

/// Binary object event (32 bytes)
#[derive(Debug, Clone, PartialEq, Eq, BinRead, BinWrite)]
#[brw(little)]
pub struct ObjectEventBinary {
    pub local_id: u16,
    pub graphics_id: u16,
    pub movement_type: u16,
    pub trainer_type: u16,
    pub hidden_flag: u16,
    pub script: u16,
    pub dir: i16,
    pub data: [u16; 3],
    pub movement_range_x: i16,
    pub movement_range_z: i16,
    pub x: u16,
    pub z: u16,
    pub y: i32,
}

impl ObjectEventBinary {
    pub fn read<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        Self::read_le(reader).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn write<W: Write + Seek>(&self, writer: &mut W) -> io::Result<()> {
        self.write_le(writer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

/// Binary warp event (12 bytes)
#[derive(Debug, Clone, PartialEq, Eq, BinRead, BinWrite)]
#[brw(little)]
pub struct WarpEventBinary {
    pub x: u16,
    pub z: u16,
    pub dest_header_id: u16,
    #[brw(pad_after = 4)]
    pub dest_warp_id: u16,
}

impl WarpEventBinary {
    pub fn read<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        Self::read_le(reader).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn write<W: Write + Seek>(&self, writer: &mut W) -> io::Result<()> {
        self.write_le(writer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

/// Binary coordinate event (16 bytes)
#[derive(Debug, Clone, PartialEq, Eq, BinRead, BinWrite)]
#[brw(little)]
pub struct CoordEventBinary {
    pub script: u16,
    pub x: u16,
    pub z: u16,
    pub width: u16,
    pub length: u16,
    pub y: u16,
    pub value: u16,
    pub var: u16,
}

impl CoordEventBinary {
    pub fn read<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        Self::read_le(reader).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn write<W: Write + Seek>(&self, writer: &mut W) -> io::Result<()> {
        self.write_le(writer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

/// Binary event file container
#[binrw::binrw]
#[brw(little)]
#[derive(Debug, Clone)]
pub struct BinaryEventFile {
    #[br(temp)]
    #[bw(calc = bg_events.len() as u32)]
    bg_count: u32,
    #[br(count = bg_count)]
    pub bg_events: Vec<BgEventBinary>,

    #[br(temp)]
    #[bw(calc = object_events.len() as u32)]
    object_count: u32,
    #[br(count = object_count)]
    pub object_events: Vec<ObjectEventBinary>,

    #[br(temp)]
    #[bw(calc = warp_events.len() as u32)]
    warp_count: u32,
    #[br(count = warp_count)]
    pub warp_events: Vec<WarpEventBinary>,

    #[br(temp)]
    #[bw(calc = coord_events.len() as u32)]
    coord_count: u32,
    #[br(count = coord_count)]
    pub coord_events: Vec<CoordEventBinary>,
}

impl BinaryEventFile {
    pub fn from_binary<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        Self::read_le(reader).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn to_binary<W: Write + Seek>(&self, writer: &mut W) -> io::Result<()> {
        self.write_le(writer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}
