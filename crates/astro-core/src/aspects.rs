//! Aspectos maiores entre pontos do mapa.

use crate::wrap180;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AspectKind {
    Conjunction,
    Sextile,
    Square,
    Trine,
    Opposition,
}

impl AspectKind {
    pub const ALL: [AspectKind; 5] = [
        AspectKind::Conjunction,
        AspectKind::Sextile,
        AspectKind::Square,
        AspectKind::Trine,
        AspectKind::Opposition,
    ];

    pub fn angle(self) -> f64 {
        match self {
            AspectKind::Conjunction => 0.0,
            AspectKind::Sextile => 60.0,
            AspectKind::Square => 90.0,
            AspectKind::Trine => 120.0,
            AspectKind::Opposition => 180.0,
        }
    }
}

/// Um ponto que pode formar aspectos.
#[derive(Debug, Clone)]
pub struct AspectPoint {
    pub key: String,
    pub lon: f64,
    pub speed: f64,
    /// Sol e Lua aceitam orbe maior.
    pub luminary: bool,
    /// Ascendente e Meio do Céu: não formam aspecto entre si.
    pub angle: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aspect {
    pub a: String,
    pub b: String,
    pub kind: AspectKind,
    /// Distância até o aspecto exato, em graus.
    pub orb: f64,
    /// `true` se o aspecto está se formando; `None` quando envolve um ângulo.
    pub applying: Option<bool>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Orbs {
    pub luminaries: f64,
    pub others: f64,
}

impl Default for Orbs {
    fn default() -> Self {
        Orbs {
            luminaries: 8.0,
            others: 6.0,
        }
    }
}

/// Lista os aspectos, do mais exato ao menos exato.
pub fn find(points: &[AspectPoint], orbs: Orbs) -> Vec<Aspect> {
    let mut out = vec![];
    for (i, a) in points.iter().enumerate() {
        for b in &points[i + 1..] {
            if a.angle && b.angle {
                continue;
            }
            let max = if a.luminary || b.luminary {
                orbs.luminaries
            } else {
                orbs.others
            };
            let sep = wrap180(b.lon - a.lon).abs();
            for kind in AspectKind::ALL {
                let orb = (sep - kind.angle()).abs();
                if orb <= max {
                    let applying = if a.angle || b.angle {
                        None
                    } else {
                        // Separação um instante depois: se o orbe diminui, o aspecto se aplica.
                        let dt = 0.01;
                        let later = wrap180((b.lon + b.speed * dt) - (a.lon + a.speed * dt)).abs();
                        Some((later - kind.angle()).abs() < orb)
                    };
                    out.push(Aspect {
                        a: a.key.clone(),
                        b: b.key.clone(),
                        kind,
                        orb,
                        applying,
                    });
                }
            }
        }
    }
    out.sort_by(|x, y| x.orb.total_cmp(&y.orb));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(key: &str, lon: f64, speed: f64, luminary: bool) -> AspectPoint {
        AspectPoint {
            key: key.into(),
            lon,
            speed,
            luminary,
            angle: false,
        }
    }

    #[test]
    fn finds_and_sorts() {
        let pts = [
            p("sun", 10.0, 1.0, true),
            p("mars", 97.0, 0.5, false),
            p("saturn", 190.5, 0.03, false),
            p("venus", 355.0, 1.2, false),
        ];
        let a = find(&pts, Orbs::default());
        assert_eq!(a[0].kind, AspectKind::Opposition); // sol–saturno, orbe 0.5
        assert!((a[0].orb - 0.5).abs() < 1e-9);
        assert_eq!(a[0].applying, Some(true)); // o Sol, mais rápido, ainda vai chegar à oposição exata
        assert!(a
            .iter()
            .any(|x| x.a == "sun" && x.b == "mars" && x.kind == AspectKind::Square && (x.orb - 3.0).abs() < 1e-9));
        // Vênus a 15° do Sol: fora do orbe de conjunção.
        assert!(!a.iter().any(|x| x.b == "venus" && x.kind == AspectKind::Conjunction));
        // Marte–Saturno: 93.5°, quadratura com orbe 3.5 (dentro de 6).
        assert!(a
            .iter()
            .any(|x| x.a == "mars" && x.b == "saturn" && x.kind == AspectKind::Square));
    }
}
