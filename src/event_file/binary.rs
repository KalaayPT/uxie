//! Binary event file structures matching pokeplatinum and DSPRE

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Seek, Write};

/// Binary background/spawnable event (20 bytes)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BgEventBinary {
    pub script: u16,
    pub event_type: u16,
    pub x: i32,
    pub z: i32,
    pub y: i32,
    pub player_facing_dir: u16,
}

impl BgEventBinary {
    pub fn read<R: Read>(reader: &mut R) -> io::Result<Self> {
        let script = reader.read_u16::<LittleEndian>()?;
        let event_type = reader.read_u16::<LittleEndian>()?;
        let x = reader.read_i32::<LittleEndian>()?;
        let z = reader.read_i32::<LittleEndian>()?;
        let y = reader.read_i32::<LittleEndian>()?;
        let player_facing_dir = reader.read_u16::<LittleEndian>()?;
        let mut padding = [0u8; 2];
        reader.read_exact(&mut padding)?;
        Ok(Self {
            script,
            event_type,
            x,
            z,
            y,
            player_facing_dir,
        })
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_u16::<LittleEndian>(self.script)?;
        writer.write_u16::<LittleEndian>(self.event_type)?;
        writer.write_i32::<LittleEndian>(self.x)?;
        writer.write_i32::<LittleEndian>(self.z)?;
        writer.write_i32::<LittleEndian>(self.y)?;
        writer.write_u16::<LittleEndian>(self.player_facing_dir)?;
        writer.write_all(&[0, 0])?;
        Ok(())
    }
}

/// Binary object event (32 bytes)
#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub fn read<R: Read>(reader: &mut R) -> io::Result<Self> {
        let local_id = reader.read_u16::<LittleEndian>()?;
        let graphics_id = reader.read_u16::<LittleEndian>()?;
        let movement_type = reader.read_u16::<LittleEndian>()?;
        let trainer_type = reader.read_u16::<LittleEndian>()?;
        let hidden_flag = reader.read_u16::<LittleEndian>()?;
        let script = reader.read_u16::<LittleEndian>()?;
        let dir = reader.read_i16::<LittleEndian>()?;
        let mut data = [0u16; 3];
        for i in 0..3 {
            data[i] = reader.read_u16::<LittleEndian>()?;
        }
        let movement_range_x = reader.read_i16::<LittleEndian>()?;
        let movement_range_z = reader.read_i16::<LittleEndian>()?;
        let x = reader.read_u16::<LittleEndian>()?;
        let z = reader.read_u16::<LittleEndian>()?;
        let y = reader.read_i32::<LittleEndian>()?;
        Ok(Self {
            local_id,
            graphics_id,
            movement_type,
            trainer_type,
            hidden_flag,
            script,
            dir,
            data,
            movement_range_x,
            movement_range_z,
            x,
            z,
            y,
        })
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_u16::<LittleEndian>(self.local_id)?;
        writer.write_u16::<LittleEndian>(self.graphics_id)?;
        writer.write_u16::<LittleEndian>(self.movement_type)?;
        writer.write_u16::<LittleEndian>(self.trainer_type)?;
        writer.write_u16::<LittleEndian>(self.hidden_flag)?;
        writer.write_u16::<LittleEndian>(self.script)?;
        writer.write_i16::<LittleEndian>(self.dir)?;
        for val in &self.data {
            writer.write_u16::<LittleEndian>(*val)?;
        }
        writer.write_i16::<LittleEndian>(self.movement_range_x)?;
        writer.write_i16::<LittleEndian>(self.movement_range_z)?;
        writer.write_u16::<LittleEndian>(self.x)?;
        writer.write_u16::<LittleEndian>(self.z)?;
        writer.write_i32::<LittleEndian>(self.y)?;
        Ok(())
    }
}

/// Binary warp event (12 bytes)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarpEventBinary {
    pub x: u16,
    pub z: u16,
    pub dest_header_id: u16,
    pub dest_warp_id: u16,
}

impl WarpEventBinary {
    pub fn read<R: Read>(reader: &mut R) -> io::Result<Self> {
        let x = reader.read_u16::<LittleEndian>()?;
        let z = reader.read_u16::<LittleEndian>()?;
        let dest_header_id = reader.read_u16::<LittleEndian>()?;
        let dest_warp_id = reader.read_u16::<LittleEndian>()?;
        let mut unused = [0u8; 4];
        reader.read_exact(&mut unused)?;
        Ok(Self {
            x,
            z,
            dest_header_id,
            dest_warp_id,
        })
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_u16::<LittleEndian>(self.x)?;
        writer.write_u16::<LittleEndian>(self.z)?;
        writer.write_u16::<LittleEndian>(self.dest_header_id)?;
        writer.write_u16::<LittleEndian>(self.dest_warp_id)?;
        writer.write_all(&[0; 4])?;
        Ok(())
    }
}

/// Binary coordinate event (16 bytes)
#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub fn read<R: Read>(reader: &mut R) -> io::Result<Self> {
        let script = reader.read_u16::<LittleEndian>()?;
        let x = reader.read_u16::<LittleEndian>()?;
        let z = reader.read_u16::<LittleEndian>()?;
        let width = reader.read_u16::<LittleEndian>()?;
        let length = reader.read_u16::<LittleEndian>()?;
        let y = reader.read_u16::<LittleEndian>()?;
        let value = reader.read_u16::<LittleEndian>()?;
        let var = reader.read_u16::<LittleEndian>()?;
        Ok(Self {
            script,
            x,
            z,
            width,
            length,
            y,
            value,
            var,
        })
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_u16::<LittleEndian>(self.script)?;
        writer.write_u16::<LittleEndian>(self.x)?;
        writer.write_u16::<LittleEndian>(self.z)?;
        writer.write_u16::<LittleEndian>(self.width)?;
        writer.write_u16::<LittleEndian>(self.length)?;
        writer.write_u16::<LittleEndian>(self.y)?;
        writer.write_u16::<LittleEndian>(self.value)?;
        writer.write_u16::<LittleEndian>(self.var)?;
        Ok(())
    }
}

/// Binary event file container
#[derive(Debug, Clone)]
pub struct BinaryEventFile {
    pub bg_events: Vec<BgEventBinary>,
    pub object_events: Vec<ObjectEventBinary>,
    pub warp_events: Vec<WarpEventBinary>,
    pub coord_events: Vec<CoordEventBinary>,
}

impl BinaryEventFile {
    pub fn from_binary<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        let bg_count = reader.read_u32::<LittleEndian>()?;
        let mut bg_events = Vec::with_capacity(bg_count as usize);
        for _ in 0..bg_count {
            bg_events.push(BgEventBinary::read(reader)?);
        }

        let object_count = reader.read_u32::<LittleEndian>()?;
        let mut object_events = Vec::with_capacity(object_count as usize);
        for _ in 0..object_count {
            object_events.push(ObjectEventBinary::read(reader)?);
        }

        let warp_count = reader.read_u32::<LittleEndian>()?;
        let mut warp_events = Vec::with_capacity(warp_count as usize);
        for _ in 0..warp_count {
            warp_events.push(WarpEventBinary::read(reader)?);
        }

        let coord_count = reader.read_u32::<LittleEndian>()?;
        let mut coord_events = Vec::with_capacity(coord_count as usize);
        for _ in 0..coord_count {
            coord_events.push(CoordEventBinary::read(reader)?);
        }

        Ok(Self {
            bg_events,
            object_events,
            warp_events,
            coord_events,
        })
    }

    pub fn to_binary<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_u32::<LittleEndian>(self.bg_events.len() as u32)?;
        for event in &self.bg_events {
            event.write(writer)?;
        }

        writer.write_u32::<LittleEndian>(self.object_events.len() as u32)?;
        for event in &self.object_events {
            event.write(writer)?;
        }

        writer.write_u32::<LittleEndian>(self.warp_events.len() as u32)?;
        for event in &self.warp_events {
            event.write(writer)?;
        }

        writer.write_u32::<LittleEndian>(self.coord_events.len() as u32)?;
        for event in &self.coord_events {
            event.write(writer)?;
        }

        Ok(())
    }
}
