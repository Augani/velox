#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PowerClass {
    Essential,
    Interactive,
    Decorative,
    Background,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PowerPolicy {
    #[default]
    Performance,
    Adaptive,
    Saving,
}

impl PowerPolicy {
    pub fn should_run(&self, class: PowerClass) -> bool {
        match self {
            PowerPolicy::Performance => true,
            PowerPolicy::Adaptive => !matches!(class, PowerClass::Background),
            PowerPolicy::Saving => matches!(class, PowerClass::Essential | PowerClass::Interactive),
        }
    }
}
