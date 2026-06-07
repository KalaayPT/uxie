use crate::game::GameFamily;
use binrw::{BinRead, BinWrite};
use std::io::{self, Read, Seek, Write};

/// Background/spawnable event (20 bytes).
/// The trailing 2 bytes after `player_facing_dir` are implicit C struct padding and always zero.
#[derive(Debug, Clone, PartialEq, Eq, BinRead, BinWrite)]
#[brw(little)]
pub struct BgEvent {
    pub script: u16,
    pub event_type: u16,
    pub x: i32,
    pub z: i32,
    pub y: i32,
    #[brw(pad_after = 2)]
    pub player_facing_dir: u16,
}

/// Object/overworld event (32 bytes).
#[derive(Debug, Clone, PartialEq, Eq, BinRead, BinWrite)]
#[brw(little)]
pub struct ObjectEvent {
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

/// Warp event (12 bytes).
/// `height` is always 0 for Platinum/DP. HGSS stores height here.
#[derive(Debug, Clone, PartialEq, Eq, BinRead, BinWrite)]
#[brw(little)]
pub struct WarpEvent {
    pub x: u16,
    pub z: u16,
    pub dest_header_id: u16,
    pub dest_warp_id: u16,
    pub height: u32,
}

/// Coordinate/trigger event (16 bytes).
/// `x` and `z` are signed per the HGSS struct definition.
#[derive(Debug, Clone, PartialEq, Eq, BinRead, BinWrite)]
#[brw(little)]
pub struct CoordEvent {
    pub script: u16,
    pub x: i16,
    pub z: i16,
    pub width: u16,
    pub length: u16,
    pub y: u16,
    pub value: u16,
    pub var: u16,
}

/// Event file container — the single in-memory representation for all formats.
#[binrw::binrw]
#[brw(little)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventFile {
    #[br(temp)]
    #[bw(calc = bg_events.len() as u32)]
    bg_count: u32,
    #[br(count = bg_count)]
    pub bg_events: Vec<BgEvent>,

    #[br(temp)]
    #[bw(calc = object_events.len() as u32)]
    object_count: u32,
    #[br(count = object_count)]
    pub object_events: Vec<ObjectEvent>,

    #[br(temp)]
    #[bw(calc = warp_events.len() as u32)]
    warp_count: u32,
    #[br(count = warp_count)]
    pub warp_events: Vec<WarpEvent>,

    #[br(temp)]
    #[bw(calc = coord_events.len() as u32)]
    coord_count: u32,
    #[br(count = coord_count)]
    pub coord_events: Vec<CoordEvent>,
}

impl EventFile {
    pub fn from_binary<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        Self::read_le(reader).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn to_binary<W: Write + Seek>(&self, writer: &mut W) -> io::Result<()> {
        self.write_le(writer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Write as binary, validating family-specific constraints.
    /// For Platinum/DP, warp `height` must be zero.
    pub fn to_binary_for<W: Write + Seek>(
        &self,
        writer: &mut W,
        family: GameFamily,
    ) -> io::Result<()> {
        if family != GameFamily::HGSS {
            for warp in &self.warp_events {
                if warp.height != 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "warp height must be 0 for {:?}, got {}",
                            family, warp.height
                        ),
                    ));
                }
            }
        }
        self.to_binary(writer)
    }
}
