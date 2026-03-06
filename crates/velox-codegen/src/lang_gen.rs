#[derive(Debug, Clone, serde::Deserialize)]
pub struct LangEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, serde::Deserialize)]
struct LangFile {
    strings: Vec<LangEntry>,
}

pub fn parse_lang_from_toml(toml_str: &str) -> Result<Vec<LangEntry>, toml::de::Error> {
    let file: LangFile = toml::from_str(toml_str)?;
    Ok(file.strings)
}

pub fn generate_lang_module(entries: &[LangEntry]) -> String {
    if entries.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    for entry in entries {
        let escaped = entry.value.replace('\\', "\\\\").replace('"', "\\\"");
        output.push_str(&format!(
            "pub fn {}() -> &'static str {{ \"{}\" }}\n",
            entry.key, escaped,
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_module_from_entries() {
        let entries = vec![
            LangEntry {
                key: "greeting".into(),
                value: "Hello".into(),
            },
            LangEntry {
                key: "app_name".into(),
                value: "MyApp".into(),
            },
        ];

        let output = generate_lang_module(&entries);
        assert!(output.contains("pub fn greeting() -> &'static str { \"Hello\" }"));
        assert!(output.contains("pub fn app_name() -> &'static str { \"MyApp\" }"));
    }

    #[test]
    fn parse_lang_toml() {
        let toml_input = r#"
[[strings]]
key = "greeting"
value = "Hello"

[[strings]]
key = "app_name"
value = "MyApp"
"#;

        let entries = parse_lang_from_toml(toml_input).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "greeting");
        assert_eq!(entries[0].value, "Hello");
        assert_eq!(entries[1].key, "app_name");
    }

    #[test]
    fn empty_entries_generates_empty_string() {
        let output = generate_lang_module(&[]);
        assert!(output.is_empty());
    }

    #[test]
    fn escapes_special_characters() {
        let entries = vec![LangEntry {
            key: "quote".into(),
            value: "He said \"hi\"".into(),
        }];

        let output = generate_lang_module(&entries);
        assert!(output.contains(r#"He said \"hi\""#));
    }
}
