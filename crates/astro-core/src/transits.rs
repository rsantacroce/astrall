//! Trânsitos dos planetas lentos sobre os pontos natais, com datas exatas.

use crate::aspects::AspectKind;
use crate::chart::{body_key, ChartInput};
use crate::engine::WithFallback;
use crate::ephem::{self, Body, EphemError};
use crate::houses;
use crate::{norm360, time, wrap180};
use serde::{Deserialize, Serialize};

pub const SLOW: [Body; 5] = [Body::Jupiter, Body::Saturn, Body::Uranus, Body::Neptune, Body::Pluto];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitHit {
    /// Instante do aspecto exato, dia juliano UT.
    pub jd_ut: f64,
    pub transiting: String,
    pub natal: String,
    pub kind: AspectKind,
    /// O planeta em trânsito está retrógrado no momento exato.
    pub retrograde: bool,
    /// Número desta passagem (1, 2, 3…) e total de passagens no período.
    pub pass: u8,
    pub passes: u8,
}

/// Aspecto de trânsito dentro do orbe no início do período.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveTransit {
    pub transiting: String,
    pub natal: String,
    pub kind: AspectKind,
    pub orb: f64,
    pub retrograde: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transits {
    pub start_jd_ut: f64,
    pub days: u32,
    pub hits: Vec<TransitHit>,
    pub active: Vec<ActiveTransit>,
}

/// Pontos natais (chave, longitude tropical) usados como alvo dos trânsitos.
fn natal_points(input: &ChartInput, eph: &WithFallback) -> Result<Vec<(String, f64)>, EphemError> {
    let mut pts = vec![];
    for b in Body::ALL {
        pts.push((body_key(b).to_string(), eph.longitude(b, input.jd_ut, input.node_type)?));
    }
    if input.time_known {
        let (eps, _) = ephem::obliquity_and_nutation(time::jde(input.jd_ut));
        let armc = norm360(ephem::greenwich_sidereal_deg(input.jd_ut) + input.lon);
        let h = houses::compute(input.house_system, armc, eps, input.lat);
        pts.push(("asc".into(), h.asc));
        pts.push(("mc".into(), h.mc));
    }
    Ok(pts)
}

/// Procura, dia a dia, os cruzamentos exatos e refina por bisseção (precisão de segundos).
pub fn compute_transits(input: &ChartInput, start_jd_ut: f64, days: u32) -> Result<Transits, EphemError> {
    let eph = WithFallback::new(input.engine)?;
    let natal = natal_points(input, &eph)?;
    let node = input.node_type;
    let mut hits = vec![];
    let mut active = vec![];

    for body in SLOW {
        let lons: Vec<f64> = (0..=days)
            .map(|i| eph.longitude(body, start_jd_ut + i as f64, node))
            .collect::<Result<_, _>>()?;
        let retro_at_start = eph.position(body, start_jd_ut, node)?.retrograde;
        for (key, natal_lon) in &natal {
            for kind in AspectKind::ALL {
                let offsets: &[f64] = match kind {
                    AspectKind::Conjunction | AspectKind::Opposition => &[1.0],
                    _ => &[1.0, -1.0],
                };
                let mut group = vec![];
                for sign in offsets {
                    let target = if kind.angle() == 0.0 || kind.angle() == 180.0 {
                        natal_lon + kind.angle()
                    } else {
                        natal_lon + sign * kind.angle()
                    };
                    let g = |lon: f64| wrap180(lon - target);
                    if g(lons[0]).abs() <= 1.0 {
                        active.push(ActiveTransit {
                            transiting: body_key(body).into(),
                            natal: key.clone(),
                            kind,
                            orb: g(lons[0]).abs(),
                            retrograde: retro_at_start,
                        });
                    }
                    for i in 0..days as usize {
                        let (a, b) = (g(lons[i]), g(lons[i + 1]));
                        if a == 0.0 || a * b > 0.0 || a.abs() > 90.0 || b.abs() > 90.0 {
                            continue;
                        }
                        let (mut lo, mut hi, mut glo) = (start_jd_ut + i as f64, start_jd_ut + i as f64 + 1.0, a);
                        for _ in 0..18 {
                            let mid = 0.5 * (lo + hi);
                            let gm = g(eph.longitude(body, mid, node)?);
                            if gm * glo <= 0.0 {
                                hi = mid;
                            } else {
                                lo = mid;
                                glo = gm;
                            }
                        }
                        let jd = 0.5 * (lo + hi);
                        group.push((jd, eph.position(body, jd, node)?.retrograde));
                    }
                }
                group.sort_by(|x, y| x.0.total_cmp(&y.0));
                let passes = group.len() as u8;
                for (n, (jd, retrograde)) in group.into_iter().enumerate() {
                    hits.push(TransitHit {
                        jd_ut: jd,
                        transiting: body_key(body).into(),
                        natal: key.clone(),
                        kind,
                        retrograde,
                        pass: n as u8 + 1,
                        passes,
                    });
                }
            }
        }
    }
    hits.sort_by(|a, b| a.jd_ut.total_cmp(&b.jd_ut));
    active.sort_by(|a, b| a.orb.total_cmp(&b.orb));
    Ok(Transits {
        start_jd_ut,
        days,
        hits,
        active,
    })
}
