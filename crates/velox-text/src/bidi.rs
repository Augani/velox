use crate::buffer::TextBuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParagraphDirection {
    LeftToRight,
    RightToLeft,
}

pub fn paragraph_direction(text: &str) -> ParagraphDirection {
    for ch in text.chars() {
        match unicode_bidi_category(ch) {
            Some(BidiCategory::StrongLtr) => return ParagraphDirection::LeftToRight,
            Some(BidiCategory::StrongRtl) => return ParagraphDirection::RightToLeft,
            _ => continue,
        }
    }
    ParagraphDirection::LeftToRight
}

enum BidiCategory {
    StrongLtr,
    StrongRtl,
}

fn unicode_bidi_category(ch: char) -> Option<BidiCategory> {
    let cp = ch as u32;
    if (0x0041..=0x005A).contains(&cp)
        || (0x0061..=0x007A).contains(&cp)
        || (0x00C0..=0x024F).contains(&cp)
    {
        return Some(BidiCategory::StrongLtr);
    }
    if (0x0590..=0x05FF).contains(&cp)
        || (0x0600..=0x06FF).contains(&cp)
        || (0x0700..=0x074F).contains(&cp)
        || (0xFB50..=0xFDFF).contains(&cp)
        || (0xFE70..=0xFEFF).contains(&cp)
    {
        return Some(BidiCategory::StrongRtl);
    }
    None
}

pub fn is_rtl_run(buffer: &TextBuffer, line_index: usize) -> bool {
    for (idx, run) in buffer.layout_runs().enumerate() {
        if idx == line_index {
            return run.rtl;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_is_ltr() {
        assert_eq!(
            paragraph_direction("Hello world"),
            ParagraphDirection::LeftToRight
        );
    }

    #[test]
    fn arabic_is_rtl() {
        assert_eq!(
            paragraph_direction("مرحبا"),
            ParagraphDirection::RightToLeft
        );
    }

    #[test]
    fn hebrew_is_rtl() {
        assert_eq!(paragraph_direction("שלום"), ParagraphDirection::RightToLeft);
    }

    #[test]
    fn empty_defaults_to_ltr() {
        assert_eq!(paragraph_direction(""), ParagraphDirection::LeftToRight);
    }

    #[test]
    fn numbers_only_defaults_to_ltr() {
        assert_eq!(
            paragraph_direction("12345"),
            ParagraphDirection::LeftToRight
        );
    }

    #[test]
    fn mixed_follows_first_strong() {
        assert_eq!(
            paragraph_direction("مرحبا Hello"),
            ParagraphDirection::RightToLeft
        );
        assert_eq!(
            paragraph_direction("Hello مرحبا"),
            ParagraphDirection::LeftToRight
        );
    }
}
