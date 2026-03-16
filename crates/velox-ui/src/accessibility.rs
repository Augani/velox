use velox_scene::{AccessibilityActionSupport, AccessibilityNode, AccessibilityRole};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccessibilityProps {
    pub role: Option<AccessibilityRole>,
    pub label: Option<String>,
    pub value: Option<String>,
    pub disabled: Option<bool>,
    pub supported_actions: Vec<AccessibilityActionSupport>,
}

impl AccessibilityProps {
    pub fn is_empty(&self) -> bool {
        self.role.is_none()
            && self.label.is_none()
            && self.value.is_none()
            && self.disabled.is_none()
            && self.supported_actions.is_empty()
    }

    pub fn resolve(
        &self,
        default_role: AccessibilityRole,
        default_label: Option<String>,
        default_value: Option<String>,
        default_disabled: bool,
    ) -> AccessibilityNode {
        let mut node = AccessibilityNode::new(self.role.unwrap_or(default_role));
        node.label = self.label.clone().or(default_label);
        node.value = self.value.clone().or(default_value);
        node.disabled = self.disabled.unwrap_or(default_disabled);
        node.supported_actions = self.supported_actions.clone();
        node
    }
}

pub trait AccessibleElement: Sized {
    fn accessibility_props_mut(&mut self) -> &mut AccessibilityProps;

    fn accessibility_role(mut self, role: AccessibilityRole) -> Self {
        self.accessibility_props_mut().role = Some(role);
        self
    }

    fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_props_mut().label = Some(label.into());
        self
    }

    fn accessibility_value(mut self, value: impl Into<String>) -> Self {
        self.accessibility_props_mut().value = Some(value.into());
        self
    }

    fn accessibility_disabled(mut self, disabled: bool) -> Self {
        self.accessibility_props_mut().disabled = Some(disabled);
        self
    }

    fn accessibility_supports_action(mut self, action: AccessibilityActionSupport) -> Self {
        let props = self.accessibility_props_mut();
        if !props.supported_actions.contains(&action) {
            props.supported_actions.push(action);
        }
        self
    }

    fn accessibility_supports_focus_actions(self) -> Self {
        self.accessibility_supports_actions([
            AccessibilityActionSupport::Focus,
            AccessibilityActionSupport::Blur,
        ])
    }

    fn accessibility_supports_click_action(self) -> Self {
        self.accessibility_supports_action(AccessibilityActionSupport::Click)
    }

    fn accessibility_supports_actions(
        mut self,
        actions: impl IntoIterator<Item = AccessibilityActionSupport>,
    ) -> Self {
        let props = self.accessibility_props_mut();
        for action in actions {
            if !props.supported_actions.contains(&action) {
                props.supported_actions.push(action);
            }
        }
        self
    }

    fn accessibility_supports_text_input_actions(self) -> Self {
        self.accessibility_supports_focus_actions()
            .accessibility_supports_actions([
                AccessibilityActionSupport::SetTextSelection,
                AccessibilityActionSupport::SetValue,
                AccessibilityActionSupport::ReplaceSelectedText,
            ])
    }
}
