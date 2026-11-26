//! Motor Swiss Ephemeris (biblioteca C da Astrodienst, via `sweph-sys`).
//!
//! Sem arquivos de dados a biblioteca usa as efemérides analíticas de Moshier, embutidas no
//! código (planetas ~1″, Lua ~3″). Com os arquivos `.se1` na pasta indicada por
//! [`set_ephe_path`] passa para as efemérides comprimidas do JPL (0,001″) e ganha Quíron,
//! que depende de `seas_18.se1`.

use crate::engine::{Engine, Ephemeris};
use crate::ephem::{Body, EphemError, Position};
use crate::houses::HouseSystem;
use crate::zodiac::NodeType;
use std::ffi::{c_char, c_int, CStr, CString};
use std::path::Path;
use std::sync::Mutex;
use sweph_sys as swe;

/// A biblioteca guarda estado global e não é segura entre threads: toda chamada passa aqui.
static LOCK: Mutex<()> = Mutex::new(());

pub struct Swiss;

/// Pasta com os arquivos `.se1` (sepl_18, semo_18, seas_18…). Chame uma vez, ao iniciar.
pub fn set_ephe_path(path: &Path) {
    if let Ok(c) = CString::new(path.to_string_lossy().as_bytes()) {
        let _guard = LOCK.lock().unwrap();
        unsafe { swe::swe_set_ephe_path(c.as_ptr()) };
    }
}

pub fn version() -> String {
    let mut buf = [0 as c_char; 256];
    let _guard = LOCK.lock().unwrap();
    unsafe {
        swe::swe_version(buf.as_mut_ptr());
        CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned()
    }
}

/// `true` quando os planetas estão saindo dos arquivos `.se1`, e não do Moshier embutido.
pub fn using_data_files() -> bool {
    matches!(calc(swe::SE_SUN, 2_451_545.0), Ok((_, flags)) if flags & swe::SEFLG_EPHMASK == swe::SEFLG_SWIEPH)
}

fn calc(ipl: c_int, jd_ut: f64) -> Result<([f64; 6], c_int), String> {
    let mut xx = [0.0f64; 6];
    let mut serr = [0 as c_char; 256];
    let _guard = LOCK.lock().unwrap();
    let ret = unsafe {
        swe::swe_calc_ut(
            jd_ut,
            ipl,
            swe::SEFLG_SWIEPH | swe::SEFLG_SPEED,
            xx.as_mut_ptr(),
            serr.as_mut_ptr(),
        )
    };
    if ret < 0 {
        Err(unsafe { CStr::from_ptr(serr.as_ptr()) }
            .to_string_lossy()
            .trim()
            .to_string())
    } else {
        Ok((xx, ret))
    }
}

impl Ephemeris for Swiss {
    fn engine(&self) -> Engine {
        Engine::Swiss
    }

    fn position(&self, body: Body, jd_ut: f64, node: NodeType) -> Result<Position, EphemError> {
        let ipl = match body {
            Body::Sun => swe::SE_SUN,
            Body::Moon => swe::SE_MOON,
            Body::Mercury => swe::SE_MERCURY,
            Body::Venus => swe::SE_VENUS,
            Body::Mars => swe::SE_MARS,
            Body::Jupiter => swe::SE_JUPITER,
            Body::Saturn => swe::SE_SATURN,
            Body::Uranus => swe::SE_URANUS,
            Body::Neptune => swe::SE_NEPTUNE,
            Body::Pluto => swe::SE_PLUTO,
            Body::Chiron => swe::SE_CHIRON,
            Body::NorthNode => match node {
                NodeType::True => swe::SE_TRUE_NODE,
                NodeType::Mean => swe::SE_MEAN_NODE,
            },
        };
        let (xx, _) = calc(ipl, jd_ut).map_err(|detail| EphemError::Engine { body, detail })?;
        Ok(Position {
            lon: xx[0],
            lat: xx[1],
            speed: xx[3],
            retrograde: xx[3] < 0.0,
        })
    }
}

/// Cúspides (casas 1–12), Ascendente e Meio do Céu pela própria Swiss Ephemeris, no zodíaco
/// tropical. Serve de referência para conferir o módulo `houses`.
pub fn houses(jd_ut: f64, lat: f64, lon: f64, system: HouseSystem) -> Option<([f64; 12], f64, f64)> {
    let code = match system {
        HouseSystem::Placidus => b'P',
        HouseSystem::Regiomontanus => b'R',
        HouseSystem::Porphyry => b'O',
        HouseSystem::Equal => b'A',
        HouseSystem::WholeSign => b'W',
    };
    let mut cusps = [0.0f64; 13];
    let mut ascmc = [0.0f64; 10];
    let _guard = LOCK.lock().unwrap();
    let ret = unsafe { swe::swe_houses(jd_ut, lat, lon, code as c_int, cusps.as_mut_ptr(), ascmc.as_mut_ptr()) };
    if ret < 0 {
        return None;
    }
    let mut out = [0.0; 12];
    out.copy_from_slice(&cusps[1..13]);
    Some((out, ascmc[0], ascmc[1]))
}
