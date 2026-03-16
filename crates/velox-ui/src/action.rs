use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use velox_scene::{Key, Modifiers, NodeId};

pub trait Action: Any + Debug + Clone + 'static {
    fn name(&self) -> &'static str;
    fn as_any(&self) -> &dyn Any;
    fn boxed_clone(&self) -> Box<dyn AnyAction>;
}

pub trait AnyAction: Any + Debug {
    fn action_type_id(&self) -> TypeId;
    fn as_any(&self) -> &dyn Any;
    fn boxed_clone(&self) -> Box<dyn AnyAction>;
    fn name(&self) -> &'static str;
}

impl<A: Action> AnyAction for A {
    fn action_type_id(&self) -> TypeId {
        TypeId::of::<A>()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn boxed_clone(&self) -> Box<dyn AnyAction> {
        Box::new(self.clone())
    }

    fn name(&self) -> &'static str {
        Action::name(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Keystroke {
    pub key: Key,
    pub modifiers: Modifiers,
}

pub struct KeyBinding {
    pub keystroke: Keystroke,
    pub context: Option<String>,
    pub action: Box<dyn AnyAction>,
}

pub struct Keymap {
    bindings: Vec<KeyBinding>,
}

impl Keymap {
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    pub fn bind<A: Action>(&mut self, keystroke: Keystroke, context: Option<&str>, action: A) {
        self.bindings.push(KeyBinding {
            keystroke,
            context: context.map(String::from),
            action: Box::new(action),
        });
    }

    pub fn match_keystroke(
        &self,
        keystroke: &Keystroke,
        contexts: &[String],
    ) -> Option<&dyn AnyAction> {
        for binding in self.bindings.iter().rev() {
            if binding.keystroke != *keystroke {
                continue;
            }
            match binding.context {
                Some(ref ctx) if !contexts.contains(ctx) => continue,
                _ => return Some(binding.action.as_ref()),
            }
        }
        None
    }

    pub fn clear(&mut self) {
        self.bindings.clear();
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self::new()
    }
}

pub type ActionHandler = Box<dyn Fn(&dyn Any)>;

pub struct ActionRegistry {
    handlers: HashMap<(NodeId, TypeId), ActionHandler>,
}

impl ActionRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.handlers.clear();
    }

    pub fn register<A: Action>(&mut self, node: NodeId, handler: impl Fn(&A) + 'static) {
        let type_id = TypeId::of::<A>();
        self.handlers.insert(
            (node, type_id),
            Box::new(move |any| {
                if let Some(action) = any.downcast_ref::<A>() {
                    handler(action);
                }
            }),
        );
    }

    pub fn dispatch(&self, node: NodeId, action: &dyn AnyAction) -> bool {
        let key = (node, action.action_type_id());
        if let Some(handler) = self.handlers.get(&key) {
            handler(action.as_any());
            return true;
        }
        false
    }
}

impl Default for ActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    fn test_node_id(n: u64) -> NodeId {
        NodeId::from(slotmap::KeyData::from_ffi(n))
    }

    #[derive(Debug, Clone)]
    struct TestAction;
    impl Action for TestAction {
        fn name(&self) -> &'static str {
            "TestAction"
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn boxed_clone(&self) -> Box<dyn AnyAction> {
            Box::new(self.clone())
        }
    }

    #[derive(Debug, Clone)]
    struct OtherAction;
    impl Action for OtherAction {
        fn name(&self) -> &'static str {
            "OtherAction"
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn boxed_clone(&self) -> Box<dyn AnyAction> {
            Box::new(self.clone())
        }
    }

    #[test]
    fn keymap_matches_binding() {
        let mut keymap = Keymap::new();
        keymap.bind(
            Keystroke {
                key: Key::Enter,
                modifiers: Modifiers::CTRL,
            },
            None,
            TestAction,
        );
        let result = keymap.match_keystroke(
            &Keystroke {
                key: Key::Enter,
                modifiers: Modifiers::CTRL,
            },
            &[],
        );
        assert!(result.is_some());
        assert_eq!(result.unwrap().name(), "TestAction");
    }

    #[test]
    fn keymap_no_match_wrong_key() {
        let mut keymap = Keymap::new();
        keymap.bind(
            Keystroke {
                key: Key::Enter,
                modifiers: Modifiers::CTRL,
            },
            None,
            TestAction,
        );
        let result = keymap.match_keystroke(
            &Keystroke {
                key: Key::Space,
                modifiers: Modifiers::CTRL,
            },
            &[],
        );
        assert!(result.is_none());
    }

    #[test]
    fn keymap_no_match_wrong_modifier() {
        let mut keymap = Keymap::new();
        keymap.bind(
            Keystroke {
                key: Key::S,
                modifiers: Modifiers::CTRL,
            },
            None,
            TestAction,
        );
        let result = keymap.match_keystroke(
            &Keystroke {
                key: Key::S,
                modifiers: Modifiers::SUPER,
            },
            &[],
        );
        assert!(result.is_none());
    }

    #[test]
    fn keymap_context_filter() {
        let mut keymap = Keymap::new();
        keymap.bind(
            Keystroke {
                key: Key::Enter,
                modifiers: Modifiers::CTRL,
            },
            Some("Editor"),
            TestAction,
        );

        let no_match = keymap.match_keystroke(
            &Keystroke {
                key: Key::Enter,
                modifiers: Modifiers::CTRL,
            },
            &["Terminal".into()],
        );
        assert!(no_match.is_none());

        let matched = keymap.match_keystroke(
            &Keystroke {
                key: Key::Enter,
                modifiers: Modifiers::CTRL,
            },
            &["Editor".into()],
        );
        assert!(matched.is_some());
    }

    #[test]
    fn keymap_last_binding_wins() {
        let mut keymap = Keymap::new();
        let ks = Keystroke {
            key: Key::S,
            modifiers: Modifiers::CTRL,
        };
        keymap.bind(ks.clone(), None, TestAction);
        keymap.bind(ks.clone(), None, OtherAction);

        let result = keymap.match_keystroke(&ks, &[]);
        assert!(result.is_some());
        assert_eq!(result.unwrap().name(), "OtherAction");
    }

    #[test]
    fn keymap_context_binding_over_global() {
        let mut keymap = Keymap::new();
        let ks = Keystroke {
            key: Key::Enter,
            modifiers: Modifiers::empty(),
        };
        keymap.bind(ks.clone(), None, TestAction);
        keymap.bind(ks.clone(), Some("Editor"), OtherAction);

        let result = keymap.match_keystroke(&ks, &["Editor".into()]);
        assert!(result.is_some());
        assert_eq!(result.unwrap().name(), "OtherAction");
    }

    #[test]
    fn action_registry_dispatch() {
        let node = test_node_id(1);
        let mut registry = ActionRegistry::new();
        let called = Rc::new(Cell::new(false));
        let flag = called.clone();
        registry.register::<TestAction>(node, move |_| {
            flag.set(true);
        });

        let dispatched = registry.dispatch(node, &TestAction);
        assert!(dispatched);
        assert!(called.get());
    }

    #[test]
    fn action_registry_wrong_node() {
        let node_a = test_node_id(1);
        let node_b = test_node_id(2);
        let mut registry = ActionRegistry::new();
        registry.register::<TestAction>(node_a, |_| {});

        let dispatched = registry.dispatch(node_b, &TestAction);
        assert!(!dispatched);
    }

    #[test]
    fn action_registry_wrong_type() {
        let node = test_node_id(1);
        let mut registry = ActionRegistry::new();
        registry.register::<TestAction>(node, |_| {});

        let dispatched = registry.dispatch(node, &OtherAction);
        assert!(!dispatched);
    }

    #[test]
    fn action_registry_clear() {
        let node = test_node_id(1);
        let mut registry = ActionRegistry::new();
        registry.register::<TestAction>(node, |_| {});
        registry.clear();

        let dispatched = registry.dispatch(node, &TestAction);
        assert!(!dispatched);
    }

    #[test]
    fn any_action_boxed_clone() {
        let action: Box<dyn AnyAction> = Box::new(TestAction);
        let cloned = action.boxed_clone();
        assert_eq!(cloned.name(), "TestAction");
        assert_eq!(cloned.action_type_id(), TypeId::of::<TestAction>());
    }
}
