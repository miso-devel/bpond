mod cat;
mod dog;
mod fish;
mod rabbit;

pub struct AnimalDef {
    pub art_a: &'static [&'static str],
    pub art_b: &'static [&'static str],
    pub name: &'static str,
    pub color_top: (u8, u8, u8),
    pub color_bot: (u8, u8, u8),
}

pub const ANIMAL_DEFS: &[AnimalDef] = &[
    cat::DEF,
    dog::DEF,
    fish::DEF,
    rabbit::DEF,
];
