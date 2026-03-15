mod bear;
mod cat;
mod dog;
mod owl;
mod penguin;
mod rabbit;
mod shark;

pub struct AnimalDef {
    pub frames: &'static [&'static [&'static str]],
    /// Animation sequence: (frame_index, duration_seconds)
    pub sequence: &'static [(usize, f64)],
    pub name: &'static str,
    pub color_top: (u8, u8, u8),
    pub color_bot: (u8, u8, u8),
}

pub const ANIMAL_DEFS: &[AnimalDef] = &[
    shark::DEF,
    cat::DEF,
    dog::DEF,
    rabbit::DEF,
    penguin::DEF,
    owl::DEF,
    bear::DEF,
];
