#[derive(Debug, Clone, serde::Deserialize)]
pub struct IconEntry {
    pub name: String,
    pub path: String,
}

#[derive(Debug, serde::Deserialize)]
struct IconsFile {
    icons: Vec<IconEntry>,
}

pub fn parse_icons_from_toml(toml_str: &str) -> Result<Vec<IconEntry>, toml::de::Error> {
    let file: IconsFile = toml::from_str(toml_str)?;
    Ok(file.icons)
}

pub fn generate_icon_enum(icons: &[IconEntry]) -> String {
    if icons.is_empty() {
        return String::from(
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\npub enum Icon {}\n",
        );
    }

    let mut variants = String::new();
    let mut match_arms = String::new();

    for icon in icons {
        let variant = to_pascal_case(&icon.name);
        variants.push_str(&format!("    {},\n", variant));
        match_arms.push_str(&format!(
            "            Icon::{} => include_bytes!(\"{}\"),\n",
            variant, icon.path,
        ));
    }

    format!(
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\npub enum Icon {{\n{}}}\nimpl Icon {{\n    pub fn data(&self) -> &'static [u8] {{\n        match self {{\n{}\
        }}\n    }}\n}}\n",
        variants, match_arms,
    )
}

fn to_pascal_case(input: &str) -> String {
    input
        .split(['_', '-', ' '])
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    let rest: String = chars.collect();
                    format!("{}{}", upper, rest)
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_enum_from_entries() {
        let icons = vec![
            IconEntry {
                name: "close".into(),
                path: "../icons/close.png".into(),
            },
            IconEntry {
                name: "settings".into(),
                path: "../icons/settings.png".into(),
            },
            IconEntry {
                name: "search".into(),
                path: "../icons/search.png".into(),
            },
        ];

        let output = generate_icon_enum(&icons);
        assert!(output.contains("Close,"));
        assert!(output.contains("Settings,"));
        assert!(output.contains("Search,"));
        assert!(output.contains("Icon::Close => include_bytes!(\"../icons/close.png\")"));
        assert!(output.contains("Icon::Settings => include_bytes!(\"../icons/settings.png\")"));
    }

    #[test]
    fn parse_icons_toml() {
        let toml_input = r#"
[[icons]]
name = "close"
path = "../icons/close.png"

[[icons]]
name = "settings"
path = "../icons/settings.png"
"#;

        let icons = parse_icons_from_toml(toml_input).unwrap();
        assert_eq!(icons.len(), 2);
        assert_eq!(icons[0].name, "close");
        assert_eq!(icons[1].path, "../icons/settings.png");
    }

    #[test]
    fn empty_icons_generates_empty_enum() {
        let output = generate_icon_enum(&[]);
        assert!(output.contains("pub enum Icon {}"));
    }

    #[test]
    fn pascal_case_conversion() {
        let icons = vec![IconEntry {
            name: "arrow_left".into(),
            path: "arrow_left.png".into(),
        }];

        let output = generate_icon_enum(&icons);
        assert!(output.contains("ArrowLeft,"));
    }
}
