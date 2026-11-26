import type { AspectKind, Engine, HouseSystem, Zodiac } from "./api";

// U+FE0E força a forma de texto: sem ele o iOS desenha os glifos como emoji.
const T = "︎";

export const SIGNS = [
  "Áries", "Touro", "Gêmeos", "Câncer", "Leão", "Virgem",
  "Libra", "Escorpião", "Sagitário", "Capricórnio", "Aquário", "Peixes",
];
export const SIGN_GLYPHS = ["♈", "♉", "♊", "♋", "♌", "♍", "♎", "♏", "♐", "♑", "♒", "♓"].map((g) => g + T);
export const ELEMENTS = ["Fogo", "Terra", "Ar", "Água"];
export const ELEMENT_KEYS = ["fire", "earth", "air", "water"];
export const MODALITIES = ["Cardinal", "Fixo", "Mutável"];

export const POINTS: Record<string, { name: string; glyph: string }> = {
  sun: { name: "Sol", glyph: "☉" + T },
  moon: { name: "Lua", glyph: "☽" + T },
  mercury: { name: "Mercúrio", glyph: "☿" + T },
  venus: { name: "Vênus", glyph: "♀" + T },
  mars: { name: "Marte", glyph: "♂" + T },
  jupiter: { name: "Júpiter", glyph: "♃" + T },
  saturn: { name: "Saturno", glyph: "♄" + T },
  uranus: { name: "Urano", glyph: "♅" + T },
  neptune: { name: "Netuno", glyph: "♆" + T },
  pluto: { name: "Plutão", glyph: "♇" + T },
  north_node: { name: "Nodo Norte", glyph: "☊" + T },
  chiron: { name: "Quíron", glyph: "⚷" + T },
  asc: { name: "Ascendente", glyph: "AC" },
  mc: { name: "Meio do Céu", glyph: "MC" },
};

export const ASPECTS: Record<AspectKind, { name: string; glyph: string; tone: "neutral" | "soft" | "hard" }> = {
  conjunction: { name: "Conjunção", glyph: "☌" + T, tone: "neutral" },
  sextile: { name: "Sextil", glyph: "⚹" + T, tone: "soft" },
  square: { name: "Quadratura", glyph: "□", tone: "hard" },
  trine: { name: "Trígono", glyph: "△", tone: "soft" },
  opposition: { name: "Oposição", glyph: "☍" + T, tone: "hard" },
};

export const HOUSE_SYSTEMS: Record<HouseSystem, string> = {
  placidus: "Placidus",
  regiomontanus: "Regiomontanus",
  porphyry: "Porphyry",
  equal: "Casas iguais",
  whole_sign: "Signos inteiros",
};

export const ENGINES: Record<Engine, string> = {
  native: "Nativo (VSOP87, ELP-2000, JPL)",
  swiss: "Swiss Ephemeris",
};

export const ZODIACS: Record<Zodiac, string> = {
  tropical: "Tropical",
  sidereal_lahiri: "Sideral (Lahiri)",
  sidereal_fagan_bradley: "Sideral (Fagan-Bradley)",
};

export const THEMES: { key: string; label: string }[] = [
  { key: "personalidade", label: "Personalidade" },
  { key: "amor", label: "Amor" },
  { key: "carreira", label: "Carreira" },
  { key: "dinheiro", label: "Dinheiro" },
  { key: "familia", label: "Família" },
  { key: "saude", label: "Saúde" },
  { key: "proposito", label: "Propósito" },
  { key: "transitos", label: "Trânsitos atuais" },
];

/** 16.2 → "16°12′" (dentro do signo, sem passar de 29°59′). */
export function degMin(signDeg: number): string {
  const total = Math.min(Math.round(signDeg * 60), 30 * 60 - 1);
  return `${Math.floor(total / 60)}°${String(total % 60).padStart(2, "0")}′`;
}

export function signOf(lon: number): number {
  return Math.min(11, Math.floor((((lon % 360) + 360) % 360) / 30));
}

/** Longitude absoluta → "16°12′ Leão". */
export function position(lon: number): string {
  const s = signOf(lon);
  return `${degMin((((lon % 360) + 360) % 360) - s * 30)} ${SIGNS[s]}`;
}

/** "1988-08-08" → "08/08/1988" */
export function brDate(iso: string): string {
  const [y, m, d] = iso.slice(0, 10).split("-");
  return `${d}/${m}/${y}`;
}

export function jdToDate(jd: number): Date {
  return new Date((jd - 2440587.5) * 86400000);
}

/** Deslocamento em segundos → "UTC−03:00". */
export function utcOffset(seconds: number): string {
  const sign = seconds < 0 ? "−" : "+";
  const abs = Math.abs(seconds);
  const h = Math.floor(abs / 3600);
  const m = Math.floor((abs % 3600) / 60);
  const s = abs % 60;
  return `UTC${sign}${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}${s ? ":" + String(s).padStart(2, "0") : ""}`;
}

export function coords(lat: number, lon: number): string {
  const f = (v: number, pos: string, neg: string) => {
    const a = Math.abs(v);
    const d = Math.floor(a);
    const m = Math.round((a - d) * 60);
    return m === 60 ? `${d + 1}°00′ ${v >= 0 ? pos : neg}` : `${d}°${String(m).padStart(2, "0")}′ ${v >= 0 ? pos : neg}`;
  };
  return `${f(lat, "N", "S")}, ${f(lon, "L", "O")}`;
}

const regionNames = new Intl.DisplayNames(["pt-BR"], { type: "region" });
export function countryName(code: string): string {
  try {
    return regionNames.of(code) ?? code;
  } catch {
    return code;
  }
}
