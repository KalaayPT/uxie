use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{self, Read, Seek, SeekFrom};

pub struct TextArchive {
    pub key: u16,
    pub messages: Vec<String>,
}

impl TextArchive {
    pub fn from_binary<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        let message_count = reader.read_u16::<LittleEndian>()?;
        let key = reader.read_u16::<LittleEndian>()?;

        let mut entries = Vec::with_capacity(message_count as usize);
        for i in 0..message_count {
            let mut offset = reader.read_u32::<LittleEndian>()?;
            let mut length = reader.read_u32::<LittleEndian>()?;

            let mut local_key: u32 = 765;
            local_key = local_key.wrapping_mul((i + 1) as u32);
            local_key = local_key.wrapping_mul(key as u32);
            local_key &= 0xFFFF;
            local_key |= local_key << 16;

            offset ^= local_key;
            length ^= local_key;

            entries.push((offset, length));
        }

        let mut messages = Vec::with_capacity(message_count as usize);
        for (i, (offset, length)) in entries.into_iter().enumerate() {
            reader.seek(SeekFrom::Start(offset as u64))?;
            let mut encrypted = vec![0u16; length as usize];
            for j in 0..length as usize {
                encrypted[j] = reader.read_u16::<LittleEndian>()?;
            }

            let decrypted = Self::decrypt(&encrypted, (i + 1) as u16);
            messages.push(Self::to_string(&decrypted));
        }

        Ok(Self { key, messages })
    }

    fn decrypt(encrypted: &[u16], index: u16) -> Vec<u16> {
        let mut decrypted = Vec::with_capacity(encrypted.len());
        let mut current_key: u16 = (index as u32).wrapping_mul(596947) as u16;

        for &c in encrypted {
            decrypted.push(c ^ current_key);
            current_key = current_key.wrapping_add(18749);
        }

        decrypted
    }

    fn to_string(decrypted: &[u16]) -> String {
        let mut decode_map = std::collections::HashMap::new();
        for i in 0..0x100 {
            decode_map.insert(i as u16, (i as u8 as char).to_string());
        }

        let _charmap = chatot::charmap::Charmap {
            encode_map: std::collections::HashMap::new(),
            decode_map,
            command_map: std::collections::HashMap::new(),
        };

        let mut s = String::new();
        for &c in decrypted {
            if c == 0xFFFF {
                break;
            }
            if c == 0xFFFE {
                continue;
            }
            if c < 0x80 {
                s.push(c as u8 as char);
            } else {
                s.push_str(&format!("\\x{:04X}", c));
            }
        }
        s
    }
}
