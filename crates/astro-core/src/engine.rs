//! Camada de motores de efemérides.
//!
//! O resto do crate (mapa, trânsitos) só conhece o trait [`Ephemeris`]. Cada motor responde
//! a mesma pergunta — posição geocêntrica aparente, tropical, de um corpo num instante UT —
//! e pode ser trocado por mapa. Um motor novo é um tipo que implementa o trait e uma linha
//! em [`get`].

use crate::ephem::{self, Body, EphemError, Position};
use crate::zodiac::NodeType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Engine {
    /// Rust puro: VSOP87, ELP-2000/82 e tabelas do JPL Horizons. Licença livre.
    #[default]
    Native,
    /// Swiss Ephemeris (Astrodienst), só quando compilado com o recurso `swisseph`. AGPL.
    Swiss,
}

pub trait Ephemeris: Sync {
    fn engine(&self) -> Engine;

    /// Posição aparente (eclíptica e equinócio verdadeiros da data) com velocidade.
    fn position(&self, body: Body, jd_ut: f64, node: NodeType) -> Result<Position, EphemError>;

    /// Só a longitude; os motores que conseguem evitam o custo da velocidade.
    fn longitude(&self, body: Body, jd_ut: f64, node: NodeType) -> Result<f64, EphemError> {
        Ok(self.position(body, jd_ut, node)?.lon)
    }
}

pub struct Native;

impl Ephemeris for Native {
    fn engine(&self) -> Engine {
        Engine::Native
    }

    fn position(&self, body: Body, jd_ut: f64, node: NodeType) -> Result<Position, EphemError> {
        ephem::position(body, jd_ut, node)
    }

    fn longitude(&self, body: Body, jd_ut: f64, node: NodeType) -> Result<f64, EphemError> {
        ephem::longitude(body, jd_ut, node)
    }
}

/// O motor pedido, ou erro se ele não foi compilado neste binário.
pub fn get(engine: Engine) -> Result<&'static dyn Ephemeris, EphemError> {
    match engine {
        Engine::Native => Ok(&Native),
        #[cfg(feature = "swisseph")]
        Engine::Swiss => Ok(&crate::swiss::Swiss),
        #[cfg(not(feature = "swisseph"))]
        Engine::Swiss => Err(EphemError::EngineUnavailable { engine }),
    }
}

/// Motores presentes neste binário.
pub fn available() -> Vec<Engine> {
    [Engine::Native, Engine::Swiss]
        .into_iter()
        .filter(|e| get(*e).is_ok())
        .collect()
}

/// Um motor com rede de segurança: quando ele não tem dados para um corpo (a Swiss Ephemeris
/// sem o arquivo de asteroides não calcula Quíron), o motor nativo responde no lugar e o
/// corpo fica anotado em `substituted`, para o mapa avisar.
pub struct WithFallback {
    primary: &'static dyn Ephemeris,
    substituted: std::sync::Mutex<Vec<Body>>,
}

impl WithFallback {
    pub fn new(engine: Engine) -> Result<Self, EphemError> {
        Ok(WithFallback {
            primary: get(engine)?,
            substituted: Default::default(),
        })
    }

    pub fn engine(&self) -> Engine {
        self.primary.engine()
    }

    /// Corpos que o motor nativo calculou no lugar do motor escolhido.
    pub fn substituted(&self) -> Vec<Body> {
        self.substituted.lock().unwrap().clone()
    }

    fn or_native<T>(
        &self,
        body: Body,
        result: Result<T, EphemError>,
        native: impl FnOnce() -> Result<T, EphemError>,
    ) -> Result<T, EphemError> {
        match result {
            Err(EphemError::Engine { .. }) if self.primary.engine() != Engine::Native => {
                let value = native()?;
                let mut list = self.substituted.lock().unwrap();
                if !list.contains(&body) {
                    list.push(body);
                }
                Ok(value)
            }
            other => other,
        }
    }

    pub fn position(&self, body: Body, jd_ut: f64, node: NodeType) -> Result<Position, EphemError> {
        self.or_native(body, self.primary.position(body, jd_ut, node), || {
            Native.position(body, jd_ut, node)
        })
    }

    pub fn longitude(&self, body: Body, jd_ut: f64, node: NodeType) -> Result<f64, EphemError> {
        self.or_native(body, self.primary.longitude(body, jd_ut, node), || {
            Native.longitude(body, jd_ut, node)
        })
    }
}
