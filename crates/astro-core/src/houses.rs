//! Ascendente, Meio do Céu e cúspides das casas.

use crate::norm360;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum HouseSystem {
    #[default]
    Placidus,
    Regiomontanus,
    Porphyry,
    Equal,
    WholeSign,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Houses {
    /// Sistema efetivamente usado (pode diferir do pedido em latitudes polares).
    pub system: HouseSystem,
    pub asc: f64,
    pub mc: f64,
    /// Cúspides das casas 1 a 12, em graus.
    pub cusps: [f64; 12],
    /// Placidus não é definido acima dos círculos polares; nesse caso cai para Porphyry.
    pub fallback: bool,
}

/// `armc_deg`: ascensão reta do Meio do Céu (tempo sideral local) em graus;
/// `eps`: obliquidade verdadeira em radianos; `lat_deg`: latitude geográfica.
pub fn compute(system: HouseSystem, armc_deg: f64, eps: f64, lat_deg: f64) -> Houses {
    let armc = armc_deg.to_radians();
    let phi = lat_deg.to_radians();
    let mc = ecliptic_from_ra(armc, eps);
    let mut asc = pole_cusp(armc + std::f64::consts::FRAC_PI_2, phi.tan(), eps);
    // O Ascendente é o ponto que nasce: fica no semicírculo oriental, depois do MC.
    if norm360(asc - mc) > 180.0 {
        asc = norm360(asc + 180.0);
    }

    let mut used = system;
    let mut fallback = false;
    let mut cusps = [0.0; 12];
    cusps[0] = asc;
    cusps[9] = mc;

    let quadrant: Option<[f64; 4]> = match system {
        HouseSystem::Placidus => placidus(armc, eps, phi).or_else(|| {
            fallback = true;
            used = HouseSystem::Porphyry;
            Some(porphyry(asc, mc))
        }),
        HouseSystem::Regiomontanus => Some(regiomontanus(armc, eps, phi)),
        HouseSystem::Porphyry => Some(porphyry(asc, mc)),
        HouseSystem::Equal | HouseSystem::WholeSign => None,
    };

    match quadrant {
        Some([c11, c12, c2, c3]) => {
            cusps[10] = c11;
            cusps[11] = c12;
            cusps[1] = c2;
            cusps[2] = c3;
            for i in 0..3 {
                cusps[3 + i] = norm360(cusps[9 + i] + 180.0);
                cusps[6 + i] = norm360(cusps[i] + 180.0);
            }
        }
        None => {
            let start = if system == HouseSystem::WholeSign {
                (asc / 30.0).floor() * 30.0
            } else {
                asc
            };
            for (i, c) in cusps.iter_mut().enumerate() {
                *c = norm360(start + 30.0 * i as f64);
            }
        }
    }
    Houses {
        system: used,
        asc,
        mc,
        cusps,
        fallback,
    }
}

/// Casa (1–12) em que cai uma longitude.
pub fn house_of(lon: f64, cusps: &[f64; 12]) -> u8 {
    for i in 0..12 {
        let width = norm360(cusps[(i + 1) % 12] - cusps[i]);
        if norm360(lon - cusps[i]) < width {
            return i as u8 + 1;
        }
    }
    1
}

/// Longitude do ponto da eclíptica com ascensão reta `ra`.
fn ecliptic_from_ra(ra: f64, eps: f64) -> f64 {
    norm360(ra.sin().atan2(ra.cos() * eps.cos()).to_degrees())
}

/// Ponto da eclíptica que cruza o círculo de posição de "polo" `tan_pole`, para a ascensão
/// reta `ra` medida sobre o equador. Com o polo igual à latitude e `ra = ARMC + 90°` dá o
/// Ascendente; com polo zero dá o ponto que culmina.
fn pole_cusp(ra: f64, tan_pole: f64, eps: f64) -> f64 {
    norm360(ra.sin().atan2(ra.cos() * eps.cos() - tan_pole * eps.sin()).to_degrees())
}

/// Placidus: cada cúspide intermediária divide em terços o semiarco (diurno ou noturno)
/// do próprio ponto. Resolve por iteração; `None` quando o ponto é circumpolar.
fn placidus(armc: f64, eps: f64, phi: f64) -> Option<[f64; 4]> {
    use std::f64::consts::PI;
    let solve = |frac: f64, below: bool| -> Option<f64> {
        let mut ra = if below {
            armc + PI - frac * PI / 2.0
        } else {
            armc + frac * PI / 2.0
        };
        for _ in 0..100 {
            let decl = (eps.tan() * ra.sin()).atan();
            let x = phi.tan() * decl.tan();
            if x.abs() > 1.0 {
                return None;
            }
            let next = if below {
                armc + PI - frac * x.acos()
            } else {
                armc + frac * (-x).acos()
            };
            let done = (next - ra).abs() < 1e-11;
            ra = next;
            if done {
                break;
            }
        }
        Some(ecliptic_from_ra(ra, eps))
    };
    Some([
        solve(1.0 / 3.0, false)?,
        solve(2.0 / 3.0, false)?,
        solve(2.0 / 3.0, true)?,
        solve(1.0 / 3.0, true)?,
    ])
}

/// Regiomontanus: o equador é dividido em arcos de 30° a partir do meridiano.
fn regiomontanus(armc: f64, eps: f64, phi: f64) -> [f64; 4] {
    let cusp = |h_deg: f64| {
        let h = h_deg.to_radians();
        pole_cusp(armc + h, phi.tan() * h.sin(), eps)
    };
    [cusp(30.0), cusp(60.0), cusp(120.0), cusp(150.0)]
}

/// Porphyry: cada quadrante da eclíptica é dividido em três partes iguais.
fn porphyry(asc: f64, mc: f64) -> [f64; 4] {
    let q1 = norm360(asc - mc) / 3.0; // MC → ASC
    let q2 = norm360(mc + 180.0 - asc) / 3.0; // ASC → IC
    [
        norm360(mc + q1),
        norm360(mc + 2.0 * q1),
        norm360(asc + q2),
        norm360(asc + 2.0 * q2),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wrap180;
    use std::f64::consts::PI;

    const EPS: f64 = 0.409_092_8; // 23.4393°

    /// Ascensão reta e declinação de um ponto da eclíptica (caminho independente do cálculo).
    fn equatorial(lon_deg: f64) -> (f64, f64) {
        let l = lon_deg.to_radians();
        ((l.sin() * EPS.cos()).atan2(l.cos()), (EPS.sin() * l.sin()).asin())
    }

    fn altitude(lon_deg: f64, armc: f64, phi: f64) -> f64 {
        let (ra, dec) = equatorial(lon_deg);
        let h = armc - ra;
        (phi.sin() * dec.sin() + phi.cos() * dec.cos() * h.cos()).asin()
    }

    #[test]
    fn ascendant_is_on_the_eastern_horizon() {
        for &lat in &[-60.0_f64, -23.55, 0.0, 35.0, 51.5, 64.0] {
            for armc in (0..360).step_by(17) {
                let h = compute(HouseSystem::Placidus, armc as f64, EPS, lat);
                let a = (armc as f64).to_radians();
                let phi = lat.to_radians();
                assert!(
                    altitude(h.asc, a, phi).abs() < 1e-9,
                    "ASC fora do horizonte: lat {lat} armc {armc}"
                );
                // Nascendo: um pouco depois (ARMC maior) o ponto está acima do horizonte.
                assert!(
                    altitude(h.asc, a + 1e-4, phi) > 0.0,
                    "ASC se pondo: lat {lat} armc {armc}"
                );
                // O MC culmina: sua ascensão reta é o ARMC.
                assert!(wrap180((equatorial(h.mc).0 - a).to_degrees()).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn placidus_cusps_trisect_semi_arcs() {
        for &lat in &[-45.0_f64, -23.55, 10.0, 40.7, 55.0] {
            for armc_deg in (0..360).step_by(23) {
                let armc = (armc_deg as f64).to_radians();
                let phi = lat.to_radians();
                let h = compute(HouseSystem::Placidus, armc_deg as f64, EPS, lat);
                assert!(!h.fallback);
                // (casa, fração do semiarco, abaixo do horizonte)
                for &(house, frac, below) in &[
                    (11, 1.0 / 3.0, false),
                    (12, 2.0 / 3.0, false),
                    (2, 2.0 / 3.0, true),
                    (3, 1.0 / 3.0, true),
                ] {
                    let (ra, dec) = equatorial(h.cusps[house - 1]);
                    let diurnal = (-phi.tan() * dec.tan()).acos();
                    let (dist, arc) = if below {
                        (wrap180((armc + PI - ra).to_degrees()).to_radians(), PI - diurnal)
                    } else {
                        (wrap180((ra - armc).to_degrees()).to_radians(), diurnal)
                    };
                    assert!(
                        (dist - frac * arc).abs() < 1e-8,
                        "casa {house} lat {lat} armc {armc_deg}"
                    );
                }
                // Cúspides em ordem zodiacal.
                let total: f64 = (0..12).map(|i| norm360(h.cusps[(i + 1) % 12] - h.cusps[i])).sum();
                assert!((total - 360.0).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn regiomontanus_matches_angles_and_orders_cusps() {
        let h = compute(HouseSystem::Regiomontanus, 123.0, EPS, -23.55);
        let total: f64 = (0..12).map(|i| norm360(h.cusps[(i + 1) % 12] - h.cusps[i])).sum();
        assert!((total - 360.0).abs() < 1e-9);
        // No equador, Regiomontanus e Placidus coincidem.
        let r = compute(HouseSystem::Regiomontanus, 200.0, EPS, 0.0);
        let p = compute(HouseSystem::Placidus, 200.0, EPS, 0.0);
        for i in 0..12 {
            assert!(wrap180(r.cusps[i] - p.cusps[i]).abs() < 1e-7);
        }
    }

    #[test]
    fn polar_latitude_falls_back() {
        let h = compute(HouseSystem::Placidus, 80.0, EPS, 78.0);
        assert!(h.fallback);
        assert_eq!(h.system, HouseSystem::Porphyry);
    }

    #[test]
    fn whole_sign_and_house_lookup() {
        let h = compute(HouseSystem::WholeSign, 10.0, EPS, 40.0);
        assert_eq!(h.cusps[0] % 30.0, 0.0);
        assert_eq!(house_of(h.cusps[0] + 1.0, &h.cusps), 1);
        assert_eq!(house_of(norm360(h.cusps[0] - 1.0), &h.cusps), 12);
        assert_eq!(house_of(norm360(h.cusps[6] + 29.0), &h.cusps), 7);
    }
}
