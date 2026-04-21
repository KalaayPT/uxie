use super::types::{MapHeader, MapHeaderDP, MapHeaderHGSS, MapHeaderPt, MAP_HEADER_SIZE};
use crate::game::GameFamily;
use binrw::{BinRead, BinWrite};
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

#[derive(Debug, Clone, Copy, BinRead, BinWrite)]
#[brw(little)]
struct MapHeaderDPRaw {
    area_data_id: u8,
    unknown1: u8,
    matrix_id: u16,
    script_file_id: u16,
    level_script_id: u16,
    text_archive_id: u16,
    music_day_id: u16,
    music_night_id: u16,
    wild_pokemon: u16,
    event_file_id: u16,
    location_name: u16,
    weather_id: u8,
    camera_angle_id: u8,
    location_specifier: u8,
    map_settings: u8,
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite)]
#[brw(little)]
struct MapHeaderPtRaw {
    area_data_id: u8,
    unknown1: u8,
    matrix_id: u16,
    script_file_id: u16,
    level_script_id: u16,
    text_archive_id: u16,
    music_day_id: u16,
    music_night_id: u16,
    wild_pokemon: u16,
    event_file_id: u16,
    location_name: u8,
    area_icon: u8,
    weather_id: u8,
    camera_angle_id: u8,
    map_settings: u16,
}

#[derive(Debug, Clone, Copy, BinRead, BinWrite)]
#[brw(little)]
struct MapHeaderHGSSRaw {
    wild_pokemon: u8,
    area_data_id: u8,
    coords: u16,
    matrix_id: u16,
    script_file_id: u16,
    level_script_id: u16,
    text_archive_id: u16,
    music_day_id: u16,
    music_night_id: u16,
    event_file_id: u16,
    location_name: u8,
    area_props: u8,
    last32: u32,
}

fn binrw_to_io_error(err: binrw::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err)
}

fn dp_from_raw(raw: MapHeaderDPRaw) -> MapHeaderDP {
    MapHeaderDP {
        area_data_id: raw.area_data_id,
        unknown1: raw.unknown1,
        matrix_id: raw.matrix_id,
        script_file_id: raw.script_file_id,
        level_script_id: raw.level_script_id,
        text_archive_id: raw.text_archive_id,
        music_day_id: raw.music_day_id,
        music_night_id: raw.music_night_id,
        wild_pokemon: raw.wild_pokemon,
        event_file_id: raw.event_file_id,
        location_name: raw.location_name,
        weather_id: raw.weather_id,
        camera_angle_id: raw.camera_angle_id,
        location_specifier: raw.location_specifier,
        battle_background: raw.map_settings & 0b_1111,
        flags: (raw.map_settings >> 4) & 0b_1111,
    }
}

fn dp_to_raw(header: &MapHeaderDP) -> MapHeaderDPRaw {
    MapHeaderDPRaw {
        area_data_id: header.area_data_id,
        unknown1: header.unknown1,
        matrix_id: header.matrix_id,
        script_file_id: header.script_file_id,
        level_script_id: header.level_script_id,
        text_archive_id: header.text_archive_id,
        music_day_id: header.music_day_id,
        music_night_id: header.music_night_id,
        wild_pokemon: header.wild_pokemon,
        event_file_id: header.event_file_id,
        location_name: header.location_name,
        weather_id: header.weather_id,
        camera_angle_id: header.camera_angle_id,
        location_specifier: header.location_specifier,
        map_settings: (header.battle_background & 0b_1111) | ((header.flags & 0b_1111) << 4),
    }
}

fn pt_from_raw(raw: MapHeaderPtRaw) -> MapHeaderPt {
    MapHeaderPt {
        area_data_id: raw.area_data_id,
        unknown1: raw.unknown1,
        matrix_id: raw.matrix_id,
        script_file_id: raw.script_file_id,
        level_script_id: raw.level_script_id,
        text_archive_id: raw.text_archive_id,
        music_day_id: raw.music_day_id,
        music_night_id: raw.music_night_id,
        wild_pokemon: raw.wild_pokemon,
        event_file_id: raw.event_file_id,
        location_name: raw.location_name,
        area_icon: raw.area_icon,
        weather_id: raw.weather_id,
        camera_angle_id: raw.camera_angle_id,
        location_specifier: (raw.map_settings & 0b_0111_1111) as u8,
        battle_background: ((raw.map_settings >> 7) & 0b_1_1111) as u8,
        flags: ((raw.map_settings >> 12) & 0b_1111) as u8,
    }
}

fn pt_to_raw(header: &MapHeaderPt) -> MapHeaderPtRaw {
    MapHeaderPtRaw {
        area_data_id: header.area_data_id,
        unknown1: header.unknown1,
        matrix_id: header.matrix_id,
        script_file_id: header.script_file_id,
        level_script_id: header.level_script_id,
        text_archive_id: header.text_archive_id,
        music_day_id: header.music_day_id,
        music_night_id: header.music_night_id,
        wild_pokemon: header.wild_pokemon,
        event_file_id: header.event_file_id,
        location_name: header.location_name,
        area_icon: header.area_icon,
        weather_id: header.weather_id,
        camera_angle_id: header.camera_angle_id,
        map_settings: (header.location_specifier as u16 & 0b_0111_1111)
            | ((header.battle_background as u16 & 0b_1_1111) << 7)
            | ((header.flags as u16 & 0b_1111) << 12),
    }
}

fn hgss_from_raw(raw: MapHeaderHGSSRaw) -> MapHeaderHGSS {
    MapHeaderHGSS {
        wild_pokemon: raw.wild_pokemon,
        area_data_id: raw.area_data_id,
        unknown0: (raw.coords & 0b_1111) as u8,
        worldmap_x: ((raw.coords >> 4) & 0b_11_1111) as u8,
        worldmap_y: ((raw.coords >> 10) & 0b_11_1111) as u8,
        matrix_id: raw.matrix_id,
        script_file_id: raw.script_file_id,
        level_script_id: raw.level_script_id,
        text_archive_id: raw.text_archive_id,
        music_day_id: raw.music_day_id,
        music_night_id: raw.music_night_id,
        event_file_id: raw.event_file_id,
        location_name: raw.location_name,
        area_icon: raw.area_props & 0b_1111,
        unknown1: (raw.area_props >> 4) & 0b_1111,
        kanto_flag: (raw.last32 & 0b_1) == 1,
        weather_id: ((raw.last32 >> 1) & 0b_111_1111) as u8,
        location_type: ((raw.last32 >> 8) & 0b_1111) as u8,
        camera_angle_id: ((raw.last32 >> 12) & 0b_11_1111) as u8,
        follow_mode: ((raw.last32 >> 18) & 0b_11) as u8,
        battle_background: ((raw.last32 >> 20) & 0b_1_1111) as u8,
        flags: ((raw.last32 >> 25) & 0b_111_1111) as u8,
    }
}

fn hgss_to_raw(header: &MapHeaderHGSS) -> MapHeaderHGSSRaw {
    let coords = (header.unknown0 as u16 & 0b_1111)
        | ((header.worldmap_x as u16 & 0b_11_1111) << 4)
        | ((header.worldmap_y as u16 & 0b_11_1111) << 10);
    let area_props = (header.area_icon & 0b_1111) | ((header.unknown1 & 0b_1111) << 4);
    let mut last32: u32 = 0;
    if header.kanto_flag {
        last32 |= 1;
    }
    last32 |= (header.weather_id as u32 & 0b_111_1111) << 1;
    last32 |= (header.location_type as u32 & 0b_1111) << 8;
    last32 |= (header.camera_angle_id as u32 & 0b_11_1111) << 12;
    last32 |= (header.follow_mode as u32 & 0b_11) << 18;
    last32 |= (header.battle_background as u32 & 0b_1_1111) << 20;
    last32 |= (header.flags as u32 & 0b_111_1111) << 25;

    MapHeaderHGSSRaw {
        wild_pokemon: header.wild_pokemon,
        area_data_id: header.area_data_id,
        coords,
        matrix_id: header.matrix_id,
        script_file_id: header.script_file_id,
        level_script_id: header.level_script_id,
        text_archive_id: header.text_archive_id,
        music_day_id: header.music_day_id,
        music_night_id: header.music_night_id,
        event_file_id: header.event_file_id,
        location_name: header.location_name,
        area_props,
        last32,
    }
}

fn read_dp_header<R: Read + Seek>(reader: &mut R) -> io::Result<MapHeaderDP> {
    MapHeaderDPRaw::read_le(reader)
        .map(dp_from_raw)
        .map_err(binrw_to_io_error)
}

fn read_pt_header<R: Read + Seek>(reader: &mut R) -> io::Result<MapHeaderPt> {
    MapHeaderPtRaw::read_le(reader)
        .map(pt_from_raw)
        .map_err(binrw_to_io_error)
}

fn read_hgss_header<R: Read + Seek>(reader: &mut R) -> io::Result<MapHeaderHGSS> {
    MapHeaderHGSSRaw::read_le(reader)
        .map(hgss_from_raw)
        .map_err(binrw_to_io_error)
}

pub fn write_map_header_to_bytes(header: &MapHeader) -> Vec<u8> {
    let mut cursor = io::Cursor::new(Vec::with_capacity(MAP_HEADER_SIZE));

    let result = match header {
        MapHeader::DP(h) => dp_to_raw(h).write_le(&mut cursor),
        MapHeader::Pt(h) => pt_to_raw(h).write_le(&mut cursor),
        MapHeader::HGSS(h) => hgss_to_raw(h).write_le(&mut cursor),
    };
    result.map_err(binrw_to_io_error).unwrap();

    let bytes = cursor.into_inner();
    debug_assert_eq!(bytes.len(), MAP_HEADER_SIZE);
    bytes
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
