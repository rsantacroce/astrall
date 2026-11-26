import { Channel, invoke } from "@tauri-apps/api/core";

export type HouseSystem = "placidus" | "regiomontanus" | "porphyry" | "equal" | "whole_sign";
export type Zodiac = "tropical" | "sidereal_lahiri" | "sidereal_fagan_bradley";
export type NodeType = "true" | "mean";
export type Engine = "native" | "swiss";
export type AspectKind = "conjunction" | "sextile" | "square" | "trine" | "opposition";

export interface City {
  name: string;
  admin: string;
  country: string;
  lat: number;
  lon: number;
  tz: string;
  population: number;
}

export interface BirthData {
  name: string;
  date: string;
  time: string | null;
  time_source: "certidao" | "memoria" | "desconhecido";
  time_approximate: boolean;
  place: string;
  lat: number;
  lon: number;
  tz: string;
  offset_override_min: number | null;
  house_system: HouseSystem;
  zodiac: Zodiac;
  node_type: NodeType;
  engine: Engine;
  themes: string[];
}

export interface TimeResolution {
  local: string;
  utc: string;
  jd_ut: number;
  offset_seconds: number;
  dst: boolean;
  abbreviation: string;
  local_mean_time: boolean;
  ambiguous: boolean;
  nonexistent: boolean;
  overridden: boolean;
}

export interface Point {
  key: string;
  lon: number;
  lat: number;
  speed: number;
  retrograde: boolean;
  sign: number;
  sign_deg: number;
  house: number | null;
  sign_uncertain: boolean;
}

export interface Aspect {
  a: string;
  b: string;
  kind: AspectKind;
  orb: number;
  applying: boolean | null;
}

export interface Stellium {
  kind: "sign" | "house";
  index: number;
  members: string[];
}

export interface Chart {
  jd_ut: number;
  delta_t: number;
  engine: Engine;
  zodiac: Zodiac;
  ayanamsa: number;
  obliquity: number;
  sidereal_time: number | null;
  points: Point[];
  houses: { system: HouseSystem; asc: number; mc: number; cusps: number[]; fallback: boolean } | null;
  aspects: Aspect[];
  highlights: {
    elements: number[];
    modalities: number[];
    dominant_elements: string[];
    dominant_modalities: string[];
    stelliums: Stellium[];
    chart_ruler: { asc_sign: number; ruler: string; traditional: string | null } | null;
    moon_phase_angle: number;
    moon_phase: string;
    day_chart: boolean | null;
  };
  warnings: string[];
}

export interface Reading {
  id: number;
  chart_id: number;
  themes: string[];
  provider: string;
  model: string;
  content: string;
  created_at: string;
}

export interface SavedChart {
  id: number;
  birth: BirthData;
  time: TimeResolution;
  chart: Chart;
  created_at: string;
  readings: Reading[];
}

export interface ChartSummary {
  id: number;
  name: string;
  date: string;
  time: string | null;
  place: string;
  sun_sign: number | null;
  moon_sign: number | null;
  asc_sign: number | null;
  readings: number;
  created_at: string;
}

export interface TransitHit {
  jd_ut: number;
  transiting: string;
  natal: string;
  kind: AspectKind;
  retrograde: boolean;
  pass: number;
  passes: number;
}

export interface Transits {
  start_jd_ut: number;
  days: number;
  hits: TransitHit[];
  active: { transiting: string; natal: string; kind: AspectKind; orb: number; retrograde: boolean }[];
}

export interface Provider {
  id: string;
  label: string;
  kind: "anthropic" | "openai_compat";
  base_url: string;
  model: string;
  needs_key: boolean;
}

export interface LlmSettings {
  active: string;
  providers: Provider[];
}

export const api = {
  listEngines: () => invoke<Engine[]>("list_engines"),
  searchCities: (query: string) => invoke<City[]>("search_cities", { query }),
  previewTime: (query: { date: string; time: string | null; tz: string; offset_override_min: number | null }) =>
    invoke<TimeResolution>("preview_time", { query }),
  createChart: (birth: BirthData) => invoke<SavedChart>("create_chart", { birth }),
  updateChart: (id: number, birth: BirthData) => invoke<SavedChart>("update_chart", { id, birth }),
  listCharts: (query = "") => invoke<ChartSummary[]>("list_charts", { query }),
  getChart: (id: number) => invoke<SavedChart>("get_chart", { id }),
  deleteChart: (id: number) => invoke<void>("delete_chart", { id }),
  deleteReading: (id: number) => invoke<void>("delete_reading", { id }),
  getTransits: (id: number) => invoke<Transits>("get_transits", { id }),
  buildPrompt: (id: number, themes: string[]) => invoke<string>("build_prompt", { id, themes }),
  saveManualReading: (id: number, themes: string[], content: string) =>
    invoke<Reading>("save_manual_reading", { id, themes, content }),
  getSettings: () => invoke<{ llm: LlmSettings; keys: string[] }>("get_settings"),
  saveSettings: (llm: LlmSettings) => invoke<void>("save_settings", { llm }),
  setApiKey: (provider: string, key: string) => invoke<boolean>("set_api_key", { provider, key }),
  listModels: (provider: Provider) => invoke<string[]>("list_models", { provider }),
  cancelReading: (requestId: string) => invoke<void>("cancel_reading", { requestId }),
  generateReading(id: number, themes: string[], requestId: string, onDelta: (text: string) => void) {
    const onEvent = new Channel<{ type: "delta"; text: string }>();
    onEvent.onmessage = (e) => onDelta(e.text);
    return invoke<Reading>("generate_reading", { id, themes, requestId, onEvent });
  },
};

/** Mensagem legível de um erro vindo do backend. */
export function message(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
}
