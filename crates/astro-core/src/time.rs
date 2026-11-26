//! Dia juliano, calendário gregoriano e ΔT.

/// Dia juliano (UT) para uma data do calendário gregoriano; `hour` é hora decimal UT.
pub fn julian_day(year: i32, month: u32, day: u32, hour: f64) -> f64 {
    let (mut y, mut m) = (year as f64, month as f64);
    if month <= 2 {
        y -= 1.0;
        m += 12.0;
    }
    let a = (y / 100.0).floor();
    let b = 2.0 - a + (a / 4.0).floor();
    (365.25 * (y + 4716.0)).floor() + (30.6001 * (m + 1.0)).floor() + day as f64 + b - 1524.5 + hour / 24.0
}

/// Data gregoriana (ano, mês, dia, hora decimal UT) de um dia juliano.
pub fn calendar_date(jd: f64) -> (i32, u32, u32, f64) {
    let z = (jd + 0.5).floor();
    let f = jd + 0.5 - z;
    let alpha = ((z - 1867216.25) / 36524.25).floor();
    let a = z + 1.0 + alpha - (alpha / 4.0).floor();
    let b = a + 1524.0;
    let c = ((b - 122.1) / 365.25).floor();
    let d = (365.25 * c).floor();
    let e = ((b - d) / 30.6001).floor();
    let day = b - d - (30.6001 * e).floor();
    let month = if e < 14.0 { e - 1.0 } else { e - 13.0 };
    let year = if month > 2.0 { c - 4716.0 } else { c - 4715.0 };
    (year as i32, month as u32, day as u32, f * 24.0)
}

/// ΔT = TT − UT em segundos (polinômios de Espenak & Meeus).
pub fn delta_t_seconds(jd_ut: f64) -> f64 {
    let (y, m, _, _) = calendar_date(jd_ut);
    astro::time::delta_t(y, m as u8)
}

/// Dia juliano das efemérides (TT) a partir do dia juliano UT.
pub fn jde(jd_ut: f64) -> f64 {
    jd_ut + delta_t_seconds(jd_ut) / 86400.0
}

/// Séculos julianos desde J2000.0.
pub fn centuries(jde: f64) -> f64 {
    (jde - 2451545.0) / 36525.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jd_roundtrip() {
        assert_eq!(julian_day(2000, 1, 1, 12.0), 2451545.0);
        assert_eq!(julian_day(1957, 10, 4, 0.81 * 24.0), 2436116.31);
        let (y, m, d, h) = calendar_date(2447382.0625);
        assert_eq!((y, m, d), (1988, 8, 8));
        assert!((h - 13.5).abs() < 1e-6);
    }

    #[test]
    fn delta_t_known_values() {
        assert!((delta_t_seconds(julian_day(2000, 1, 1, 0.0)) - 63.8).abs() < 1.0);
        assert!((delta_t_seconds(julian_day(1950, 1, 1, 0.0)) - 29.1).abs() < 1.0);
        assert!((delta_t_seconds(julian_day(2020, 1, 1, 0.0)) - 69.4).abs() < 3.0);
    }
}
