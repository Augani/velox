#[derive(Debug, Clone)]
pub enum ImeEvent {
    Enabled,
    Disabled,
    Preedit {
        text: String,
        cursor_range: Option<(usize, usize)>,
    },
    Commit {
        text: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preedit_event_construction() {
        let event = ImeEvent::Preedit {
            text: "にほ".into(),
            cursor_range: Some((0, 6)),
        };
        if let ImeEvent::Preedit { text, cursor_range } = &event {
            assert_eq!(text, "にほ");
            assert_eq!(*cursor_range, Some((0, 6)));
        } else {
            panic!("expected Preedit");
        }
    }

    #[test]
    fn commit_event_construction() {
        let event = ImeEvent::Commit {
            text: "日本".into(),
        };
        if let ImeEvent::Commit { text } = &event {
            assert_eq!(text, "日本");
        } else {
            panic!("expected Commit");
        }
    }
}
