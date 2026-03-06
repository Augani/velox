#[derive(Debug, Clone, serde::Deserialize)]
pub struct TokenDefinition {
    pub name: String,
    pub token_type: String,
    pub default: String,
}

#[derive(Debug, serde::Deserialize)]
struct TokensFile {
    tokens: Vec<TokenDefinition>,
}

pub fn parse_tokens_from_toml(toml_str: &str) -> Result<Vec<TokenDefinition>, toml::de::Error> {
    let file: TokensFile = toml::from_str(toml_str)?;
    Ok(file.tokens)
}

pub fn generate_style_tokens(tokens: &[TokenDefinition]) -> String {
    if tokens.is_empty() {
        return String::from(
            "#[derive(Debug, Clone)]\npub struct ThemeTokens {}\nimpl Default for ThemeTokens {\n    fn default() -> Self {\n        Self {}\n    }\n}\n",
        );
    }

    let mut fields = String::new();
    let mut defaults = String::new();

    for token in tokens {
        let rust_type = map_token_type(&token.token_type);
        fields.push_str(&format!("    pub {}: {},\n", token.name, rust_type));
        let default_value = map_default_value(&token.token_type, &token.default);
        defaults.push_str(&format!("            {}: {},\n", token.name, default_value));
    }

    format!(
        "#[derive(Debug, Clone)]\npub struct ThemeTokens {{\n{}}}\nimpl Default for ThemeTokens {{\n    fn default() -> Self {{\n        Self {{\n{}\
        }}\n    }}\n}}\n",
        fields, defaults,
    )
}

fn map_token_type(token_type: &str) -> &'static str {
    match token_type {
        "color" => "[u8; 4]",
        "f32" => "f32",
        "bool" => "bool",
        _ => "String",
    }
}

fn map_default_value(token_type: &str, default: &str) -> String {
    match token_type {
        "color" => {
            let parts: Vec<&str> = default.split(',').collect();
            if parts.len() == 4 {
                format!(
                    "[{}, {}, {}, {}]",
                    parts[0].trim(),
                    parts[1].trim(),
                    parts[2].trim(),
                    parts[3].trim()
                )
            } else {
                String::from("[0, 0, 0, 255]")
            }
        }
        "f32" => {
            if default.contains('.') {
                format!("{}f32", default)
            } else {
                format!("{}.0f32", default)
            }
        }
        "bool" => default.to_string(),
        _ => format!("String::from(\"{}\")", default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_tokens_from_definitions() {
        let tokens = vec![
            TokenDefinition {
                name: "bg_primary".into(),
                token_type: "color".into(),
                default: "30,30,35,255".into(),
            },
            TokenDefinition {
                name: "font_size".into(),
                token_type: "f32".into(),
                default: "14.0".into(),
            },
            TokenDefinition {
                name: "rounded".into(),
                token_type: "bool".into(),
                default: "true".into(),
            },
        ];

        let output = generate_style_tokens(&tokens);
        assert!(output.contains("pub bg_primary: [u8; 4]"));
        assert!(output.contains("pub font_size: f32"));
        assert!(output.contains("pub rounded: bool"));
        assert!(output.contains("[30, 30, 35, 255]"));
        assert!(output.contains("14.0f32"));
        assert!(output.contains("rounded: true"));
    }

    #[test]
    fn parse_toml_round_trip() {
        let toml_input = r#"
[[tokens]]
name = "bg_primary"
token_type = "color"
default = "30,30,35,255"

[[tokens]]
name = "font_size"
token_type = "f32"
default = "14.0"
"#;

        let tokens = parse_tokens_from_toml(toml_input).unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].name, "bg_primary");
        assert_eq!(tokens[1].token_type, "f32");

        let output = generate_style_tokens(&tokens);
        assert!(output.contains("pub bg_primary: [u8; 4]"));
        assert!(output.contains("pub font_size: f32"));
    }

    #[test]
    fn empty_tokens_generates_empty_struct() {
        let output = generate_style_tokens(&[]);
        assert!(output.contains("pub struct ThemeTokens {}"));
    }
}
