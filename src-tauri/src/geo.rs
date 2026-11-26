//! Busca de cidades (GeoNames embutido) e fuso horário histórico (IANA, via chrono-tz).

use chrono::{Duration, LocalResult, NaiveDate, NaiveDateTime, Offset, TimeZone, Timelike};
use chrono_tz::{OffsetComponents, OffsetName, Tz};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::sync::OnceLock;

static CITIES_GZ: &[u8] = include_bytes!("../data/cities.tsv.gz");

#[derive(Debug, Clone, Serialize)]
pub struct City {
    pub name: String,
    pub admin: String,
    pub country: String,
    pub lat: f64,
    pub lon: f64,
    pub tz: String,
    pub population: u64,
}

struct Entry {
    city: City,
    /// Nome e nomes alternativos, normalizados.
    names: Vec<String>,
    admin_norm: String,
}

/// Minúsculas, sem acentos nem pontuação.
pub fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars().flat_map(|c| c.to_lowercase()) {
        let c = match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'ē' | 'ė' | 'ę' | 'ě' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'ī' | 'ı' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ő' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' => 'u',
            'ç' | 'ć' | 'č' => 'c',
            'ñ' | 'ń' | 'ň' => 'n',
            'ý' | 'ÿ' => 'y',
            'š' | 'ś' | 'ş' | 'ș' => 's',
            'ž' | 'ź' | 'ż' => 'z',
            'ł' => 'l',
            'ğ' => 'g',
            'ř' => 'r',
            'ț' | 'ť' => 't',
            'ď' | 'đ' => 'd',
            '-' | '\'' | '’' | '.' => ' ',
            c => c,
        };
        out.push(c);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn entries() -> &'static [Entry] {
    static DATA: OnceLock<Vec<Entry>> = OnceLock::new();
    DATA.get_or_init(|| {
        let mut text = String::new();
        flate2::read::GzDecoder::new(CITIES_GZ)
            .read_to_string(&mut text)
            .expect("cities.tsv.gz corrompido");
        text.lines()
            .filter_map(|line| {
                let c: Vec<&str> = line.split('\t').collect();
                if c.len() < 8 {
                    return None;
                }
                let mut names = vec![normalize(c[0])];
                names.extend(c[7].split('|').filter(|s| !s.is_empty()).map(normalize));
                Some(Entry {
                    city: City {
                        name: c[0].into(),
                        admin: c[1].into(),
                        country: c[2].into(),
                        lat: c[3].parse().ok()?,
                        lon: c[4].parse().ok()?,
                        tz: c[5].into(),
                        population: c[6].parse().unwrap_or(0),
                    },
                    names,
                    admin_norm: normalize(c[1]),
                })
            })
            .collect()
    })
}

/// Busca por nome; "cidade, estado" ou "cidade, BR" filtra pela segunda parte.
/// A base já vem ordenada por população, então o desempate é pela ordem.
pub fn search(query: &str, limit: usize) -> Vec<City> {
    let mut parts = query.split(',').map(normalize);
    let name = parts.next().unwrap_or_default();
    let filters: Vec<String> = parts.filter(|p| !p.is_empty()).collect();
    if name.chars().count() < 2 {
        return vec![];
    }
    // 0 = nome exato, 1 = prefixo do nome principal, 2 = prefixo de nome alternativo, 3 = contém.
    let mut buckets: [Vec<&Entry>; 4] = Default::default();
    for e in entries() {
        if !filters
            .iter()
            .all(|f| e.admin_norm.contains(f.as_str()) || e.city.country.eq_ignore_ascii_case(f))
        {
            continue;
        }
        let rank = if e.names.contains(&name) {
            0
        } else if e.names[0].starts_with(&name) {
            1
        } else if e.names.iter().any(|n| n.starts_with(&name)) {
            2
        } else if name.len() >= 4 && e.names[0].contains(&name) {
            3
        } else {
            continue;
        };
        if buckets[rank].len() < limit {
            buckets[rank].push(e);
        }
    }
    buckets
        .into_iter()
        .flatten()
        .take(limit)
        .map(|e| e.city.clone())
        .collect()
}

#[derive(Debug, Clone, Deserialize)]
pub struct TimeQuery {
    /// "AAAA-MM-DD"
    pub date: String,
    /// "HH:MM"; `None` quando o horário é desconhecido (usa meio-dia).
    pub time: Option<String>,
    /// Fuso IANA, ex.: "America/Sao_Paulo".
    pub tz: String,
    /// Deslocamento informado à mão, em minutos a leste de UTC; ignora o banco de fusos.
    pub offset_override_min: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimeResolution {
    /// Hora local usada no cálculo, "AAAA-MM-DD HH:MM".
    pub local: String,
    pub utc: String,
    pub jd_ut: f64,
    /// Deslocamento total em relação a UTC, em segundos (leste positivo).
    pub offset_seconds: i32,
    /// Horário de verão em vigor naquela data.
    pub dst: bool,
    /// Abreviação do fuso naquela data (ex.: "EDT", "LMT"); vazia nos fusos sem sigla.
    pub abbreviation: String,
    /// Antes da hora legal: o banco usa a hora média local da cidade de referência do fuso.
    pub local_mean_time: bool,
    /// A hora caiu na repetição do fim do horário de verão (existe duas vezes).
    pub ambiguous: bool,
    /// A hora caiu no salto do início do horário de verão (não existiu).
    pub nonexistent: bool,
    pub overridden: bool,
    /// Início e fim do dia local, em JD UT.
    pub day_start_jd: f64,
    pub day_end_jd: f64,
}

fn jd_of(utc: NaiveDateTime) -> f64 {
    let d = utc.date();
    use chrono::Datelike;
    let hour = utc.hour() as f64 + utc.minute() as f64 / 60.0 + utc.second() as f64 / 3600.0;
    astro_core::time::julian_day(d.year(), d.month(), d.day(), hour)
}

pub fn resolve_time(q: &TimeQuery) -> Result<TimeResolution, String> {
    let date = NaiveDate::parse_from_str(&q.date, "%Y-%m-%d").map_err(|_| "Data inválida.".to_string())?;
    let local = match &q.time {
        Some(t) => {
            let t = chrono::NaiveTime::parse_from_str(t, "%H:%M").map_err(|_| "Horário inválido.".to_string())?;
            date.and_time(t)
        }
        None => date.and_hms_opt(12, 0, 0).unwrap(),
    };
    let midnight = date.and_hms_opt(0, 0, 0).unwrap();

    let (offset_seconds, dst, abbreviation, lmt, ambiguous, nonexistent) = if let Some(min) = q.offset_override_min {
        (min * 60, false, "manual".to_string(), false, false, false)
    } else {
        let tz: Tz =
            q.tz.parse()
                .map_err(|_| format!("Fuso horário desconhecido: {}", q.tz))?;
        let (dt, ambiguous, nonexistent) = match tz.from_local_datetime(&local) {
            LocalResult::Single(dt) => (dt, false, false),
            // Na hora repetida, fica a primeira ocorrência (ainda em horário de verão).
            LocalResult::Ambiguous(first, _) => (first, true, false),
            // Na hora que não existiu, vale o deslocamento de logo antes do salto.
            LocalResult::None => {
                let before = tz
                    .from_local_datetime(&(local - Duration::hours(3)))
                    .earliest()
                    .ok_or("Não foi possível resolver o horário neste fuso.")?;
                (before, false, true)
            }
        };
        let off = dt.offset();
        let secs = off.fix().local_minus_utc();
        let abbr = off.abbreviation().unwrap_or("").to_string();
        (
            secs,
            off.dst_offset().num_seconds() != 0,
            abbr.clone(),
            abbr == "LMT",
            ambiguous,
            nonexistent,
        )
    };

    let utc = local - Duration::seconds(offset_seconds as i64);
    let day_start = midnight - Duration::seconds(offset_seconds as i64);
    Ok(TimeResolution {
        local: local.format("%Y-%m-%d %H:%M").to_string(),
        utc: utc.format("%Y-%m-%d %H:%M").to_string(),
        jd_ut: jd_of(utc),
        offset_seconds,
        dst,
        abbreviation,
        local_mean_time: lmt,
        ambiguous,
        nonexistent,
        overridden: q.offset_override_min.is_some(),
        day_start_jd: jd_of(day_start),
        day_end_jd: jd_of(day_start) + 1.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(date: &str, time: &str, tz: &str) -> TimeQuery {
        TimeQuery {
            date: date.into(),
            time: Some(time.into()),
            tz: tz.into(),
            offset_override_min: None,
        }
    }

    #[test]
    fn brazilian_dst_history() {
        // Verão 1988/89: horário de verão em vigor em São Paulo em janeiro.
        let r = resolve_time(&q("1989-01-15", "10:00", "America/Sao_Paulo")).unwrap();
        assert_eq!((r.offset_seconds, r.dst), (-2 * 3600, true));
        assert_eq!(r.utc, "1989-01-15 12:00");
        // Agosto: hora padrão.
        let r = resolve_time(&q("1988-08-08", "10:30", "America/Sao_Paulo")).unwrap();
        assert_eq!((r.offset_seconds, r.dst), (-3 * 3600, false));
        assert!((r.jd_ut - 2447382.0625).abs() < 1e-9);
        // Janeiro de 2020: o Brasil já tinha abolido o horário de verão.
        let r = resolve_time(&q("2020-01-15", "10:00", "America/Sao_Paulo")).unwrap();
        assert_eq!((r.offset_seconds, r.dst), (-3 * 3600, false));
        // 1970: sem horário de verão naquele verão.
        let r = resolve_time(&q("1970-01-15", "10:00", "America/Sao_Paulo")).unwrap();
        assert!(!r.dst);
    }

    #[test]
    fn local_mean_time_before_standard_time() {
        // Antes de 1914 São Paulo seguia a hora média local (−3:06:28).
        let r = resolve_time(&q("1900-06-01", "12:00", "America/Sao_Paulo")).unwrap();
        assert!(r.local_mean_time);
        assert_eq!(r.offset_seconds, -(3 * 3600 + 6 * 60 + 28));
    }

    #[test]
    fn gaps_overlaps_and_override() {
        // EUA, 2021-03-14 02:30 não existiu em Nova York.
        let r = resolve_time(&q("2021-03-14", "02:30", "America/New_York")).unwrap();
        assert!(r.nonexistent);
        assert_eq!(r.offset_seconds, -5 * 3600);
        // 2021-11-07 01:30 aconteceu duas vezes.
        let r = resolve_time(&q("2021-11-07", "01:30", "America/New_York")).unwrap();
        assert!(r.ambiguous && r.dst);
        let mut o = q("1988-08-08", "10:30", "America/Sao_Paulo");
        o.offset_override_min = Some(-120);
        let r = resolve_time(&o).unwrap();
        assert_eq!(r.utc, "1988-08-08 12:30");
        assert!(r.overridden);
    }

    #[test]
    fn city_search() {
        let r = search("sao paulo", 5);
        assert_eq!(r[0].name, "São Paulo");
        assert_eq!(r[0].tz, "America/Sao_Paulo");
        assert_eq!(search("Lisboa", 3)[0].country, "PT");
        assert_eq!(search("london, ca", 3)[0].admin, "Ontario");
        assert_eq!(search("santos, sao paulo", 3)[0].country, "BR");
        assert!(search("x", 5).is_empty());
    }
}
