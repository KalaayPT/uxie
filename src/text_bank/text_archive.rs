use std::fs;
use std::path::Path;

/// Decode all binary text archives in `binary_dir` to `.json` files in
/// `output_dir` using the built-in HGSS charmap.
pub fn decode_text_archives(
    binary_dir: &Path,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let charmap = chatot::get_default_charmap();
    fs::create_dir_all(output_dir)?;

    for entry in fs::read_dir(binary_dir)? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }

        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
        let json_path = output_dir.join(format!("{stem}.json"));

        let mut file = fs::File::open(&path)?;
        let archive = chatot::decode_archive(charmap, &mut file, false)?;

        // Output in the chatot-compatible JSON format so encode_text_archives
        // can read it back directly.
        let json = serde_json::json!({
            "key": archive.key,
            "messages": archive.messages.iter().enumerate().map(|(i, msg)| {
                serde_json::json!({
                    "id": format!("msg_{stem}_{i:05}"),
                    "en_US": msg,
                })
            }).collect::<Vec<_>>(),
        });
        fs::write(&json_path, serde_json::to_string_pretty(&json)?)?;
    }

    Ok(())
}

/// Encode all `.json` text archive sources in `source_dir` to binary archives
/// in `output_dir` using the built-in HGSS charmap.
pub fn encode_text_archives(
    source_dir: &Path,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let charmap = chatot::get_default_charmap();
    fs::create_dir_all(output_dir)?;

    for entry in fs::read_dir(source_dir)? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !ext.eq_ignore_ascii_case("json") {
            continue;
        }

        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
        let binary_path = output_dir.join(stem);

        let src = chatot::TextSource {
            txt: Some(vec![path]),
            text_dir: None,
        };
        let dst = chatot::BinarySource {
            archive: Some(vec![binary_path]),
            archive_dir: None,
        };
        let settings = chatot::Settings {
            json: true,
            lang: "en_US".to_string(),
            newer_only: false,
            msgenc_format: false,
        };
        chatot::encode::encode_texts(charmap, &src, &dst, &settings)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_decode_text_archives_directory() {
        let dir = tempdir().unwrap();
        let out = tempdir().unwrap();

        // Create a dummy binary archive using chatot directly
        let archive_path = dir.path().join("7_000");
        let charmap = chatot::get_default_charmap();
        let text = "// Key: 0xABCD\nHello\nWorld\n";
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), text).unwrap();
        let src = chatot::TextSource {
            txt: Some(vec![temp.path().to_path_buf()]),
            text_dir: None,
        };
        let dst = chatot::BinarySource {
            archive: Some(vec![archive_path.clone()]),
            archive_dir: None,
        };
        let settings = chatot::Settings {
            json: false,
            lang: "en_US".to_string(),
            newer_only: false,
            msgenc_format: false,
        };
        chatot::encode::encode_texts(&charmap, &src, &dst, &settings).unwrap();

        // Now decode using our wrapper
        decode_text_archives(dir.path(), out.path()).unwrap();

        let json_path = out.path().join("7_000.json");
        assert!(json_path.exists(), "decoded JSON should exist");

        let content = std::fs::read_to_string(&json_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["key"], 0xABCD);
        assert_eq!(parsed["messages"][0]["en_US"], "Hello");
        assert_eq!(parsed["messages"][1]["en_US"], "World");
    }

    #[test]
    fn test_encode_text_archives_directory() {
        let dir = tempdir().unwrap();
        let out = tempdir().unwrap();

        // Create a JSON source file
        let json_content = serde_json::json!({
            "key": 0x1234,
            "messages": [
                {"id": "msg_000", "en_US": "First"},
                {"id": "msg_001", "en_US": "Second"},
            ]
        });
        let json_path = dir.path().join("7_001.json");
        std::fs::write(
            &json_path,
            serde_json::to_string_pretty(&json_content).unwrap(),
        )
        .unwrap();

        // Encode using our wrapper
        encode_text_archives(dir.path(), out.path()).unwrap();

        let binary_path = out.path().join("7_001");
        assert!(binary_path.exists(), "encoded binary should exist");

        // Decode it back and verify content
        let charmap = chatot::get_default_charmap();
        let mut file = std::fs::File::open(&binary_path).unwrap();
        let archive = chatot::decode_archive(&charmap, &mut file, false).unwrap();
        assert_eq!(archive.key, 0x1234);
        assert_eq!(archive.messages, vec!["First", "Second"]);
    }

    #[test]
    fn test_text_archives_round_trip() {
        let dir = tempdir().unwrap();
        let json_out = tempdir().unwrap();
        let bin_out = tempdir().unwrap();

        // Create source binary
        let archive_path = dir.path().join("7_002");
        let charmap = chatot::get_default_charmap();
        let text = "// Key: 0xCAFE\nMessage one\nMessage two\nMessage three\n";
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), text).unwrap();
        let src = chatot::TextSource {
            txt: Some(vec![temp.path().to_path_buf()]),
            text_dir: None,
        };
        let dst = chatot::BinarySource {
            archive: Some(vec![archive_path]),
            archive_dir: None,
        };
        let settings = chatot::Settings {
            json: false,
            lang: "en_US".to_string(),
            newer_only: false,
            msgenc_format: false,
        };
        chatot::encode::encode_texts(&charmap, &src, &dst, &settings).unwrap();

        // Decode
        decode_text_archives(dir.path(), json_out.path()).unwrap();

        // Encode back
        encode_text_archives(json_out.path(), bin_out.path()).unwrap();

        // Verify
        let mut file = std::fs::File::open(bin_out.path().join("7_002")).unwrap();
        let archive = chatot::decode_archive(&charmap, &mut file, false).unwrap();
        assert_eq!(archive.key, 0xCAFE);
        assert_eq!(
            archive.messages,
            vec!["Message one", "Message two", "Message three"]
        );
    }
}
