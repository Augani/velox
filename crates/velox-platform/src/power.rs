#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BatteryState {
    Unknown,
    Unplugged(f32),
    Charging(f32),
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PowerSource {
    Battery,
    AC,
    Unknown,
}

pub trait PlatformPower {
    fn battery_state(&self) -> BatteryState;
    fn power_source(&self) -> PowerSource;
    fn is_low_power_mode(&self) -> bool;
}
