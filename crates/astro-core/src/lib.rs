//! Cálculo de mapa astral em Rust puro.
//!
//! Fontes: VSOP87 (planetas) e ELP-2000/82 truncada (Lua) via crate `astro`;
//! Plutão e Quíron por tabelas do JPL Horizons interpoladas (1800–2200).
//! Todas as longitudes públicas estão em graus, no intervalo [0, 360).

pub mod aspects;
pub mod chart;
pub mod engine;
pub mod ephem;
pub mod houses;
#[cfg(feature = "swisseph")]
pub mod swiss;
pub mod time;
pub mod transits;
pub mod zodiac;

pub use aspects::{Aspect, AspectKind, Orbs};
pub use chart::{compute_chart, Chart, ChartInput};
pub use engine::{Engine, Ephemeris};
pub use ephem::{Body, EphemError, Position};
pub use houses::{HouseSystem, Houses};
pub use transits::{compute_transits, TransitHit, Transits};
pub use zodiac::{NodeType, Zodiac};

/// Normaliza um ângulo em graus para [0, 360).
pub fn norm360(x: f64) -> f64 {
    let r = x % 360.0;
    if r < 0.0 {
        r + 360.0
    } else {
        r
    }
}

/// Diferença angular com sinal em (-180, 180].
pub fn wrap180(x: f64) -> f64 {
    let r = norm360(x);
    if r > 180.0 {
        r - 360.0
    } else {
        r
    }
}
