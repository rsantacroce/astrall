//! Confere as posições contra o JPL Horizons (geocêntricas aparentes, eclíptica da data).

use astro_core::{ephem, wrap180, Body, NodeType};
use std::collections::HashMap;

#[derive(serde::Deserialize)]
struct Row {
    jd_ut: f64,
    lon: f64,
    lat: f64,
}

#[test]
fn matches_horizons() {
    let data: HashMap<String, Vec<Row>> = serde_json::from_str(include_str!("horizons_reference.json")).unwrap();
    // Tolerâncias em segundos de arco.
    let bodies = [
        ("sun", Body::Sun, 1.0),
        ("moon", Body::Moon, 8.0),
        ("mercury", Body::Mercury, 1.0),
        ("venus", Body::Venus, 1.0),
        ("mars", Body::Mars, 1.0),
        ("jupiter", Body::Jupiter, 1.0),
        ("saturn", Body::Saturn, 1.0),
        ("uranus", Body::Uranus, 2.0),
        ("neptune", Body::Neptune, 2.0),
        ("pluto", Body::Pluto, 1.0),
        ("chiron", Body::Chiron, 1.0),
    ];
    let mut failures = vec![];
    for (name, body, tol) in bodies {
        let (mut past, mut future): (f64, f64) = (0.0, 0.0);
        for r in &data[name] {
            let p = ephem::position(body, r.jd_ut, NodeType::True).unwrap();
            let err = (wrap180(p.lon - r.lon) * 3600.0)
                .abs()
                .max(((p.lat - r.lat) * 3600.0).abs());
            // Depois de 2024 o Horizons e o polinômio de ΔT divergem na previsão da rotação da
            // Terra; a diferença em longitude é só esse deslocamento de tempo.
            if r.jd_ut < 2_460_401.0 {
                past = past.max(err)
            } else {
                future = future.max(err)
            }
        }
        println!("{name:8} erro máximo: passado {past:5.2}\"  futuro {future:5.2}\"");
        if past > tol || future > tol * 8.0 {
            failures.push(format!("{name}: {past:.2}\" / {future:.2}\" (tolerância {tol}\")"));
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

/// Nodo verdadeiro contra os elementos osculadores da Lua no Horizons (Ω na eclíptica J2000,
/// trazido para a data pela precessão geral). Tolerância de 2′.
#[test]
fn true_node_matches_osculating_elements() {
    let refs = [
        (2433282.5, 13.23683594),
        (2447382.0625, 344.54538197),
        (2451545.0, 123.95805544),
        (2460400.5, 15.29143717),
    ];
    for (jd, omega_j2000) in refs {
        let precession = (jd - 2451545.0) / 36525.0 * 5028.796195 / 3600.0;
        let node = ephem::longitude(Body::NorthNode, jd, NodeType::True).unwrap();
        let err = wrap180(node - precession - omega_j2000) * 60.0;
        assert!(err.abs() < 2.0, "jd {jd}: erro de {err:.2}′");
    }
}
