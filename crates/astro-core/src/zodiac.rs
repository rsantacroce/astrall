//! Signos, zodíaco tropical/sideral e opções de cálculo.

use serde::{Deserialize, Serialize};

pub const SIGNS: [&str; 12] = [
    "Áries",
    "Touro",
    "Gêmeos",
    "Câncer",
    "Leão",
    "Virgem",
    "Libra",
    "Escorpião",
    "Sagitário",
    "Capricórnio",
    "Aquário",
    "Peixes",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Zodiac {
    #[default]
    Tropical,
    /// Sideral com ayanamsa Lahiri.
    SiderealLahiri,
    /// Sideral com ayanamsa Fagan-Bradley.
    SiderealFaganBradley,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    #[default]
    True,
    Mean,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Element {
    Fire,
    Earth,
    Air,
    Water,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    Cardinal,
    Fixed,
    Mutable,
}

/// Índice do signo (0 = Áries) de uma longitude em graus.
pub fn sign_index(lon: f64) -> usize {
    ((crate::norm360(lon) / 30.0).floor() as usize).min(11)
}

pub fn element(sign: usize) -> Element {
    [Element::Fire, Element::Earth, Element::Air, Element::Water][sign % 4]
}

pub fn modality(sign: usize) -> Modality {
    [Modality::Cardinal, Modality::Fixed, Modality::Mutable][sign % 3]
}

/// Ayanamsa média em graus (valor em J2000 + precessão geral em longitude, IAU 2006).
pub fn ayanamsa(zodiac: Zodiac, jde: f64) -> f64 {
    let t = crate::time::centuries(jde);
    let j2000 = match zodiac {
        Zodiac::Tropical => return 0.0,
        Zodiac::SiderealLahiri => 23.85705,
        Zodiac::SiderealFaganBradley => 24.74041,
    };
    j2000 + (5028.796195 * t + 1.1054348 * t * t) / 3600.0
}
