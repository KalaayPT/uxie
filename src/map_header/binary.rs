use super::types::*;
use crate::game::GameFamily;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{self, Read, Seek, SeekFrom};

pub fn read_map_header_from_bytes(data: &[u8], family: GameFamily) -> io::Result<MapHeader> {
    if data.len() < MAP_HEADER_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Buffer too small: {} bytes, need {}",
                data.len(),
                MAP_HEADER_SIZE
            ),
        ));
    }

    let mut cursor = io::Cursor::new(data);
    match family {
        GameFamily::DP => Ok(MapHeader::DP(read_dp_header(&mut cursor)?)),
        GameFamily::Platinum => Ok(MapHeader::Pt(read_pt_header(&mut cursor)?)),
        GameFamily::HGSS => Ok(MapHeader::HGSS(read_hgss_header(&mut cursor)?)),
    }
}

pub fn read_map_headers_from_arm9<R: Read + Seek>(
    reader: &mut R,
    header_table_offset: u64,
    count: usize,
    family: GameFamily,
) -> io::Result<Vec<MapHeader>> {
    let mut headers = Vec::with_capacity(count);
    reader.seek(SeekFrom::Start(header_table_offset))?;

    for _ in 0..count {
        let mut buf = [0u8; MAP_HEADER_SIZE];
        reader.read_exact(&mut buf)?;
        headers.push(read_map_header_from_bytes(&buf, family)?);
    }

    Ok(headers)
}

fn read_dp_header<R: Read>(reader: &mut R) -> io::Result<MapHeaderDP> {
    let mut h = MapHeaderDP::default();
    h.area_data_id = reader.read_u8()?;
    h.unknown1 = reader.read_u8()?;
    h.matrix_id = reader.read_u16::<LittleEndian>()?;
    h.script_file_id = reader.read_u16::<LittleEndian>()?;
    h.level_script_id = reader.read_u16::<LittleEndian>()?;
    h.text_archive_id = reader.read_u16::<LittleEndian>()?;
    h.music_day_id = reader.read_u16::<LittleEndian>()?;
    h.music_night_id = reader.read_u16::<LittleEndian>()?;
    h.wild_pokemon = reader.read_u16::<LittleEndian>()?;
    h.event_file_id = reader.read_u16::<LittleEndian>()?;
    h.location_name = reader.read_u16::<LittleEndian>()?;
    h.weather_id = reader.read_u8()?;
    h.camera_angle_id = reader.read_u8()?;
    h.location_specifier = reader.read_u8()?;

    let map_settings = reader.read_u8()?;
    h.battle_background = map_settings & 0b_1111;
    h.flags = (map_settings >> 4) & 0b_1111;

    Ok(h)
}

fn read_pt_header<R: Read>(reader: &mut R) -> io::Result<MapHeaderPt> {
    let mut h = MapHeaderPt::default();
    h.area_data_id = reader.read_u8()?;
    h.unknown1 = reader.read_u8()?;
    h.matrix_id = reader.read_u16::<LittleEndian>()?;
    h.script_file_id = reader.read_u16::<LittleEndian>()?;
    h.level_script_id = reader.read_u16::<LittleEndian>()?;
    h.text_archive_id = reader.read_u16::<LittleEndian>()?;
    h.music_day_id = reader.read_u16::<LittleEndian>()?;
    h.music_night_id = reader.read_u16::<LittleEndian>()?;
    h.wild_pokemon = reader.read_u16::<LittleEndian>()?;
    h.event_file_id = reader.read_u16::<LittleEndian>()?;
    h.location_name = reader.read_u8()?;
    h.area_icon = reader.read_u8()?;
    h.weather_id = reader.read_u8()?;
    h.camera_angle_id = reader.read_u8()?;

    // Bitfield packing: mapType:7 | battleBG:5 | flags:4
    let map_settings = reader.read_u16::<LittleEndian>()?;
    h.location_specifier = (map_settings & 0b_0111_1111) as u8;
    h.battle_background = ((map_settings >> 7) & 0b_1_1111) as u8;
    h.flags = ((map_settings >> 12) & 0b_1111) as u8;

    Ok(h)
}

fn read_hgss_header<R: Read>(reader: &mut R) -> io::Result<MapHeaderHGSS> {
    let mut h = MapHeaderHGSS::default();
    h.wild_pokemon = reader.read_u8()?;
    h.area_data_id = reader.read_u8()?;

    // Bitfield: unknown0:4 | worldmapX:6 | worldmapY:6
    let coords = reader.read_u16::<LittleEndian>()?;
    h.unknown0 = (coords & 0b_1111) as u8;
    h.worldmap_x = ((coords >> 4) & 0b_11_1111) as u8;
    h.worldmap_y = ((coords >> 10) & 0b_11_1111) as u8;

    h.matrix_id = reader.read_u16::<LittleEndian>()?;
    h.script_file_id = reader.read_u16::<LittleEndian>()?;
    h.level_script_id = reader.read_u16::<LittleEndian>()?;
    h.text_archive_id = reader.read_u16::<LittleEndian>()?;
    h.music_day_id = reader.read_u16::<LittleEndian>()?;
    h.music_night_id = reader.read_u16::<LittleEndian>()?;
    h.event_file_id = reader.read_u16::<LittleEndian>()?;
    h.location_name = reader.read_u8()?;

    // Bitfield: areaIcon:4 | unknown1:4
    let area_props = reader.read_u8()?;
    h.area_icon = area_props & 0b_1111;
    h.unknown1 = (area_props >> 4) & 0b_1111;

    // Last 4 bytes: complex bitfield
    // kantoFlag:1 | weatherID:7 | locationType:4 | cameraAngleID:6 | followMode:2 | battleBG:5 | flags:7
    let last32 = reader.read_u32::<LittleEndian>()?;
    h.kanto_flag = (last32 & 0b_1) == 1;
    h.weather_id = ((last32 >> 1) & 0b_111_1111) as u8;
    h.location_type = ((last32 >> 8) & 0b_1111) as u8;
    h.camera_angle_id = ((last32 >> 12) & 0b_11_1111) as u8;
    h.follow_mode = ((last32 >> 18) & 0b_11) as u8;
    h.battle_background = ((last32 >> 20) & 0b_1_1111) as u8;
    h.flags = ((last32 >> 25) & 0b_111_1111) as u8;

    Ok(h)
}

pub fn write_map_header_to_bytes(header: &MapHeader) -> Vec<u8> {
    use byteorder::WriteBytesExt;
    let mut buf = Vec::with_capacity(MAP_HEADER_SIZE);

    match header {
        MapHeader::DP(h) => {
            buf.write_u8(h.area_data_id).unwrap();
            buf.write_u8(h.unknown1).unwrap();
            buf.write_u16::<LittleEndian>(h.matrix_id).unwrap();
            buf.write_u16::<LittleEndian>(h.script_file_id).unwrap();
            buf.write_u16::<LittleEndian>(h.level_script_id).unwrap();
            buf.write_u16::<LittleEndian>(h.text_archive_id).unwrap();
            buf.write_u16::<LittleEndian>(h.music_day_id).unwrap();
            buf.write_u16::<LittleEndian>(h.music_night_id).unwrap();
            buf.write_u16::<LittleEndian>(h.wild_pokemon).unwrap();
            buf.write_u16::<LittleEndian>(h.event_file_id).unwrap();
            buf.write_u16::<LittleEndian>(h.location_name).unwrap();
            buf.write_u8(h.weather_id).unwrap();
            buf.write_u8(h.camera_angle_id).unwrap();
            buf.write_u8(h.location_specifier).unwrap();
            let map_settings = (h.battle_background & 0b_1111) | ((h.flags & 0b_1111) << 4);
            buf.write_u8(map_settings).unwrap();
        }
        MapHeader::Pt(h) => {
            buf.write_u8(h.area_data_id).unwrap();
            buf.write_u8(h.unknown1).unwrap();
            buf.write_u16::<LittleEndian>(h.matrix_id).unwrap();
            buf.write_u16::<LittleEndian>(h.script_file_id).unwrap();
            buf.write_u16::<LittleEndian>(h.level_script_id).unwrap();
            buf.write_u16::<LittleEndian>(h.text_archive_id).unwrap();
            buf.write_u16::<LittleEndian>(h.music_day_id).unwrap();
            buf.write_u16::<LittleEndian>(h.music_night_id).unwrap();
            buf.write_u16::<LittleEndian>(h.wild_pokemon).unwrap();
            buf.write_u16::<LittleEndian>(h.event_file_id).unwrap();
            buf.write_u8(h.location_name).unwrap();
            buf.write_u8(h.area_icon).unwrap();
            buf.write_u8(h.weather_id).unwrap();
            buf.write_u8(h.camera_angle_id).unwrap();
            let map_settings: u16 = (h.location_specifier as u16 & 0b_0111_1111)
                | ((h.battle_background as u16 & 0b_1_1111) << 7)
                | ((h.flags as u16 & 0b_1111) << 12);
            buf.write_u16::<LittleEndian>(map_settings).unwrap();
        }
        MapHeader::HGSS(h) => {
            buf.write_u8(h.wild_pokemon).unwrap();
            buf.write_u8(h.area_data_id).unwrap();
            let coords: u16 = (h.unknown0 as u16 & 0b_1111)
                | ((h.worldmap_x as u16 & 0b_11_1111) << 4)
                | ((h.worldmap_y as u16 & 0b_11_1111) << 10);
            buf.write_u16::<LittleEndian>(coords).unwrap();
            buf.write_u16::<LittleEndian>(h.matrix_id).unwrap();
            buf.write_u16::<LittleEndian>(h.script_file_id).unwrap();
            buf.write_u16::<LittleEndian>(h.level_script_id).unwrap();
            buf.write_u16::<LittleEndian>(h.text_archive_id).unwrap();
            buf.write_u16::<LittleEndian>(h.music_day_id).unwrap();
            buf.write_u16::<LittleEndian>(h.music_night_id).unwrap();
            buf.write_u16::<LittleEndian>(h.event_file_id).unwrap();
            buf.write_u8(h.location_name).unwrap();
            let area_props = (h.area_icon & 0b_1111) | ((h.unknown1 & 0b_1111) << 4);
            buf.write_u8(area_props).unwrap();
            let mut last32: u32 = 0;
            if h.kanto_flag {
                last32 |= 1;
            }
            last32 |= (h.weather_id as u32 & 0b_111_1111) << 1;
            last32 |= (h.location_type as u32 & 0b_1111) << 8;
            last32 |= (h.camera_angle_id as u32 & 0b_11_1111) << 12;
            last32 |= (h.follow_mode as u32 & 0b_11) << 18;
            last32 |= (h.battle_background as u32 & 0b_1_1111) << 20;
            last32 |= (h.flags as u32 & 0b_111_1111) << 25;
            buf.write_u32::<LittleEndian>(last32).unwrap();
        }
    }

    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_pt_header_roundtrip() {
        let original = MapHeader::Pt(MapHeaderPt {
            area_data_id: 0x06,
            unknown1: 0x00,
            matrix_id: 0x00,
            script_file_id: 0x0100,
            level_script_id: 0x0200,
            text_archive_id: 0x0005,
            music_day_id: 0x0400,
            music_night_id: 0x0401,
            wild_pokemon: 0xFFFF,
            event_file_id: 0x0003,
            location_name: 0x03,
            area_icon: 0x01,
            weather_id: 0x00,
            camera_angle_id: 0x00,
            location_specifier: 0x01,
            battle_background: 0x02,
            flags: 0x0F,
        });

        let bytes = write_map_header_to_bytes(&original);
        assert_eq!(bytes.len(), MAP_HEADER_SIZE);

        let parsed = read_map_header_from_bytes(&bytes, GameFamily::Platinum).unwrap();
        assert_eq!(original, parsed);
    }

    fn dp_header_strategy() -> impl Strategy<Value = MapHeader> {
        let part1 = (
            any::<u8>(),
            any::<u8>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
        );
        let part2 = (
            any::<u16>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            0u8..16,
            0u8..16,
        );

        (part1, part2).prop_map(
            |(
                (
                    area_data_id,
                    unknown1,
                    matrix_id,
                    script_file_id,
                    level_script_id,
                    text_archive_id,
                    music_day_id,
                    music_night_id,
                    wild_pokemon,
                    event_file_id,
                ),
                (
                    location_name,
                    weather_id,
                    camera_angle_id,
                    location_specifier,
                    battle_background,
                    flags,
                ),
            )| {
                MapHeader::DP(MapHeaderDP {
                    area_data_id,
                    unknown1,
                    matrix_id,
                    script_file_id,
                    level_script_id,
                    text_archive_id,
                    music_day_id,
                    music_night_id,
                    wild_pokemon,
                    event_file_id,
                    location_name,
                    weather_id,
                    camera_angle_id,
                    location_specifier,
                    battle_background,
                    flags,
                })
            },
        )
    }

    fn pt_header_strategy() -> impl Strategy<Value = MapHeader> {
        let part1 = (
            any::<u8>(),
            any::<u8>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
        );
        let part2 = (
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            0u8..128,
            0u8..32,
            0u8..16,
        );

        (part1, part2).prop_map(
            |(
                (
                    area_data_id,
                    unknown1,
                    matrix_id,
                    script_file_id,
                    level_script_id,
                    text_archive_id,
                    music_day_id,
                    music_night_id,
                    wild_pokemon,
                    event_file_id,
                ),
                (
                    location_name,
                    area_icon,
                    weather_id,
                    camera_angle_id,
                    location_specifier,
                    battle_background,
                    flags,
                ),
            )| {
                MapHeader::Pt(MapHeaderPt {
                    area_data_id,
                    unknown1,
                    matrix_id,
                    script_file_id,
                    level_script_id,
                    text_archive_id,
                    music_day_id,
                    music_night_id,
                    wild_pokemon,
                    event_file_id,
                    location_name,
                    area_icon,
                    weather_id,
                    camera_angle_id,
                    location_specifier,
                    battle_background,
                    flags,
                })
            },
        )
    }

    fn hgss_header_strategy() -> impl Strategy<Value = MapHeader> {
        let part1 = (
            any::<u8>(),
            any::<u8>(),
            0u8..16,
            0u8..64,
            0u8..64,
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
        );
        let part2 = (
            any::<u16>(),
            any::<u8>(),
            0u8..16,
            0u8..16,
            any::<bool>(),
            0u8..128,
            0u8..16,
            0u8..64,
            0u8..4,
            0u8..32,
            0u8..128,
        );

        (part1, part2).prop_map(
            |(
                (
                    wild_pokemon,
                    area_data_id,
                    unknown0,
                    worldmap_x,
                    worldmap_y,
                    matrix_id,
                    script_file_id,
                    level_script_id,
                    text_archive_id,
                    music_day_id,
                    music_night_id,
                ),
                (
                    event_file_id,
                    location_name,
                    area_icon,
                    unknown1,
                    kanto_flag,
                    weather_id,
                    location_type,
                    camera_angle_id,
                    follow_mode,
                    battle_background,
                    flags,
                ),
            )| {
                MapHeader::HGSS(MapHeaderHGSS {
                    wild_pokemon,
                    area_data_id,
                    unknown0,
                    worldmap_x,
                    worldmap_y,
                    matrix_id,
                    script_file_id,
                    level_script_id,
                    text_archive_id,
                    music_day_id,
                    music_night_id,
                    event_file_id,
                    location_name,
                    area_icon,
                    unknown1,
                    kanto_flag,
                    weather_id,
                    location_type,
                    camera_angle_id,
                    follow_mode,
                    battle_background,
                    flags,
                })
            },
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 64,
            .. ProptestConfig::default()
        })]

        #[test]
        fn prop_dp_header_roundtrip(header in dp_header_strategy()) {
            let bytes = write_map_header_to_bytes(&header);
            prop_assert_eq!(bytes.len(), MAP_HEADER_SIZE);
            let parsed = read_map_header_from_bytes(&bytes, GameFamily::DP).unwrap();
            prop_assert_eq!(header, parsed);
        }

        #[test]
        fn prop_pt_header_roundtrip(header in pt_header_strategy()) {
            let bytes = write_map_header_to_bytes(&header);
            prop_assert_eq!(bytes.len(), MAP_HEADER_SIZE);
            let parsed = read_map_header_from_bytes(&bytes, GameFamily::Platinum).unwrap();
            prop_assert_eq!(header, parsed);
        }

        #[test]
        fn prop_hgss_header_roundtrip(header in hgss_header_strategy()) {
            let bytes = write_map_header_to_bytes(&header);
            prop_assert_eq!(bytes.len(), MAP_HEADER_SIZE);
            let parsed = read_map_header_from_bytes(&bytes, GameFamily::HGSS).unwrap();
            prop_assert_eq!(header, parsed);
        }
    }
}
