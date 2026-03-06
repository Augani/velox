#[derive(Debug, Clone, Default)]
pub struct CompositionState {
    pub preedit_text: Option<String>,
    pub cursor_range: Option<(usize, usize)>,
}

impl CompositionState {
    pub fn is_composing(&self) -> bool {
        self.preedit_text.is_some()
    }

    pub fn set_preedit(&mut self, text: String, cursor: Option<(usize, usize)>) {
        self.preedit_text = Some(text);
        self.cursor_range = cursor;
    }

    pub fn commit(&mut self) -> Option<String> {
        self.cursor_range = None;
        self.preedit_text.take()
    }

    pub fn clear(&mut self) {
        self.preedit_text = None;
        self.cursor_range = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initially_not_composing() {
        let state = CompositionState::default();
        assert!(!state.is_composing());
    }

    #[test]
    fn set_preedit_activates_composing() {
        let mut state = CompositionState::default();
        state.set_preedit("にほ".into(), Some((0, 6)));
        assert!(state.is_composing());
        assert_eq!(state.preedit_text.as_deref(), Some("にほ"));
    }

    #[test]
    fn commit_returns_text_and_clears() {
        let mut state = CompositionState::default();
        state.set_preedit("日本語".into(), None);
        let committed = state.commit();
        assert_eq!(committed.as_deref(), Some("日本語"));
        assert!(!state.is_composing());
    }

    #[test]
    fn clear_resets_state() {
        let mut state = CompositionState::default();
        state.set_preedit("test".into(), Some((0, 4)));
        state.clear();
        assert!(!state.is_composing());
    }
}
