#[derive(Debug, Clone)]
pub struct TextAttrs {
    pub family: FontFamily,
    pub size: f32,
    pub weight: u16,
    pub style: FontStyle,
}

#[derive(Debug, Clone)]
pub enum FontFamily {
    SansSerif,
    Serif,
    Monospace,
    Named(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    Normal,
    Italic,
}

impl Default for TextAttrs {
    fn default() -> Self {
        Self {
            family: FontFamily::SansSerif,
            size: 14.0,
            weight: 400,
            style: FontStyle::Normal,
        }
    }
}

impl TextAttrs {
    pub(crate) fn to_cosmic(&self) -> cosmic_text::Attrs<'_> {
        let family = match &self.family {
            FontFamily::SansSerif => cosmic_text::Family::SansSerif,
            FontFamily::Serif => cosmic_text::Family::Serif,
            FontFamily::Monospace => cosmic_text::Family::Monospace,
            FontFamily::Named(name) => cosmic_text::Family::Name(name.as_str()),
        };
        let style = match self.style {
            FontStyle::Normal => cosmic_text::Style::Normal,
            FontStyle::Italic => cosmic_text::Style::Italic,
        };
        cosmic_text::Attrs::new()
            .family(family)
            .weight(cosmic_text::Weight(self.weight))
            .style(style)
    }
}
