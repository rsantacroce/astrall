//! Montagem do mapa natal: pontos, casas, aspectos e destaques.

use crate::aspects::{self, Aspect, AspectPoint, Orbs};
use crate::engine::{Engine, WithFallback};
use crate::ephem::{self, Body, EphemError};
use crate::houses::{self, HouseSystem, Houses};
use crate::zodiac::{self, Element, Modality, NodeType, Zodiac};
use crate::{norm360, time};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartInput {
    /// Instante do nascimento, dia juliano UT. Sem horário conhecido: meio-dia local.
    pub jd_ut: f64,
    /// Latitude em graus (norte positivo).
    pub lat: f64,
    /// Longitude em graus (leste positivo).
    pub lon: f64,
    pub time_known: bool,
    #[serde(default)]
    pub house_system: HouseSystem,
    #[serde(default)]
    pub zodiac: Zodiac,
    #[serde(default)]
    pub node_type: NodeType,
    /// Motor de efemérides; o nativo quando omitido.
    #[serde(default)]
    pub engine: Engine,
    #[serde(default)]
    pub orbs: Orbs,
    /// Início e fim do dia local (JD UT). Com horário desconhecido, serve para marcar os
    /// pontos que trocam de signo ao longo do dia.
    #[serde(default)]
    pub day_range: Option<(f64, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    /// "sun", "moon", …, "north_node", "chiron", "asc", "mc".
    pub key: String,
    /// Longitude no zodíaco escolhido, em graus.
    pub lon: f64,
    pub lat: f64,
    pub speed: f64,
    pub retrograde: bool,
    /// Signo, 0 = Áries.
    pub sign: usize,
    /// Graus dentro do signo.
    pub sign_deg: f64,
    pub house: Option<u8>,
    /// Sem horário: o ponto muda de signo durante o dia de nascimento.
    pub sign_uncertain: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stellium {
    /// "sign" ou "house".
    pub kind: String,
    /// Índice do signo (0–11) ou número da casa (1–12).
    pub index: usize,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartRuler {
    pub asc_sign: usize,
    /// Regente moderno do signo ascendente.
    pub ruler: String,
    /// Regente tradicional, quando difere do moderno.
    pub traditional: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Highlights {
    /// Planetas (Sol a Plutão) por elemento: fogo, terra, ar, água.
    pub elements: [u8; 4],
    /// Planetas por modalidade: cardinal, fixo, mutável.
    pub modalities: [u8; 3],
    pub dominant_elements: Vec<Element>,
    pub dominant_modalities: Vec<Modality>,
    pub stelliums: Vec<Stellium>,
    pub chart_ruler: Option<ChartRuler>,
    /// Elongação Lua − Sol em graus.
    pub moon_phase_angle: f64,
    pub moon_phase: String,
    /// Mapa diurno (Sol acima do horizonte); `None` sem horário.
    pub day_chart: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chart {
    pub jd_ut: f64,
    pub delta_t: f64,
    /// Motor que calculou as posições (mapas salvos antes desta opção: nativo).
    #[serde(default)]
    pub engine: Engine,
    pub zodiac: Zodiac,
    /// Ayanamsa aplicada, em graus (zero no tropical).
    pub ayanamsa: f64,
    /// Obliquidade verdadeira da eclíptica, em graus.
    pub obliquity: f64,
    /// Tempo sideral local em horas; `None` sem horário.
    pub sidereal_time: Option<f64>,
    pub points: Vec<Point>,
    pub houses: Option<Houses>,
    pub aspects: Vec<Aspect>,
    pub highlights: Highlights,
    pub warnings: Vec<String>,
}

pub fn body_key(b: Body) -> &'static str {
    match b {
        Body::Sun => "sun",
        Body::Moon => "moon",
        Body::Mercury => "mercury",
        Body::Venus => "venus",
        Body::Mars => "mars",
        Body::Jupiter => "jupiter",
        Body::Saturn => "saturn",
        Body::Uranus => "uranus",
        Body::Neptune => "neptune",
        Body::Pluto => "pluto",
        Body::NorthNode => "north_node",
        Body::Chiron => "chiron",
    }
}

const MODERN_RULERS: [Body; 12] = [
    Body::Mars,
    Body::Venus,
    Body::Mercury,
    Body::Moon,
    Body::Sun,
    Body::Mercury,
    Body::Venus,
    Body::Pluto,
    Body::Jupiter,
    Body::Saturn,
    Body::Uranus,
    Body::Neptune,
];

fn traditional_ruler(sign: usize) -> Option<Body> {
    match sign {
        7 => Some(Body::Mars),
        10 => Some(Body::Saturn),
        11 => Some(Body::Jupiter),
        _ => None,
    }
}

fn moon_phase_name(angle: f64) -> &'static str {
    const NAMES: [&str; 8] = [
        "Lua Nova",
        "Lua Crescente",
        "Quarto Crescente",
        "Gibosa Crescente",
        "Lua Cheia",
        "Disseminadora",
        "Quarto Minguante",
        "Balsâmica",
    ];
    NAMES[((angle / 45.0).floor() as usize).min(7)]
}

/// Deslocamento a subtrair das longitudes tropicais aparentes para o zodíaco escolhido.
pub(crate) fn zodiac_offset(zodiac: Zodiac, jde: f64) -> f64 {
    match zodiac {
        Zodiac::Tropical => 0.0,
        // A ayanamsa é média; a nutação já embutida na longitude aparente sai junto.
        _ => zodiac::ayanamsa(zodiac, jde) + ephem::obliquity_and_nutation(jde).1.to_degrees(),
    }
}

pub fn compute_chart(input: &ChartInput) -> Result<Chart, EphemError> {
    let jde = time::jde(input.jd_ut);
    let (eps, _) = ephem::obliquity_and_nutation(jde);
    let offset = zodiac_offset(input.zodiac, jde);
    let mut warnings = vec![];
    let eph = WithFallback::new(input.engine)?;

    let (houses, sidereal_time) = if input.time_known {
        let armc = norm360(ephem::greenwich_sidereal_deg(input.jd_ut) + input.lon);
        let mut h = houses::compute(input.house_system, armc, eps, input.lat);
        if h.fallback {
            warnings.push(
                "Placidus não é definido nesta latitude (região polar); as casas foram calculadas em Porphyry.".into(),
            );
        }
        h.asc = norm360(h.asc - offset);
        h.mc = norm360(h.mc - offset);
        for c in h.cusps.iter_mut() {
            *c = norm360(*c - offset);
        }
        if input.house_system == HouseSystem::WholeSign && offset != 0.0 {
            // Signos inteiros seguem o zodíaco escolhido.
            let start = (h.asc / 30.0).floor() * 30.0;
            for (i, c) in h.cusps.iter_mut().enumerate() {
                *c = norm360(start + 30.0 * i as f64);
            }
        }
        (Some(h), Some(armc / 15.0))
    } else {
        warnings.push(
            "Horário de nascimento desconhecido: posições calculadas para o meio-dia local. Ascendente, Meio do Céu e casas não foram calculados."
                .into(),
        );
        (None, None)
    };

    let mut points = vec![];
    for body in Body::ALL {
        let p = eph.position(body, input.jd_ut, input.node_type)?;
        let lon = norm360(p.lon - offset);
        let sign = zodiac::sign_index(lon);
        let mut sign_uncertain = false;
        if let (false, Some((a, b))) = (input.time_known, input.day_range) {
            let sa = zodiac::sign_index(
                eph.longitude(body, a, input.node_type)? - zodiac_offset(input.zodiac, time::jde(a)),
            );
            let sb = zodiac::sign_index(
                eph.longitude(body, b, input.node_type)? - zodiac_offset(input.zodiac, time::jde(b)),
            );
            sign_uncertain = sa != sb;
        }
        points.push(Point {
            key: body_key(body).into(),
            lon,
            lat: p.lat,
            speed: p.speed,
            retrograde: p.retrograde,
            sign,
            sign_deg: lon - 30.0 * sign as f64,
            house: houses.as_ref().map(|h| houses::house_of(lon, &h.cusps)),
            sign_uncertain,
        });
    }
    for p in points.iter().filter(|p| p.sign_uncertain) {
        let what = if p.key == "moon" {
            "A Lua".to_string()
        } else {
            format!("O ponto \"{}\"", p.key)
        };
        warnings.push(format!(
            "{what} muda de signo no dia do nascimento; sem o horário, o signo indicado ({}) é incerto.",
            zodiac::SIGNS[p.sign]
        ));
    }
    for body in eph.substituted() {
        warnings.push(format!(
            "O motor escolhido não tem dados para o ponto \"{}\" (falta o arquivo de asteroides); ele foi calculado pelo motor nativo.",
            body_key(body)
        ));
    }
    if let Some(h) = &houses {
        for (key, lon) in [("asc", h.asc), ("mc", h.mc)] {
            let sign = zodiac::sign_index(lon);
            points.push(Point {
                key: key.into(),
                lon,
                lat: 0.0,
                speed: 0.0,
                retrograde: false,
                sign,
                sign_deg: lon - 30.0 * sign as f64,
                house: Some(if key == "asc" {
                    1
                } else {
                    houses::house_of(lon, &h.cusps)
                }),
                sign_uncertain: false,
            });
        }
    }

    let aspect_points: Vec<AspectPoint> = points
        .iter()
        .map(|p| AspectPoint {
            key: p.key.clone(),
            lon: p.lon,
            speed: p.speed,
            luminary: p.key == "sun" || p.key == "moon",
            angle: p.key == "asc" || p.key == "mc",
        })
        .collect();
    let aspects = aspects::find(&aspect_points, input.orbs);

    let highlights = highlights(&points, houses.as_ref());
    Ok(Chart {
        jd_ut: input.jd_ut,
        delta_t: time::delta_t_seconds(input.jd_ut),
        engine: eph.engine(),
        zodiac: input.zodiac,
        ayanamsa: zodiac::ayanamsa(input.zodiac, jde),
        obliquity: eps.to_degrees(),
        sidereal_time,
        points,
        houses,
        aspects,
        highlights,
        warnings,
    })
}

fn highlights(points: &[Point], houses: Option<&Houses>) -> Highlights {
    let planets: Vec<&Point> = points.iter().take(10).collect();
    let mut elements = [0u8; 4];
    let mut modalities = [0u8; 3];
    for p in &planets {
        elements[p.sign % 4] += 1;
        modalities[p.sign % 3] += 1;
    }
    let emax = *elements.iter().max().unwrap();
    let mmax = *modalities.iter().max().unwrap();
    let dominant_elements = (0..4).filter(|&i| elements[i] == emax).map(zodiac::element).collect();
    let dominant_modalities = (0..3)
        .filter(|&i| modalities[i] == mmax)
        .map(zodiac::modality)
        .collect();

    let mut stelliums = vec![];
    for sign in 0..12 {
        let members: Vec<String> = planets
            .iter()
            .filter(|p| p.sign == sign)
            .map(|p| p.key.clone())
            .collect();
        if members.len() >= 3 {
            stelliums.push(Stellium {
                kind: "sign".into(),
                index: sign,
                members,
            });
        }
    }
    if houses.is_some() {
        for house in 1..=12u8 {
            let members: Vec<String> = planets
                .iter()
                .filter(|p| p.house == Some(house))
                .map(|p| p.key.clone())
                .collect();
            if members.len() >= 3 {
                stelliums.push(Stellium {
                    kind: "house".into(),
                    index: house as usize,
                    members,
                });
            }
        }
    }

    let chart_ruler = houses.map(|h| {
        let asc_sign = zodiac::sign_index(h.asc);
        ChartRuler {
            asc_sign,
            ruler: body_key(MODERN_RULERS[asc_sign]).into(),
            traditional: traditional_ruler(asc_sign).map(|b| body_key(b).to_string()),
        }
    });

    let moon_phase_angle = norm360(points[1].lon - points[0].lon);
    Highlights {
        elements,
        modalities,
        dominant_elements,
        dominant_modalities,
        stelliums,
        chart_ruler,
        moon_phase_angle,
        moon_phase: moon_phase_name(moon_phase_angle).into(),
        day_chart: points[0].house.map(|h| h >= 7),
    }
}
