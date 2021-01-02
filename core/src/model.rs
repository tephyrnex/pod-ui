use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Config {
    pub name: String,
    pub family: u16,
    pub member: u16,

    pub program_size: usize,
    pub all_programs_size: usize,
    pub pod_id: u8, // used in sysex dump messages

    pub amp_models: Vec<Amp>,
    pub cab_models: Vec<String>,
    pub effects: Vec<Effect>,
    pub controls: HashMap<String, Control>
}

#[derive(Clone, Default, Debug)]
pub struct Amp {
    pub name: String,
    pub bright_switch: bool,
    pub presence: bool,
    pub delay2: bool,
}

#[derive(Clone, Default, Debug)]
pub struct Effect {
    pub name: String,
    pub delay: Option<bool>,
}


#[derive(Clone, Debug)]
pub enum Control {
    SwitchControl(SwitchControl),
    RangeControl(RangeControl),
    Select(Select)
}

#[derive(Clone, Debug)]
pub struct SwitchControl { pub cc: u8 }
#[derive(Clone, Debug)]
pub struct RangeControl { pub cc: u8, pub from: u8, pub to: u8 }
#[derive(Clone, Debug)]
pub struct Select { pub cc: u8 }

impl From<SwitchControl> for Control {
    fn from(c: SwitchControl) -> Self {
        Control::SwitchControl(c)
    }
}

impl Default for RangeControl {
    fn default() -> Self {
        RangeControl { cc: 0, from: 0, to: 127 }
    }
}
impl From<RangeControl> for Control {
    fn from(c: RangeControl) -> Self {
        Control::RangeControl(c)
    }
}

impl From<Select> for Control {
    fn from(c: Select) -> Self {
        Control::Select(c)
    }
}

pub trait GetCC {
    fn get_cc(&self) -> Option<u8>;
}

impl GetCC for RangeControl {
    fn get_cc(&self) -> Option<u8> { Some(self.cc) }
}

impl GetCC for SwitchControl {
    fn get_cc(&self) -> Option<u8> { Some(self.cc) }
}

impl GetCC for Select {
    fn get_cc(&self) -> Option<u8> { Some(self.cc) }
}

impl GetCC for Control {
    fn get_cc(&self) -> Option<u8> {
        let cc: &GetCC = match self {
            Control::SwitchControl(c) => c,
            Control::RangeControl(c) => c,
            Control::Select(c) => c
        };
        cc.get_cc()

    }
}
