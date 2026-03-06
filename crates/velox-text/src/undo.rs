const MAX_UNDO_DEPTH: usize = 100;

#[derive(Debug, Clone, PartialEq)]
pub enum EditCommand {
    Insert {
        position: usize,
        text: String,
    },
    Delete {
        position: usize,
        text: String,
    },
    Replace {
        position: usize,
        old: String,
        new: String,
    },
}

pub struct UndoStack {
    undo: Vec<EditCommand>,
    redo: Vec<EditCommand>,
}

impl UndoStack {
    pub fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn push(&mut self, cmd: EditCommand) {
        self.redo.clear();
        self.undo.push(cmd);
        if self.undo.len() > MAX_UNDO_DEPTH {
            self.undo.remove(0);
        }
    }

    pub fn push_coalesced(&mut self, cmd: EditCommand) {
        self.redo.clear();
        if let EditCommand::Insert { position, text } = &cmd {
            if let Some(EditCommand::Insert {
                position: prev_pos,
                text: prev_text,
            }) = self.undo.last_mut()
            {
                if *prev_pos + prev_text.len() == *position {
                    prev_text.push_str(text);
                    return;
                }
            }
        }
        self.undo.push(cmd);
        if self.undo.len() > MAX_UNDO_DEPTH {
            self.undo.remove(0);
        }
    }

    pub fn undo(&mut self) -> Option<EditCommand> {
        let cmd = self.undo.pop()?;
        self.redo.push(cmd.clone());
        Some(cmd)
    }

    pub fn redo(&mut self) -> Option<EditCommand> {
        let cmd = self.redo.pop()?;
        self.undo.push(cmd.clone());
        Some(cmd)
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_undo() {
        let mut stack = UndoStack::new();
        stack.push(EditCommand::Insert {
            position: 0,
            text: "hello".into(),
        });
        let cmd = stack.undo();
        assert!(cmd.is_some());
        match cmd.unwrap() {
            EditCommand::Insert { position, text } => {
                assert_eq!(position, 0);
                assert_eq!(text, "hello");
            }
            _ => panic!("expected Insert"),
        }
    }

    #[test]
    fn undo_then_redo() {
        let mut stack = UndoStack::new();
        stack.push(EditCommand::Insert {
            position: 0,
            text: "hi".into(),
        });
        stack.undo();
        let cmd = stack.redo();
        assert!(cmd.is_some());
    }

    #[test]
    fn push_after_undo_clears_redo() {
        let mut stack = UndoStack::new();
        stack.push(EditCommand::Insert {
            position: 0,
            text: "a".into(),
        });
        stack.push(EditCommand::Insert {
            position: 1,
            text: "b".into(),
        });
        stack.undo();
        stack.push(EditCommand::Insert {
            position: 1,
            text: "c".into(),
        });
        let redo = stack.redo();
        assert!(redo.is_none());
    }

    #[test]
    fn coalesce_consecutive_inserts() {
        let mut stack = UndoStack::new();
        stack.push_coalesced(EditCommand::Insert {
            position: 0,
            text: "h".into(),
        });
        stack.push_coalesced(EditCommand::Insert {
            position: 1,
            text: "e".into(),
        });
        stack.push_coalesced(EditCommand::Insert {
            position: 2,
            text: "l".into(),
        });
        let cmd = stack.undo();
        assert!(cmd.is_some());
        match cmd.unwrap() {
            EditCommand::Insert { position, text } => {
                assert_eq!(position, 0);
                assert_eq!(text, "hel");
            }
            _ => panic!("expected coalesced Insert"),
        }
        assert!(stack.undo().is_none());
    }

    #[test]
    fn delete_breaks_coalescing() {
        let mut stack = UndoStack::new();
        stack.push_coalesced(EditCommand::Insert {
            position: 0,
            text: "a".into(),
        });
        stack.push(EditCommand::Delete {
            position: 0,
            text: "a".into(),
        });
        assert!(stack.undo().is_some());
        assert!(stack.undo().is_some());
        assert!(stack.undo().is_none());
    }

    #[test]
    fn empty_undo_returns_none() {
        let mut stack = UndoStack::new();
        assert!(stack.undo().is_none());
    }

    #[test]
    fn stack_cap() {
        let mut stack = UndoStack::new();
        for i in 0..150 {
            stack.push(EditCommand::Insert {
                position: i,
                text: "x".into(),
            });
        }
        let mut count = 0;
        while stack.undo().is_some() {
            count += 1;
        }
        assert!(count <= 100);
    }
}
