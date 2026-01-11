use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CEnumVariant {
    pub name: String,
    pub value: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CEnum {
    pub name: Option<String>,
    pub variants: Vec<CEnumVariant>,
}

impl CEnum {
    pub fn get_value(&self, name: &str) -> Option<i64> {
        let mut current_value: i64 = 0;
        for variant in &self.variants {
            if let Some(v) = variant.value {
                current_value = v;
            }
            if variant.name == name {
                return Some(current_value);
            }
            current_value += 1;
        }
        None
    }

    pub fn get_name(&self, value: i64) -> Option<&str> {
        let mut current_value: i64 = 0;
        for variant in &self.variants {
            if let Some(v) = variant.value {
                current_value = v;
            }
            if current_value == value {
                return Some(&variant.name);
            }
            current_value += 1;
        }
        None
    }
}

pub fn parse_enum(source: &str) -> Option<CEnum> {
    let enum_pattern = Regex::new(r"enum\s*([A-Za-z_][A-Za-z0-9_]*)?\s*\{([^}]+)\}").unwrap();

    let variant_pattern = Regex::new(r"([A-Za-z_][A-Za-z0-9_]*)(?:\s*=\s*([^,\n]+))?").unwrap();

    let caps = enum_pattern.captures(source)?;
    let name = caps.get(1).map(|m| m.as_str().to_string());
    let body = &caps[2];

    let mut variants = Vec::new();
    for line in body.lines() {
        let line = line.trim().trim_end_matches(',');
        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        if let Some(var_caps) = variant_pattern.captures(line) {
            let var_name = var_caps[1].to_string();
            let value = var_caps.get(2).and_then(|m| {
                let v = m.as_str().trim();
                if v.starts_with("0x") || v.starts_with("0X") {
                    i64::from_str_radix(&v[2..], 16).ok()
                } else {
                    v.parse().ok()
                }
            });
            variants.push(CEnumVariant {
                name: var_name,
                value,
            });
        }
    }

    Some(CEnum { name, variants })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_enum() {
        let source = r#"
enum MapHeader {
    MAP_HEADER_EVERYWHERE = 0,
    MAP_HEADER_NOTHING,
    MAP_HEADER_UNDERGROUND,
    MAP_HEADER_JUBILIFE_CITY,
    MAP_HEADER_COUNT = 593,
}
        "#;

        let e = parse_enum(source).unwrap();
        assert_eq!(e.name, Some("MapHeader".to_string()));
        assert_eq!(e.get_value("MAP_HEADER_EVERYWHERE"), Some(0));
        assert_eq!(e.get_value("MAP_HEADER_NOTHING"), Some(1));
        assert_eq!(e.get_value("MAP_HEADER_UNDERGROUND"), Some(2));
        assert_eq!(e.get_value("MAP_HEADER_COUNT"), Some(593));
    }
}
