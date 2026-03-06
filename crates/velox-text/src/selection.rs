#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Affinity {
    Before,
    After,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextPosition {
    pub index: usize,
    pub affinity: Affinity,
}

impl TextPosition {
    pub fn new(index: usize) -> Self {
        Self {
            index,
            affinity: Affinity::Before,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSelection {
    pub anchor: usize,
    pub focus: usize,
}

impl TextSelection {
    pub fn collapsed(index: usize) -> Self {
        Self {
            anchor: index,
            focus: index,
        }
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.focus
    }

    pub fn range(&self) -> (usize, usize) {
        if self.anchor <= self.focus {
            (self.anchor, self.focus)
        } else {
            (self.focus, self.anchor)
        }
    }

    pub fn selected_text<'a>(&self, source: &'a str) -> &'a str {
        let (start, end) = self.range();
        let start = start.min(source.len());
        let end = end.min(source.len());
        &source[start..end]
    }
}

impl Default for TextSelection {
    fn default() -> Self {
        Self::collapsed(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapsed_selection() {
        let sel = TextSelection::collapsed(5);
        assert!(sel.is_collapsed());
        assert_eq!(sel.range(), (5, 5));
    }

    #[test]
    fn forward_selection() {
        let sel = TextSelection {
            anchor: 2,
            focus: 8,
        };
        assert!(!sel.is_collapsed());
        assert_eq!(sel.range(), (2, 8));
    }

    #[test]
    fn backward_selection() {
        let sel = TextSelection {
            anchor: 10,
            focus: 3,
        };
        assert!(!sel.is_collapsed());
        assert_eq!(sel.range(), (3, 10));
    }

    #[test]
    fn selected_text_extracts_range() {
        let sel = TextSelection {
            anchor: 0,
            focus: 5,
        };
        assert_eq!(sel.selected_text("Hello, world!"), "Hello");
    }

    #[test]
    fn selected_text_backward() {
        let sel = TextSelection {
            anchor: 7,
            focus: 0,
        };
        assert_eq!(sel.selected_text("Hello, world!"), "Hello, ");
    }

    #[test]
    fn collapsed_selected_text_is_empty() {
        let sel = TextSelection::collapsed(3);
        assert_eq!(sel.selected_text("Hello"), "");
    }
}
