//! Posições geocêntricas aparentes (eclíptica e equinócio verdadeiros da data).

use crate::{norm360, time, wrap180, zodiac::NodeType};
use astro::planet::Planet;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Body {
    Sun,
    Moon,
    Mercury,
    Venus,
    Mars,
    Jupiter,
    Saturn,
    Uranus,
    Neptune,
    Pluto,
    NorthNode,
    Chiron,
}

impl Body {
    pub const ALL: [Body; 12] = [
        Body::Sun,
        Body::Moon,
        Body::Mercury,
        Body::Venus,
        Body::Mars,
        Body::Jupiter,
        Body::Saturn,
        Body::Uranus,
        Body::Neptune,
        Body::Pluto,
        Body::NorthNode,
        Body::Chiron,
    ];

    /// Os dez "planetas" da astrologia (Sol a Plutão).
    pub const PLANETS: [Body; 10] = [
        Body::Sun,
        Body::Moon,
        Body::Mercury,
        Body::Venus,
        Body::Mars,
        Body::Jupiter,
        Body::Saturn,
        Body::Uranus,
        Body::Neptune,
        Body::Pluto,
    ];
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum EphemError {
    /// Data fora do intervalo coberto pelas tabelas embutidas.
    OutOfRange { body: Body, min_year: i32, max_year: i32 },
    /// O motor pedido não foi compilado neste binário.
    EngineUnavailable { engine: crate::engine::Engine },
    /// O motor não conseguiu calcular o corpo (em geral, falta um arquivo de dados).
    Engine { body: Body, detail: String },
}

impl std::fmt::Display for EphemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EphemError::OutOfRange {
                body,
                min_year,
                max_year,
            } => {
                write!(f, "{body:?}: data fora do intervalo suportado ({min_year}–{max_year})")
            }
            EphemError::EngineUnavailable { engine } => {
                write!(
                    f,
                    "o motor de efemérides {engine:?} não está disponível nesta versão do app"
                )
            }
            EphemError::Engine { body, detail } => write!(f, "{body:?}: {detail}"),
        }
    }
}

impl std::error::Error for EphemError {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    /// Longitude eclíptica aparente em graus (tropical).
    pub lon: f64,
    /// Latitude eclíptica em graus.
    pub lat: f64,
    /// Velocidade em longitude, graus por dia.
    pub speed: f64,
    pub retrograde: bool,
}

const ARCSEC: f64 = PI / (180.0 * 3600.0);
/// Tempo-luz por unidade astronômica, em dias.
const LIGHT_DAYS_PER_AU: f64 = 0.005_775_518_3;
const J2000: f64 = 2_451_545.0;

/// Longitude (graus, tropical aparente) em um dia juliano UT.
pub fn longitude(body: Body, jd_ut: f64, node: NodeType) -> Result<f64, EphemError> {
    Ok(apparent(body, time::jde(jd_ut), node)?.0)
}

/// Posição completa, com velocidade por diferença central.
pub fn position(body: Body, jd_ut: f64, node: NodeType) -> Result<Position, EphemError> {
    let jde = time::jde(jd_ut);
    let (lon, lat) = apparent(body, jde, node)?;
    let h = 0.02;
    // Perto dos limites das tabelas a diferença central cai fora do intervalo; usa a lateral.
    let speed = match (apparent(body, jde - h, node), apparent(body, jde + h, node)) {
        (Ok(a), Ok(b)) => wrap180(b.0 - a.0) / (2.0 * h),
        (Ok(a), Err(_)) => wrap180(lon - a.0) / h,
        (Err(_), Ok(b)) => wrap180(b.0 - lon) / h,
        (Err(e), Err(_)) => return Err(e),
    };
    Ok(Position {
        lon,
        lat,
        speed,
        retrograde: speed < 0.0,
    })
}

/// (longitude, latitude) aparentes em graus para um JDE (TT).
fn apparent(body: Body, jde: f64, node: NodeType) -> Result<(f64, f64), EphemError> {
    let (dpsi, _) = astro::nutation::nutation(jde);
    let (lon, lat) = match body {
        Body::Sun => sun(jde),
        Body::Moon => {
            let (p, _) = astro::lunar::geocent_ecl_pos(jde);
            (p.long, p.lat)
        }
        Body::Mercury => planet(&Planet::Mercury, jde),
        Body::Venus => planet(&Planet::Venus, jde),
        Body::Mars => planet(&Planet::Mars, jde),
        Body::Jupiter => planet(&Planet::Jupiter, jde),
        Body::Saturn => planet(&Planet::Saturn, jde),
        Body::Uranus => planet(&Planet::Uranus, jde),
        Body::Neptune => planet(&Planet::Neptune, jde),
        Body::Pluto => tabulated(body, &PLUTO, jde)?,
        Body::Chiron => tabulated(body, &CHIRON, jde)?,
        Body::NorthNode => {
            let jc = time::centuries(jde);
            let l = match node {
                NodeType::True => osculating_node(jde),
                NodeType::Mean => astro::lunar::mn_ascend_node(jc),
            };
            (l, 0.0)
        }
    };
    Ok((norm360((lon + dpsi).to_degrees()), lat.to_degrees()))
}

/// Nodo verdadeiro (osculador): onde o plano instantâneo da órbita da Lua, definido pela
/// posição e pela velocidade geocêntricas, corta a eclíptica. Confere com os elementos
/// osculadores do JPL Horizons dentro de ~1,5′; a série curta de Meeus chega a errar 5′.
fn osculating_node(jde: f64) -> f64 {
    let moon = |t: f64| {
        let (p, r) = astro::lunar::geocent_ecl_pos(t);
        rect(p.long, p.lat, r)
    };
    let h = 0.05;
    let (a, r, b) = (moon(jde - h), moon(jde), moon(jde + h));
    let v = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    // Normal ao plano orbital: r × v. O nodo ascendente fica a 90° dela, sobre a eclíptica.
    let n = [r[1] * v[2] - r[2] * v[1], r[2] * v[0] - r[0] * v[2]];
    n[0].atan2(-n[1])
}

/// Excentricidade da órbita da Terra e longitude do periélio (Meeus, cap. 23).
fn earth_orbit(t: f64) -> (f64, f64) {
    let e = 0.016_708_634 - 0.000_042_037 * t - 0.000_000_126_7 * t * t;
    let peri = (102.937_35 + 1.719_46 * t + 0.000_46 * t * t).to_radians();
    (e, peri)
}

/// Aberração anual em coordenadas eclípticas (radianos).
fn aberration(lon: f64, lat: f64, sun_lon: f64, t: f64) -> (f64, f64) {
    let k = 20.495_52 * ARCSEC;
    let (e, peri) = earth_orbit(t);
    let dlon = (-k * (sun_lon - lon).cos() + e * k * (peri - lon).cos()) / lat.cos();
    let dlat = -k * lat.sin() * ((sun_lon - lon).sin() - e * (peri - lon).sin());
    (dlon, dlat)
}

/// Passagem do referencial dinâmico do VSOP87 para o FK5 (Meeus, cap. 32).
fn fk5(lon: f64, lat: f64, t: f64) -> (f64, f64) {
    let l1 = lon - (1.397 * t + 0.000_31 * t * t).to_radians();
    let dlon = -0.090_33 * ARCSEC + 0.039_16 * ARCSEC * (l1.cos() + l1.sin()) * lat.tan();
    let dlat = 0.039_16 * ARCSEC * (l1.cos() - l1.sin());
    (lon + dlon, lat + dlat)
}

fn sun(jde: f64) -> (f64, f64) {
    let t = time::centuries(jde);
    let (l0, b0, r0) = astro::planet::heliocent_coords(&Planet::Earth, jde);
    // O crate devolve a latitude em [0, 2π); o seno desfaz a volta.
    let (lon, lat) = fk5(l0 + PI, -(b0.sin().asin()), t);
    (lon - 20.4898 * ARCSEC / r0, lat)
}

fn rect(l: f64, b: f64, r: f64) -> [f64; 3] {
    [r * b.cos() * l.cos(), r * b.cos() * l.sin(), r * b.sin()]
}

fn polar(v: [f64; 3]) -> (f64, f64, f64) {
    let r = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    (v[1].atan2(v[0]), (v[2] / r).asin(), r)
}

/// Planeta do VSOP87: geocêntrico, com tempo-luz, aberração e FK5; equinócio médio da data.
fn planet(p: &Planet, jde: f64) -> (f64, f64) {
    let t = time::centuries(jde);
    let (l0, b0, r0) = astro::planet::heliocent_coords(&Planet::Earth, jde);
    let earth = rect(l0, b0, r0);
    let mut tau = 0.0;
    let mut geo = (0.0, 0.0, 0.0);
    for _ in 0..3 {
        let (l, b, r) = astro::planet::heliocent_coords(p, jde - tau);
        let h = rect(l, b, r);
        geo = polar([h[0] - earth[0], h[1] - earth[1], h[2] - earth[2]]);
        tau = LIGHT_DAYS_PER_AU * geo.2;
    }
    let (dl, db) = aberration(geo.0, geo.1, l0 + PI, t);
    fk5(geo.0 + dl, geo.1 + db, t)
}

/// Tabela de posições heliocêntricas retangulares (eclíptica J2000, UA), passo fixo.
struct Table {
    bytes: &'static [u8],
}

static PLUTO: Table = Table {
    bytes: include_bytes!("../data/pluto.bin"),
};
static CHIRON: Table = Table {
    bytes: include_bytes!("../data/chiron.bin"),
};

impl Table {
    fn f64_at(&self, off: usize) -> f64 {
        f64::from_le_bytes(self.bytes[off..off + 8].try_into().unwrap())
    }
    fn jd0(&self) -> f64 {
        self.f64_at(0)
    }
    fn step(&self) -> f64 {
        self.f64_at(8)
    }
    fn len(&self) -> usize {
        u64::from_le_bytes(self.bytes[16..24].try_into().unwrap()) as usize
    }
    fn sample(&self, i: usize) -> [f64; 3] {
        let o = 24 + i * 24;
        [self.f64_at(o), self.f64_at(o + 8), self.f64_at(o + 16)]
    }

    /// Interpolação de Lagrange com 8 pontos.
    fn eval(&self, jde: f64) -> Option<[f64; 3]> {
        const N: usize = 8;
        let x = (jde - self.jd0()) / self.step();
        let n = self.len();
        if !(x >= 0.0 && x <= (n - 1) as f64) {
            return None;
        }
        let first = (x.floor() as isize - (N as isize / 2 - 1)).clamp(0, (n - N) as isize) as usize;
        let mut out = [0.0; 3];
        for i in 0..N {
            let mut w = 1.0;
            for j in 0..N {
                if i != j {
                    w *= (x - (first + j) as f64) / (i as f64 - j as f64);
                }
            }
            let s = self.sample(first + i);
            for k in 0..3 {
                out[k] += w * s[k];
            }
        }
        Some(out)
    }
}

fn tabulated(body: Body, table: &Table, jde: f64) -> Result<(f64, f64), EphemError> {
    let err = || EphemError::OutOfRange {
        body,
        min_year: 1800,
        max_year: 2199,
    };
    let t = time::centuries(jde);
    // Terra: do equinócio da data para J2000, no mesmo referencial da tabela.
    let (l0, b0, r0) = astro::planet::heliocent_coords(&Planet::Earth, jde);
    let (l0j, b0j) = astro::precess::precess_ecl_coords(l0, b0, jde, J2000);
    let earth = rect(l0j, b0j, r0);
    let mut tau = 0.0;
    let mut geo = (0.0, 0.0, 0.0);
    for _ in 0..3 {
        let h = table.eval(jde - tau).ok_or_else(err)?;
        geo = polar([h[0] - earth[0], h[1] - earth[1], h[2] - earth[2]]);
        tau = LIGHT_DAYS_PER_AU * geo.2;
    }
    let (lon, lat) = astro::precess::precess_ecl_coords(geo.0, geo.1, J2000, jde);
    let (dl, db) = aberration(lon, lat, l0 + PI, t);
    Ok((lon + dl, lat + db))
}

/// Obliquidade verdadeira da eclíptica (radianos) e nutação em longitude (radianos).
pub fn obliquity_and_nutation(jde: f64) -> (f64, f64) {
    let (dpsi, deps) = astro::nutation::nutation(jde);
    (astro::ecliptic::mn_oblq_laskar(jde) + deps, dpsi)
}

/// Tempo sideral aparente de Greenwich, em graus.
pub fn greenwich_sidereal_deg(jd_ut: f64) -> f64 {
    let (eps, dpsi) = obliquity_and_nutation(time::jde(jd_ut));
    norm360((astro::time::mn_sidr(jd_ut) + dpsi * eps.cos()).to_degrees())
}
