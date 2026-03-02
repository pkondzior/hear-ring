use std::f32::consts::PI;

pub const SECTOR_COUNT: usize = 8;
pub const ORDERED_SECTORS: [Sector8; SECTOR_COUNT] = [
    Sector8::F,
    Sector8::FR,
    Sector8::R,
    Sector8::BR,
    Sector8::B,
    Sector8::BL,
    Sector8::L,
    Sector8::FL,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelLayout {
    Stereo,
    Surround71,
}

impl ChannelLayout {
    pub fn label(self) -> &'static str {
        match self {
            ChannelLayout::Stereo => "Stereo",
            ChannelLayout::Surround71 => "7.1",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sector8 {
    F,
    FR,
    R,
    BR,
    B,
    BL,
    L,
    FL,
}

impl Sector8 {
    pub fn label(self) -> &'static str {
        match self {
            Sector8::F => "F",
            Sector8::FR => "FR",
            Sector8::R => "R",
            Sector8::BR => "BR",
            Sector8::B => "B",
            Sector8::BL => "BL",
            Sector8::L => "L",
            Sector8::FL => "FL",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Sector8::F => 0,
            Sector8::FR => 1,
            Sector8::R => 2,
            Sector8::BR => 3,
            Sector8::B => 4,
            Sector8::BL => 5,
            Sector8::L => 6,
            Sector8::FL => 7,
        }
    }

    pub fn angle(self) -> f32 {
        match self {
            Sector8::F => -PI / 2.0,
            Sector8::FR => -PI / 4.0,
            Sector8::R => 0.0,
            Sector8::BR => PI / 4.0,
            Sector8::B => PI / 2.0,
            Sector8::BL => 3.0 * PI / 4.0,
            Sector8::L => PI,
            Sector8::FL => -3.0 * PI / 4.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ChannelEnergies {
    pub fl: f32,
    pub fr: f32,
    pub c: f32,
    pub lfe: f32,
    pub sl: f32,
    pub sr: f32,
    pub rl: f32,
    pub rr: f32,
}

impl ChannelEnergies {
    pub fn total_directional(self) -> f32 {
        self.fl + self.fr + self.c + self.sl + self.sr + self.rl + self.rr
    }

    pub fn total_with_lfe(self) -> f32 {
        self.total_directional() + self.lfe
    }
}

#[derive(Debug, Clone)]
pub struct DirectionFrame {
    pub scores: [f32; SECTOR_COUNT],
    pub confidence: f32,
    pub intensity: f32,
    pub active: bool,
}

impl DirectionFrame {
    pub fn empty() -> Self {
        Self {
            scores: [0.0; SECTOR_COUNT],
            confidence: 0.0,
            intensity: 0.0,
            active: false,
        }
    }

    pub fn dominant_sector(&self) -> Option<Sector8> {
        let (index, value) = self
            .scores
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())?;

        if *value <= 0.01 {
            return None;
        }

        Some(ORDERED_SECTORS[index])
    }
}
