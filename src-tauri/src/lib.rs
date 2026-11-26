//! Astrall: comandos expostos à interface.

mod db;
mod geo;
mod llm;
mod prompt;
mod secrets;

use astro_core::{chart::Chart, ChartInput};
use db::{BirthData, Reading, SavedChart};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{ipc::Channel, Manager, State};

struct AppState {
    db: Mutex<Connection>,
    http: reqwest::Client,
    /// Leituras em andamento, por id do pedido, para permitir cancelar.
    running: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

type CmdResult<T> = Result<T, String>;

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

/// Trânsitos: 18 meses a partir de hoje.
const TRANSIT_DAYS: u32 = 548;

fn chart_input(birth: &BirthData, time: &geo::TimeResolution) -> ChartInput {
    ChartInput {
        jd_ut: time.jd_ut,
        lat: birth.lat,
        lon: birth.lon,
        time_known: birth.time.is_some(),
        house_system: birth.house_system,
        zodiac: birth.zodiac,
        node_type: birth.node_type,
        engine: birth.engine,
        orbs: Default::default(),
        day_range: Some((time.day_start_jd, time.day_end_jd)),
    }
}

fn time_query(birth: &BirthData) -> geo::TimeQuery {
    geo::TimeQuery {
        date: birth.date.clone(),
        time: birth.time.clone(),
        tz: birth.tz.clone(),
        offset_override_min: birth.offset_override_min,
    }
}

fn validate(birth: &BirthData) -> CmdResult<()> {
    if !(-90.0..=90.0).contains(&birth.lat) || !(-180.0..=180.0).contains(&birth.lon) {
        return Err("Coordenadas fora do intervalo válido.".into());
    }
    Ok(())
}

/// Calcula o mapa a partir dos dados confirmados.
fn calculate(birth: &BirthData) -> CmdResult<(geo::TimeResolution, Chart)> {
    validate(birth)?;
    let time = geo::resolve_time(&time_query(birth))?;
    let chart = astro_core::compute_chart(&chart_input(birth, &time)).map_err(|e| match e {
        astro_core::EphemError::OutOfRange { min_year, max_year, .. } => {
            format!("Data fora do intervalo suportado pelas efemérides ({min_year} a {max_year}).")
        }
        astro_core::EphemError::EngineUnavailable { .. } => {
            "Este mapa pede a Swiss Ephemeris, que não foi incluída nesta versão do app. Edite o mapa e escolha o motor nativo.".into()
        }
        astro_core::EphemError::Engine { detail, .. } => format!("O motor de efemérides falhou: {detail}"),
    })?;
    Ok((time, chart))
}

fn now_jd() -> f64 {
    chrono::Utc::now().timestamp() as f64 / 86400.0 + 2_440_587.5
}

/// Motores de efemérides incluídos neste binário.
#[tauri::command]
fn list_engines() -> Vec<astro_core::Engine> {
    astro_core::engine::available()
}

#[tauri::command]
fn search_cities(query: String) -> Vec<geo::City> {
    geo::search(&query, 12)
}

/// Etapa de conferência: fuso em vigor na data, horário de verão e hora em UTC.
#[tauri::command]
fn preview_time(query: geo::TimeQuery) -> CmdResult<geo::TimeResolution> {
    geo::resolve_time(&query)
}

#[tauri::command]
fn create_chart(state: State<AppState>, birth: BirthData) -> CmdResult<SavedChart> {
    let (time, chart) = calculate(&birth)?;
    let conn = state.db.lock().unwrap();
    let id = db::insert_chart(
        &conn,
        &birth,
        &serde_json::to_value(&time).unwrap(),
        &serde_json::to_value(&chart).unwrap(),
    )
    .map_err(err)?;
    db::get_chart(&conn, id)
        .map_err(err)?
        .ok_or_else(|| "Mapa não encontrado.".into())
}

/// Recalcula um mapa salvo com dados ou opções novas (as leituras antigas ficam).
#[tauri::command]
fn update_chart(state: State<AppState>, id: i64, birth: BirthData) -> CmdResult<SavedChart> {
    let (time, chart) = calculate(&birth)?;
    let conn = state.db.lock().unwrap();
    db::update_chart(
        &conn,
        id,
        &birth,
        &serde_json::to_value(&time).unwrap(),
        &serde_json::to_value(&chart).unwrap(),
    )
    .map_err(err)?;
    db::get_chart(&conn, id)
        .map_err(err)?
        .ok_or_else(|| "Mapa não encontrado.".into())
}

#[tauri::command]
fn list_charts(state: State<AppState>, query: String) -> CmdResult<Vec<db::ChartSummary>> {
    db::list_charts(&state.db.lock().unwrap(), &query).map_err(err)
}

#[tauri::command]
fn get_chart(state: State<AppState>, id: i64) -> CmdResult<SavedChart> {
    db::get_chart(&state.db.lock().unwrap(), id)
        .map_err(err)?
        .ok_or_else(|| "Mapa não encontrado.".into())
}

#[tauri::command]
fn delete_chart(state: State<AppState>, id: i64) -> CmdResult<()> {
    db::delete_chart(&state.db.lock().unwrap(), id).map_err(err)
}

#[tauri::command]
fn delete_reading(state: State<AppState>, id: i64) -> CmdResult<()> {
    db::delete_reading(&state.db.lock().unwrap(), id).map_err(err)
}

fn load_chart(state: &AppState, id: i64) -> CmdResult<(SavedChart, Chart, ChartInput)> {
    let saved = db::get_chart(&state.db.lock().unwrap(), id)
        .map_err(err)?
        .ok_or("Mapa não encontrado.")?;
    let chart: Chart = serde_json::from_value(saved.chart.clone()).map_err(err)?;
    let time = geo::resolve_time(&time_query(&saved.birth))?;
    let input = chart_input(&saved.birth, &time);
    Ok((saved, chart, input))
}

/// Trânsitos de Júpiter a Plutão sobre o mapa, nos próximos 18 meses.
#[tauri::command(async)]
fn get_transits(state: State<AppState>, id: i64) -> CmdResult<astro_core::Transits> {
    let (_, _, input) = load_chart(&state, id)?;
    astro_core::compute_transits(&input, now_jd().floor() + 0.5, TRANSIT_DAYS).map_err(err)
}

#[derive(Serialize)]
struct PromptParts {
    system: String,
    user: String,
}

fn build_prompt_parts(state: &AppState, id: i64, themes: &[String]) -> CmdResult<PromptParts> {
    let (mut saved, chart, input) = load_chart(state, id)?;
    saved.birth.themes = themes.to_vec();
    let transits = if themes.iter().any(|t| t == "transitos") {
        Some(astro_core::compute_transits(&input, now_jd().floor() + 0.5, TRANSIT_DAYS).map_err(err)?)
    } else {
        None
    };
    let local = saved.time["local"].as_str().unwrap_or(&saved.birth.date).to_string();
    Ok(PromptParts {
        system: prompt::SYSTEM.into(),
        user: prompt::user_message(&saved.birth, &local, &chart, transits.as_ref()),
    })
}

/// O prompt completo, para colar em qualquer chat quando não há provedor configurado.
#[tauri::command(async)]
fn build_prompt(state: State<AppState>, id: i64, themes: Vec<String>) -> CmdResult<String> {
    let p = build_prompt_parts(&state, id, &themes)?;
    Ok(format!("{}\n\n---\n\n{}", p.system, p.user))
}

/// Salva uma leitura colada à mão (resposta de um chat externo).
#[tauri::command]
fn save_manual_reading(state: State<AppState>, id: i64, themes: Vec<String>, content: String) -> CmdResult<Reading> {
    if content.trim().is_empty() {
        return Err("A leitura está vazia.".into());
    }
    db::insert_reading(&state.db.lock().unwrap(), id, &themes, "manual", "", content.trim()).map_err(err)
}

#[derive(Serialize)]
struct SettingsView {
    llm: llm::LlmSettings,
    /// Provedores que já têm chave guardada.
    keys: Vec<String>,
}

fn llm_settings(conn: &Connection) -> llm::LlmSettings {
    let defaults = llm::LlmSettings::default();
    let Some(mut saved) = db::get_setting(conn, "llm")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str::<llm::LlmSettings>(&s).ok())
    else {
        return defaults;
    };
    // Provedores acrescentados em versões novas aparecem também para quem já tinha configuração.
    for p in defaults.providers {
        if !saved.providers.iter().any(|s| s.id == p.id) {
            saved.providers.push(p);
        }
    }
    saved
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> SettingsView {
    let conn = state.db.lock().unwrap();
    let llm = llm_settings(&conn);
    let keys = llm
        .providers
        .iter()
        .filter(|p| secrets::get(&conn, &p.id).is_some())
        .map(|p| p.id.clone())
        .collect();
    SettingsView { llm, keys }
}

#[tauri::command]
fn save_settings(state: State<AppState>, llm: llm::LlmSettings) -> CmdResult<()> {
    db::set_setting(&state.db.lock().unwrap(), "llm", &serde_json::to_string(&llm).unwrap()).map_err(err)
}

/// Grava (ou apaga, com valor vazio) a chave de um provedor. Devolve `false` quando o cofre do
/// sistema não estava disponível e a chave ficou no banco local.
#[tauri::command]
fn set_api_key(state: State<AppState>, provider: String, key: String) -> CmdResult<bool> {
    let conn = state.db.lock().unwrap();
    if key.trim().is_empty() {
        secrets::delete(&conn, &provider)?;
        Ok(true)
    } else {
        secrets::set(&conn, &provider, key.trim())
    }
}

fn provider_and_key(state: &AppState, id: Option<&str>) -> CmdResult<(llm::Provider, Option<String>)> {
    let conn = state.db.lock().unwrap();
    let settings = llm_settings(&conn);
    let id = id.unwrap_or(&settings.active);
    let provider = settings
        .providers
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .ok_or("Provedor não configurado.")?;
    let key = secrets::get(&conn, &provider.id);
    if provider.base_url.trim().is_empty() {
        return Err(format!(
            "Informe o endereço da API de \"{}\" nas configurações.",
            provider.label
        ));
    }
    if provider.needs_key && key.is_none() {
        return Err(format!(
            "Falta a chave de API de \"{}\". Adicione-a nas configurações.",
            provider.label
        ));
    }
    Ok((provider, key))
}

#[tauri::command]
async fn list_models(state: State<'_, AppState>, provider: llm::Provider) -> CmdResult<Vec<String>> {
    let key = secrets::get(&state.db.lock().unwrap(), &provider.id);
    llm::list_models(&state.http, &provider, key.as_deref()).await
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
enum StreamEvent {
    Delta { text: String },
}

/// Gera a leitura com o provedor ativo, transmitindo o texto, e salva no histórico.
#[tauri::command]
async fn generate_reading(
    state: State<'_, AppState>,
    id: i64,
    themes: Vec<String>,
    request_id: String,
    on_event: Channel<StreamEvent>,
) -> CmdResult<Reading> {
    let (provider, key) = provider_and_key(&state, None)?;
    let parts = build_prompt_parts(&state, id, &themes)?;
    let cancel = Arc::new(AtomicBool::new(false));
    state.running.lock().unwrap().insert(request_id.clone(), cancel.clone());
    let result = llm::stream(
        &state.http,
        &provider,
        key.as_deref(),
        &parts.system,
        &parts.user,
        &cancel,
        |t| {
            let _ = on_event.send(StreamEvent::Delta { text: t.to_string() });
        },
    )
    .await;
    state.running.lock().unwrap().remove(&request_id);
    let outcome = result?;
    let mut content = outcome.text;
    if outcome.truncated {
        content.push_str("\n\n*(A resposta foi cortada pelo limite de tamanho do modelo.)*");
    }
    db::insert_reading(
        &state.db.lock().unwrap(),
        id,
        &themes,
        &provider.label,
        &provider.model,
        &content,
    )
    .map_err(err)
}

#[tauri::command]
fn cancel_reading(state: State<AppState>, request_id: String) {
    if let Some(flag) = state.running.lock().unwrap().get(&request_id) {
        flag.store(true, Ordering::Relaxed);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let conn = db::open(&dir.join("astrall.db"))?;
            // Arquivos .se1 colocados em "ephe" dão à Swiss Ephemeris a precisão máxima e Quíron.
            #[cfg(feature = "swisseph")]
            {
                let ephe = dir.join("ephe");
                std::fs::create_dir_all(&ephe)?;
                astro_core::swiss::set_ephe_path(&ephe);
            }
            let http = reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(20))
                .user_agent(concat!("astrall/", env!("CARGO_PKG_VERSION")))
                .build()?;
            app.manage(AppState {
                db: Mutex::new(conn),
                http,
                running: Default::default(),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_engines,
            search_cities,
            preview_time,
            create_chart,
            update_chart,
            list_charts,
            get_chart,
            delete_chart,
            delete_reading,
            get_transits,
            build_prompt,
            save_manual_reading,
            get_settings,
            save_settings,
            set_api_key,
            list_models,
            generate_reading,
            cancel_reading,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Astrall");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn birth() -> BirthData {
        BirthData {
            name: "Ana".into(),
            date: "1988-08-08".into(),
            time: Some("10:30".into()),
            time_source: "certidao".into(),
            time_approximate: false,
            place: "São Paulo, São Paulo, Brasil".into(),
            lat: -23.5475,
            lon: -46.6361,
            tz: "America/Sao_Paulo".into(),
            offset_override_min: None,
            house_system: Default::default(),
            zodiac: Default::default(),
            node_type: Default::default(),
            engine: Default::default(),
            themes: vec!["personalidade".into(), "transitos".into()],
        }
    }

    /// O caminho inteiro de um mapa: calcular, salvar, listar, ler, anexar leitura e excluir.
    #[test]
    fn chart_lifecycle() {
        let conn = db::open(std::path::Path::new(":memory:")).unwrap();
        let b = birth();
        let (time, chart) = calculate(&b).unwrap();
        assert_eq!(time.utc, "1988-08-08 13:30");
        let id = db::insert_chart(
            &conn,
            &b,
            &serde_json::to_value(&time).unwrap(),
            &serde_json::to_value(&chart).unwrap(),
        )
        .unwrap();

        let list = db::list_charts(&conn, "sao paulo").unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!((list[0].sun_sign, list[0].asc_sign), (Some(4), Some(7)));
        assert!(db::list_charts(&conn, "lisboa").unwrap().is_empty());

        let saved = db::get_chart(&conn, id).unwrap().unwrap();
        let restored: Chart = serde_json::from_value(saved.chart).unwrap();
        assert_eq!(restored.points.len(), 14);

        let r = db::insert_reading(&conn, id, &b.themes, "Claude", "claude-opus-5-5", "# Leitura").unwrap();
        assert_eq!(db::list_charts(&conn, "").unwrap()[0].readings, 1);
        assert_eq!(db::get_chart(&conn, id).unwrap().unwrap().readings[0].id, r.id);

        db::delete_chart(&conn, id).unwrap();
        assert!(
            db::readings(&conn, id).unwrap().is_empty(),
            "as leituras saem junto com o mapa"
        );
    }

    #[test]
    fn prompt_carries_the_computed_chart() {
        let b = birth();
        let (time, chart) = calculate(&b).unwrap();
        let transits = astro_core::compute_transits(&chart_input(&b, &time), 2_461_371.5, 120).unwrap();
        let msg = prompt::user_message(&b, &time.local, &chart, Some(&transits));
        assert!(msg.contains("- Sol: 16°12' Leão, casa 10"), "{msg}");
        assert!(msg.contains("Regente do mapa: Plutão (Ascendente em Escorpião); regente tradicional: Marte"));
        assert!(msg.contains("Temas que quero aprofundar: Personalidade. Inclua também a seção de trânsitos."));
        assert!(msg.contains("Trânsitos de Júpiter a Plutão"));
    }

    #[test]
    fn rejects_dates_outside_the_tables() {
        let mut b = birth();
        b.date = "1750-01-01".into();
        assert!(calculate(&b).unwrap_err().contains("1800"));
    }
}
