use astro_core::{compute_chart, compute_transits, time, ChartInput, HouseSystem, NodeType, Orbs, Zodiac};

fn input() -> ChartInput {
    // 8 de agosto de 1988, 10:30 em São Paulo (UTC−3) = 13:30 UT.
    ChartInput {
        jd_ut: time::julian_day(1988, 8, 8, 13.5),
        lat: -23.5475,
        lon: -46.6361,
        time_known: true,
        house_system: HouseSystem::Placidus,
        zodiac: Zodiac::Tropical,
        node_type: NodeType::True,
        engine: Default::default(),
        orbs: Orbs::default(),
        day_range: None,
    }
}

#[test]
fn natal_chart_is_consistent() {
    let c = compute_chart(&input()).unwrap();
    assert_eq!(c.points.len(), 14);
    let get = |k: &str| c.points.iter().find(|p| p.key == k).unwrap();
    // Mesmo instante da referência do JPL Horizons (tests/horizons_reference.json).
    assert!((get("sun").lon - 136.2049).abs() < 0.001, "sol {}", get("sun").lon);
    assert_eq!(get("sun").sign, 4); // Leão
    let h = c.houses.as_ref().unwrap();
    assert_eq!(get("asc").lon, h.cusps[0]);
    assert_eq!(get("mc").lon, h.cusps[9]);
    assert!(c.aspects.windows(2).all(|w| w[0].orb <= w[1].orb));
    assert_eq!(c.highlights.elements.iter().sum::<u8>(), 10);
    assert_eq!(c.highlights.modalities.iter().sum::<u8>(), 10);
    assert!(c.highlights.chart_ruler.is_some());
    for p in &c.points {
        println!(
            "{:10} {:7.3} {:>12} {:5.2} casa {:?} R={}",
            p.key,
            p.lon,
            astro_core::zodiac::SIGNS[p.sign],
            p.sign_deg,
            p.house,
            p.retrograde
        );
    }
    println!("{:?}", c.highlights);
}

#[test]
fn unknown_time_has_no_houses_and_flags_moon() {
    let mut i = input();
    i.time_known = false;
    // Procura um dia (em São Paulo, UTC−3) em que a Lua troca de signo.
    let sign = |jd: f64| {
        astro_core::zodiac::sign_index(
            astro_core::ephem::longitude(astro_core::Body::Moon, jd, NodeType::True).unwrap(),
        )
    };
    let mut start = time::julian_day(2000, 3, 1, 3.0);
    while sign(start) == sign(start + 1.0) {
        start += 1.0;
    }
    i.jd_ut = start + 0.5;
    i.day_range = Some((start, start + 1.0));
    let c = compute_chart(&i).unwrap();
    assert!(c.houses.is_none());
    assert_eq!(c.points.len(), 12);
    assert!(c.points.iter().all(|p| p.house.is_none()));
    println!("{:?}", c.warnings);
    assert!(c.points[1].sign_uncertain, "a Lua deveria estar marcada como incerta");
    assert!(c.points.iter().filter(|p| p.sign_uncertain).count() == 1);
    assert_eq!(c.warnings.len(), 2);
}

#[test]
fn sidereal_shifts_by_ayanamsa() {
    let t = compute_chart(&input()).unwrap();
    let mut i = input();
    i.zodiac = Zodiac::SiderealLahiri;
    let s = compute_chart(&i).unwrap();
    let d = astro_core::norm360(t.points[0].lon - s.points[0].lon);
    assert!((d - 23.70).abs() < 0.02, "ayanamsa Lahiri em 1988: {d}");
}

#[test]
fn transits_are_exact_and_ordered() {
    let i = input();
    let start = time::julian_day(2026, 11, 26, 0.0);
    let t = compute_transits(&i, start, 548).unwrap();
    assert!(!t.hits.is_empty());
    assert!(t.hits.windows(2).all(|w| w[0].jd_ut <= w[1].jd_ut));
    let natal = compute_chart(&i).unwrap();
    for h in &t.hits {
        let body = match h.transiting.as_str() {
            "jupiter" => astro_core::Body::Jupiter,
            "saturn" => astro_core::Body::Saturn,
            "uranus" => astro_core::Body::Uranus,
            "neptune" => astro_core::Body::Neptune,
            _ => astro_core::Body::Pluto,
        };
        let lon = astro_core::ephem::longitude(body, h.jd_ut, NodeType::True).unwrap();
        let n = natal.points.iter().find(|p| p.key == h.natal).unwrap().lon;
        let sep = astro_core::wrap180(lon - n).abs();
        assert!((sep - h.kind.angle()).abs() < 1e-4, "{h:?} sep {sep}");
        assert!(h.pass >= 1 && h.pass <= h.passes);
    }
    println!("{} trânsitos exatos; {} ativos no início", t.hits.len(), t.active.len());
}
