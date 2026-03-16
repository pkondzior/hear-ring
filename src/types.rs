use std::{
    f32::consts::PI,
    ops::{Index, IndexMut},
};

pub const DIRECTION_COUNT: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyChannel {
    FL,
    FR,
    C,
    SL,
    SR,
    RL,
    RR,
    Lfe,
}

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

impl EnergyChannel {
    pub const ALL: [Self; 8] = [
        Self::FL,
        Self::FR,
        Self::C,
        Self::SL,
        Self::SR,
        Self::RL,
        Self::RR,
        Self::Lfe,
    ];

    pub fn id(self) -> &'static str {
        match self {
            EnergyChannel::FL => "energy-fl",
            EnergyChannel::FR => "energy-fr",
            EnergyChannel::C => "energy-c",
            EnergyChannel::SL => "energy-sl",
            EnergyChannel::SR => "energy-sr",
            EnergyChannel::RL => "energy-rl",
            EnergyChannel::RR => "energy-rr",
            EnergyChannel::Lfe => "energy-lfe",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            EnergyChannel::FL => "FL",
            EnergyChannel::FR => "FR",
            EnergyChannel::C => "C",
            EnergyChannel::SL => "SL",
            EnergyChannel::SR => "SR",
            EnergyChannel::RL => "RL",
            EnergyChannel::RR => "RR",
            EnergyChannel::Lfe => "LFE",
        }
    }

    pub fn value(self, energies: &ChannelEnergies) -> f32 {
        match self {
            EnergyChannel::FL => energies.fl,
            EnergyChannel::FR => energies.fr,
            EnergyChannel::C => energies.c,
            EnergyChannel::SL => energies.sl,
            EnergyChannel::SR => energies.sr,
            EnergyChannel::RL => energies.rl,
            EnergyChannel::RR => energies.rr,
            EnergyChannel::Lfe => energies.lfe,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Direction {
    F,
    FR,
    R,
    BR,
    B,
    BL,
    L,
    FL,
}

impl Direction {
    pub const ALL: [Self; DIRECTION_COUNT] = [
        Self::F,
        Self::FR,
        Self::R,
        Self::BR,
        Self::B,
        Self::BL,
        Self::L,
        Self::FL,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Direction::F => "F",
            Direction::FR => "FR",
            Direction::R => "R",
            Direction::BR => "BR",
            Direction::B => "B",
            Direction::BL => "BL",
            Direction::L => "L",
            Direction::FL => "FL",
        }
    }

    pub fn angle(self) -> f32 {
        match self {
            Direction::F => -PI / 2.0,
            Direction::FR => -PI / 4.0,
            Direction::R => 0.0,
            Direction::BR => PI / 4.0,
            Direction::B => PI / 2.0,
            Direction::BL => 3.0 * PI / 4.0,
            Direction::L => PI,
            Direction::FL => -3.0 * PI / 4.0,
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
    /// Stereo-only cue from raw PCM.
    /// -1.0 = hard left, 0.0 = center, +1.0 = hard right.
    pub stereo_pan: f32,
    /// 0.0 = center/mono-heavy, 1.0 = wide side-heavy content.
    pub stereo_width: f32,
}

impl ChannelEnergies {
    pub fn total_directional(self) -> f32 {
        self.fl + self.fr + self.c + self.sl + self.sr + self.rl + self.rr
    }

    pub fn total_with_lfe(self) -> f32 {
        self.total_directional() + self.lfe
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DirectionScores([f32; DIRECTION_COUNT]);

impl DirectionScores {
    pub fn iter(&self) -> impl Iterator<Item = &f32> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut f32> {
        self.0.iter_mut()
    }
}

impl Default for DirectionScores {
    fn default() -> Self {
        Self([0.0; DIRECTION_COUNT])
    }
}

impl From<[f32; DIRECTION_COUNT]> for DirectionScores {
    fn from(scores: [f32; DIRECTION_COUNT]) -> Self {
        Self(scores)
    }
}

impl Index<Direction> for DirectionScores {
    type Output = f32;

    fn index(&self, index: Direction) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl IndexMut<Direction> for DirectionScores {
    fn index_mut(&mut self, index: Direction) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}

#[derive(Debug, Clone)]
pub struct DirectionFrame {
    pub scores: DirectionScores,
    pub confidence: f32,
    pub intensity: f32,
    pub active: bool,
}

impl DirectionFrame {
    pub fn empty() -> Self {
        Self {
            scores: DirectionScores::default(),
            confidence: 0.0,
            intensity: 0.0,
            active: false,
        }
    }

    pub fn dominant_direction(&self) -> Option<Direction> {
        let (index, value) = self
            .scores
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())?;

        if *value <= 0.01 {
            return None;
        }

        Some(Direction::ALL[index])
    }
}
