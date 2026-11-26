//! Histórico em SQLite: cada mapa calculado e cada leitura gerada ficam salvos.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Dados de nascimento e opções, como o usuário informou e confirmou.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirthData {
    pub name: String,
    /// "AAAA-MM-DD"
    pub date: String,
    /// "HH:MM"; `None` = horário desconhecido.
    pub time: Option<String>,
    /// "certidao", "memoria" ou "desconhecido".
    pub time_source: String,
    pub time_approximate: bool,
    pub place: String,
    pub lat: f64,
    pub lon: f64,
    pub tz: String,
    pub offset_override_min: Option<i32>,
    pub house_system: astro_core::HouseSystem,
    pub zodiac: astro_core::Zodiac,
    pub node_type: astro_core::NodeType,
    /// Motor de efemérides; mapas salvos antes desta opção usam o nativo.
    #[serde(default)]
    pub engine: astro_core::Engine,
    pub themes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SavedChart {
    pub id: i64,
    pub birth: BirthData,
    pub time: serde_json::Value,
    pub chart: serde_json::Value,
    pub created_at: String,
    pub readings: Vec<Reading>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChartSummary {
    pub id: i64,
    pub name: String,
    pub date: String,
    pub time: Option<String>,
    pub place: String,
    pub sun_sign: Option<usize>,
    pub moon_sign: Option<usize>,
    pub asc_sign: Option<usize>,
    pub readings: u32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Reading {
    pub id: i64,
    pub chart_id: i64,
    pub themes: Vec<String>,
    pub provider: String,
    pub model: String,
    pub content: String,
    pub created_at: String,
}

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS charts (
             id          INTEGER PRIMARY KEY,
             name        TEXT NOT NULL,
             birth_json  TEXT NOT NULL,
             time_json   TEXT NOT NULL,
             chart_json  TEXT NOT NULL,
             created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
         );
         CREATE TABLE IF NOT EXISTS readings (
             id          INTEGER PRIMARY KEY,
             chart_id    INTEGER NOT NULL REFERENCES charts(id) ON DELETE CASCADE,
             themes_json TEXT NOT NULL,
             provider    TEXT NOT NULL,
             model       TEXT NOT NULL,
             content     TEXT NOT NULL,
             created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
         );
         CREATE INDEX IF NOT EXISTS readings_chart ON readings(chart_id);
         CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
    )?;
    Ok(conn)
}

pub fn insert_chart(
    conn: &Connection,
    birth: &BirthData,
    time: &serde_json::Value,
    chart: &serde_json::Value,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO charts (name, birth_json, time_json, chart_json) VALUES (?1, ?2, ?3, ?4)",
        params![
            birth.name,
            serde_json::to_string(birth).unwrap(),
            time.to_string(),
            chart.to_string()
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_chart(
    conn: &Connection,
    id: i64,
    birth: &BirthData,
    time: &serde_json::Value,
    chart: &serde_json::Value,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE charts SET name = ?2, birth_json = ?3, time_json = ?4, chart_json = ?5 WHERE id = ?1",
        params![
            id,
            birth.name,
            serde_json::to_string(birth).unwrap(),
            time.to_string(),
            chart.to_string()
        ],
    )?;
    Ok(())
}

pub fn delete_chart(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM charts WHERE id = ?1", params![id])?;
    Ok(())
}

fn json(s: String) -> serde_json::Value {
    serde_json::from_str(&s).unwrap_or(serde_json::Value::Null)
}

pub fn get_chart(conn: &Connection, id: i64) -> rusqlite::Result<Option<SavedChart>> {
    let row = conn
        .query_row(
            "SELECT birth_json, time_json, chart_json, created_at FROM charts WHERE id = ?1",
            params![id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?;
    let Some((birth, time, chart, created_at)) = row else {
        return Ok(None);
    };
    let Ok(birth) = serde_json::from_str::<BirthData>(&birth) else {
        return Ok(None);
    };
    Ok(Some(SavedChart {
        id,
        birth,
        time: json(time),
        chart: json(chart),
        created_at,
        readings: readings(conn, id)?,
    }))
}

pub fn list_charts(conn: &Connection, query: &str) -> rusqlite::Result<Vec<ChartSummary>> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.birth_json, c.chart_json, c.created_at,
                (SELECT COUNT(*) FROM readings r WHERE r.chart_id = c.id)
         FROM charts c ORDER BY c.created_at DESC, c.id DESC",
    )?;
    let needle = crate::geo::normalize(query);
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, u32>(4)?,
        ))
    })?;
    let mut out = vec![];
    for row in rows {
        let (id, birth, chart, created_at, n) = row?;
        let Ok(birth) = serde_json::from_str::<BirthData>(&birth) else {
            continue;
        };
        if !needle.is_empty()
            && !crate::geo::normalize(&birth.name).contains(&needle)
            && !crate::geo::normalize(&birth.place).contains(&needle)
            && !birth.date.contains(&needle)
        {
            continue;
        }
        let chart = json(chart);
        let sign = |key: &str| {
            chart["points"]
                .as_array()?
                .iter()
                .find(|p| p["key"] == key)?
                .get("sign")?
                .as_u64()
                .map(|s| s as usize)
        };
        out.push(ChartSummary {
            id,
            name: birth.name,
            date: birth.date,
            time: birth.time,
            place: birth.place,
            sun_sign: sign("sun"),
            moon_sign: sign("moon"),
            asc_sign: sign("asc"),
            readings: n,
            created_at,
        });
    }
    Ok(out)
}

pub fn readings(conn: &Connection, chart_id: i64) -> rusqlite::Result<Vec<Reading>> {
    let mut stmt = conn.prepare(
        "SELECT id, themes_json, provider, model, content, created_at FROM readings
         WHERE chart_id = ?1 ORDER BY created_at DESC, id DESC",
    )?;
    let rows = stmt.query_map(params![chart_id], |r| {
        Ok(Reading {
            id: r.get(0)?,
            chart_id,
            themes: serde_json::from_str(&r.get::<_, String>(1)?).unwrap_or_default(),
            provider: r.get(2)?,
            model: r.get(3)?,
            content: r.get(4)?,
            created_at: r.get(5)?,
        })
    })?;
    rows.collect()
}

pub fn insert_reading(
    conn: &Connection,
    chart_id: i64,
    themes: &[String],
    provider: &str,
    model: &str,
    content: &str,
) -> rusqlite::Result<Reading> {
    conn.execute(
        "INSERT INTO readings (chart_id, themes_json, provider, model, content) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            chart_id,
            serde_json::to_string(themes).unwrap(),
            provider,
            model,
            content
        ],
    )?;
    let id = conn.last_insert_rowid();
    Ok(readings(conn, chart_id)?
        .into_iter()
        .find(|r| r.id == id)
        .expect("leitura recém-inserida"))
}

pub fn delete_reading(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM readings WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn get_setting(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", params![key], |r| r.get(0))
        .optional()
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn delete_setting(conn: &Connection, key: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
    Ok(())
}
