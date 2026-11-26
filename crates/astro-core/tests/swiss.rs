//! Confronto entre o motor nativo e a Swiss Ephemeris. Só com `--features swisseph`.
#![cfg(feature = "swisseph")]

use astro_core::engine::{self, Engine, WithFallback};
use astro_core::{compute_chart, ephem, houses, swiss, time, wrap180, Body, ChartInput, HouseSystem, NodeType};

const DATES: [f64; 6] = [2415093.75, 2433282.5, 2442413.9, 2447382.0625, 2451545.0, 2460400.5];

#[test]
fn engines_agree_on_positions() {
    let swiss = engine::get(Engine::Swiss).unwrap();
    let native = engine::get(Engine::Native).unwrap();
    println!(
        "Swiss Ephemeris {} (arquivos .se1: {})",
        swiss::version(),
        swiss::using_data_files()
    );
    // Tolerâncias em segundos de arco: soma do erro do nativo com o do Moshier embutido.
    let cases = [
        (Body::Sun, NodeType::True, 2.0),
        (Body::Moon, NodeType::True, 10.0),
        (Body::Mercury, NodeType::True, 2.0),
        (Body::Venus, NodeType::True, 2.0),
        (Body::Mars, NodeType::True, 2.0),
        (Body::Jupiter, NodeType::True, 2.0),
        (Body::Saturn, NodeType::True, 2.0),
        (Body::Uranus, NodeType::True, 3.0),
        (Body::Neptune, NodeType::True, 3.0),
        (Body::Pluto, NodeType::True, 3.0),
        (Body::NorthNode, NodeType::Mean, 5.0),
        (Body::NorthNode, NodeType::True, 120.0),
    ];
    let mut failures = vec![];
    for (body, node, tol) in cases {
        let mut worst: f64 = 0.0;
        for jd in DATES {
            let s = swiss.position(body, jd, node).unwrap();
            let n = native.position(body, jd, node).unwrap();
            worst = worst.max((wrap180(s.lon - n.lon) * 3600.0).abs());
            if !matches!(body, Body::NorthNode) {
                assert_eq!(s.retrograde, n.retrograde, "{body:?} em {jd}: sentido do movimento");
                assert!(
                    (s.speed - n.speed).abs() < 0.002,
                    "{body:?} em {jd}: velocidade {} x {}",
                    s.speed,
                    n.speed
                );
            }
        }
        println!("{body:?} ({node:?}): diferença máxima {worst:.2}\"");
        if worst > tol {
            failures.push(format!("{body:?}: {worst:.2}\" > {tol}\""));
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn houses_match_swiss_ephemeris() {
    let places = [
        (-23.5475, -46.6361),
        (51.5, -0.12),
        (40.71, -74.0),
        (-33.87, 151.2),
        (64.15, -21.94),
        (0.0, 30.0),
    ];
    let systems = [
        HouseSystem::Placidus,
        HouseSystem::Regiomontanus,
        HouseSystem::Porphyry,
        HouseSystem::Equal,
        HouseSystem::WholeSign,
    ];
    let mut worst: f64 = 0.0;
    for jd in DATES {
        for (lat, lon) in places {
            let (eps, _) = ephem::obliquity_and_nutation(time::jde(jd));
            let armc = astro_core::norm360(ephem::greenwich_sidereal_deg(jd) + lon);
            for system in systems {
                let ours = houses::compute(system, armc, eps, lat);
                let (cusps, asc, mc) = swiss::houses(jd, lat, lon, system).unwrap();
                let mut diffs = vec![wrap180(ours.asc - asc), wrap180(ours.mc - mc)];
                diffs.extend((0..12).map(|i| wrap180(ours.cusps[i] - cusps[i])));
                for d in diffs {
                    let arcsec = d.abs() * 3600.0;
                    worst = worst.max(arcsec);
                    assert!(arcsec < 2.0, "{system:?} lat {lat} jd {jd}: {arcsec:.2}\"");
                }
            }
        }
    }
    println!("casas: diferença máxima {worst:.3}\"");
}

#[test]
fn chiron_falls_back_to_native_without_asteroid_file() {
    if swiss::using_data_files() {
        return; // com os arquivos instalados não há o que substituir
    }
    let eph = WithFallback::new(Engine::Swiss).unwrap();
    let chiron = eph.position(Body::Chiron, 2451545.0, NodeType::True).unwrap();
    assert!((chiron.lon - 251.6176).abs() < 0.001);
    assert_eq!(eph.substituted(), vec![Body::Chiron]);

    let chart = compute_chart(&ChartInput {
        jd_ut: 2447382.0625,
        lat: -23.5475,
        lon: -46.6361,
        time_known: true,
        house_system: HouseSystem::Placidus,
        zodiac: Default::default(),
        node_type: NodeType::True,
        engine: Engine::Swiss,
        orbs: Default::default(),
        day_range: None,
    })
    .unwrap();
    assert_eq!(chart.engine, Engine::Swiss);
    assert!(
        chart.warnings.iter().any(|w| w.contains("chiron")),
        "{:?}",
        chart.warnings
    );
}
