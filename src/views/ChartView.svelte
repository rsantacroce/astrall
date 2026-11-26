<script lang="ts">
  import { onMount } from "svelte";
  import Markdown from "../components/Markdown.svelte";
  import Wheel from "../components/Wheel.svelte";
  import { api, message, type Reading, type SavedChart, type Transits } from "../lib/api";
  import {
    ASPECTS,
    ENGINES,
    ELEMENTS,
    ELEMENT_KEYS,
    HOUSE_SYSTEMS,
    MODALITIES,
    POINTS,
    SIGNS,
    SIGN_GLYPHS,
    THEMES,
    ZODIACS,
    brDate,
    coords,
    degMin,
    jdToDate,
    position,
    signOf,
    utcOffset,
  } from "../lib/astro";

  let {
    id,
    onback,
    onedit,
    ondeleted,
    onsettings,
    onchanged,
  }: {
    id: number;
    onback: () => void;
    onedit: (chart: SavedChart) => void;
    ondeleted: () => void;
    onsettings: () => void;
    onchanged: () => void;
  } = $props();

  const TABS = ["Mapa", "Aspectos", "Destaques", "Trânsitos", "Leitura"] as const;
  type Tab = (typeof TABS)[number];

  let saved = $state<SavedChart | null>(null);
  let tab = $state<Tab>("Mapa");
  let error = $state("");
  let confirmingDelete = $state(false);

  let transits = $state<Transits | null>(null);
  let transitsError = $state("");

  let themes = $state<string[]>([]);
  let provider = $state("");
  let streaming = $state(false);
  let draft = $state("");
  let readError = $state("");
  let requestId = "";
  let copied = $state(false);
  let pasting = $state(false);
  let pasted = $state("");
  let openReading = $state<number | null>(null);

  const chart = $derived(saved?.chart ?? null);
  const point = (key: string) => chart?.points.find((p) => p.key === key);

  onMount(async () => {
    try {
      saved = await api.getChart(id);
      themes = [...saved.birth.themes];
      openReading = saved.readings[0]?.id ?? null;
      const s = await api.getSettings();
      const p = s.llm.providers.find((x) => x.id === s.llm.active);
      provider = p ? `${p.label}${p.model ? ` · ${p.model}` : ""}` : "";
    } catch (e) {
      error = message(e);
    }
  });

  async function show(t: Tab) {
    tab = t;
    if (t === "Trânsitos" && !transits && !transitsError) {
      try {
        transits = await api.getTransits(id);
      } catch (e) {
        transitsError = message(e);
      }
    }
  }

  const months = $derived.by(() => {
    const fmt = new Intl.DateTimeFormat("pt-BR", { month: "long", year: "numeric" });
    const groups: { label: string; hits: Transits["hits"] }[] = [];
    for (const h of transits?.hits ?? []) {
      const label = fmt.format(jdToDate(h.jd_ut));
      if (groups.at(-1)?.label !== label) groups.push({ label, hits: [] });
      groups.at(-1)!.hits.push(h);
    }
    return groups;
  });

  const dayFmt = new Intl.DateTimeFormat("pt-BR", { day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit" });
  const stampFmt = new Intl.DateTimeFormat("pt-BR", { dateStyle: "short", timeStyle: "short" });

  function toggleTheme(key: string) {
    themes = themes.includes(key) ? themes.filter((t) => t !== key) : [...themes, key];
  }

  function added(reading: Reading) {
    saved!.readings = [reading, ...saved!.readings];
    openReading = reading.id;
    onchanged();
  }

  async function generate() {
    streaming = true;
    draft = "";
    readError = "";
    requestId = crypto.randomUUID();
    try {
      added(await api.generateReading(id, $state.snapshot(themes), requestId, (t) => (draft += t)));
    } catch (e) {
      const msg = message(e);
      if (msg !== "cancelled") readError = msg;
    }
    draft = "";
    streaming = false;
  }

  async function copyPrompt() {
    readError = "";
    try {
      await navigator.clipboard.writeText(await api.buildPrompt(id, $state.snapshot(themes)));
      copied = true;
      setTimeout(() => (copied = false), 2500);
    } catch (e) {
      readError = message(e);
    }
  }

  async function savePasted() {
    readError = "";
    try {
      added(await api.saveManualReading(id, $state.snapshot(themes), pasted));
      pasted = "";
      pasting = false;
    } catch (e) {
      readError = message(e);
    }
  }

  async function removeReading(r: Reading) {
    try {
      await api.deleteReading(r.id);
      saved!.readings = saved!.readings.filter((x) => x.id !== r.id);
      onchanged();
    } catch (e) {
      readError = message(e);
    }
  }

  async function remove() {
    try {
      await api.deleteChart(id);
      ondeleted();
    } catch (e) {
      error = message(e);
    }
  }

  const themeLabel = (key: string) => THEMES.find((t) => t.key === key)?.label ?? key;
</script>

{#if error}
  <p class="notice error" role="alert">{error}</p>
  <button class="btn" onclick={onback}>Voltar</button>
{:else if saved && chart}
  {@const b = saved.birth}
  <div class="stack page">
    <header class="top">
      <button class="btn ghost back" onclick={onback}>Voltar</button>
      <div class="title">
        <h2>{b.name || "Mapa sem nome"}</h2>
        <p class="muted small">
          {brDate(b.date)}{b.time ? ` às ${b.time}` : ", horário desconhecido"} · {b.place}
        </p>
      </div>
      <div class="row">
        <button class="btn" onclick={() => onedit(saved!)}>Editar</button>
        {#if confirmingDelete}
          <button class="btn danger" onclick={remove}>Excluir mapa e leituras</button>
          <button class="btn ghost" onclick={() => (confirmingDelete = false)}>Manter</button>
        {:else}
          <button class="btn danger" onclick={() => (confirmingDelete = true)}>Excluir</button>
        {/if}
      </div>
    </header>

    <div class="tabs" role="tablist">
      {#each TABS as t}
        <button role="tab" aria-selected={tab === t} onclick={() => show(t)}>
          {t}{#if t === "Leitura" && saved.readings.length}<span class="count">{saved.readings.length}</span>{/if}
        </button>
      {/each}
    </div>

    {#each chart.warnings as w}<p class="notice">{w}</p>{/each}

    {#if tab === "Mapa"}
      <div class="map">
        <div class="stack">
          <Wheel {chart} />
          <div class="pillars">
            {#each ["sun", "moon", "asc"] as key}
              {@const p = point(key)}
              <div class="card pillar">
                <span class="muted small">{POINTS[key].name}</span>
                {#if p}
                  <span class="big"><span class="glyph">{SIGN_GLYPHS[p.sign]}</span> {SIGNS[p.sign]}</span>
                  <span class="muted small">
                    {degMin(p.sign_deg)}{p.house && key !== "asc" ? ` · casa ${p.house}` : ""}{p.sign_uncertain
                      ? " · signo incerto"
                      : ""}
                  </span>
                {:else}
                  <span class="big muted">—</span>
                  <span class="muted small">precisa do horário</span>
                {/if}
              </div>
            {/each}
          </div>
        </div>

        <div class="stack">
          <div class="card table-wrap">
            <table>
              <thead>
                <tr><th>Ponto</th><th>Posição</th><th>Casa</th><th><abbr title="Retrógrado">Retr.</abbr></th></tr>
              </thead>
              <tbody>
                {#each chart.points as p}
                  <tr>
                    <td><span class="glyph g">{POINTS[p.key].glyph}</span> {POINTS[p.key].name}</td>
                    <td>
                      {degMin(p.sign_deg)}
                      <span class={`glyph ${ELEMENT_KEYS[p.sign % 4]}`}>{SIGN_GLYPHS[p.sign]}</span>
                      {SIGNS[p.sign]}{p.sign_uncertain ? " ?" : ""}
                    </td>
                    <td>{p.house ?? "—"}</td>
                    <td>{p.key === "asc" || p.key === "mc" ? "—" : p.retrograde ? "sim" : "não"}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>

          {#if chart.houses}
            <div class="card table-wrap">
              <table>
                <thead>
                  <tr><th>Casa</th><th>Cúspide</th><th>Casa</th><th>Cúspide</th></tr>
                </thead>
                <tbody>
                  {#each [0, 1, 2, 3, 4, 5] as i}
                    <tr>
                      <td>{i + 1}</td>
                      <td>{position(chart.houses.cusps[i])}</td>
                      <td>{i + 7}</td>
                      <td>{position(chart.houses.cusps[i + 6])}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {/if}

          <p class="muted small">
            {coords(b.lat, b.lon)} · {utcOffset(saved.time.offset_seconds)}{saved.time.dst ? " (horário de verão)" : ""}
            · UTC {brDate(saved.time.utc)}
            {saved.time.utc.slice(11)} · {ZODIACS[chart.zodiac]}{chart.houses
              ? ` · ${HOUSE_SYSTEMS[chart.houses.system]}`
              : ""} · motor {ENGINES[chart.engine ?? "native"]}
          </p>
        </div>
      </div>
    {:else if tab === "Aspectos"}
      <div class="card table-wrap narrow">
        <table>
          <thead>
            <tr><th>Aspecto</th><th>Orbe</th><th>Fase</th></tr>
          </thead>
          <tbody>
            {#each chart.aspects as a}
              <tr>
                <td>
                  <span class="glyph g">{POINTS[a.a].glyph}</span>
                  {POINTS[a.a].name}
                  <span class={`glyph asp ${ASPECTS[a.kind].tone}`} title={ASPECTS[a.kind].name}>{ASPECTS[a.kind].glyph}</span>
                  <span class="muted">{ASPECTS[a.kind].name.toLowerCase()}</span>
                  <span class="glyph g">{POINTS[a.b].glyph}</span>
                  {POINTS[a.b].name}
                </td>
                <td>{a.orb.toFixed(1).replace(".", ",")}°</td>
                <td class="muted">{a.applying === null ? "—" : a.applying ? "aplicativo" : "separativo"}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      <p class="muted small">Orbe de até 8° com Sol ou Lua e 6° nos demais, do mais exato ao menos exato.</p>
    {:else if tab === "Destaques"}
      {@const hl = chart.highlights}
      <div class="grid2">
        <div class="card stack">
          <h3>Elementos</h3>
          {#each ELEMENTS as name, i}
            <div class="bar">
              <span>{name}</span>
              <span class="track"><span class={`fill ${ELEMENT_KEYS[i]}`} style:width={`${hl.elements[i] * 10}%`}></span></span>
              <span>{hl.elements[i]}</span>
            </div>
          {/each}
          <h3>Modalidades</h3>
          {#each MODALITIES as name, i}
            <div class="bar">
              <span>{name}</span>
              <span class="track"><span class="fill" style:width={`${hl.modalities[i] * 10}%`}></span></span>
              <span>{hl.modalities[i]}</span>
            </div>
          {/each}
          <p class="muted small">Contagem dos dez planetas, do Sol a Plutão.</p>
        </div>

        <div class="card stack">
          <h3>Regente do mapa</h3>
          {#if hl.chart_ruler}
            {@const r = point(hl.chart_ruler.ruler)}
            <p>
              <strong>{POINTS[hl.chart_ruler.ruler].name}</strong>, regente de {SIGNS[hl.chart_ruler.asc_sign]} (o
              Ascendente){r ? `, em ${position(r.lon)}, casa ${r.house}` : ""}.
              {#if hl.chart_ruler.traditional}
                Pela regência tradicional: {POINTS[hl.chart_ruler.traditional].name}.
              {/if}
            </p>
          {:else}
            <p class="muted">Depende do Ascendente, que precisa do horário de nascimento.</p>
          {/if}

          <h3>Stelliums</h3>
          {#each hl.stelliums as s}
            <p>
              <strong>{s.kind === "sign" ? SIGNS[s.index] : `Casa ${s.index}`}</strong>:
              {s.members.map((m) => POINTS[m].name).join(", ")}
            </p>
          {:else}
            <p class="muted">Nenhum signo ou casa reúne três ou mais planetas.</p>
          {/each}

          <h3>Aspectos mais exatos</h3>
          <ul class="plain">
            {#each chart.aspects.slice(0, 5) as a}
              <li>
                {POINTS[a.a].name}
                <span class="muted">{ASPECTS[a.kind].name.toLowerCase()}</span>
                {POINTS[a.b].name}
                <span class="muted">· {a.orb.toFixed(1).replace(".", ",")}°</span>
              </li>
            {/each}
          </ul>

          <h3>Fase da Lua e hora do dia</h3>
          <p>
            Nascimento na fase {hl.moon_phase}.
            {#if hl.day_chart !== null}Mapa {hl.day_chart ? "diurno (Sol acima do horizonte)" : "noturno (Sol abaixo do horizonte)"}.{/if}
          </p>
        </div>
      </div>
    {:else if tab === "Trânsitos"}
      {#if transitsError}
        <p class="notice error" role="alert">{transitsError}</p>
      {:else if !transits}
        <p class="muted">Calculando os trânsitos dos próximos 18 meses…</p>
      {:else}
        <p class="muted small">
          Júpiter a Plutão sobre os pontos natais, de {jdToDate(transits.start_jd_ut).toLocaleDateString("pt-BR")} a
          {jdToDate(transits.start_jd_ut + transits.days).toLocaleDateString("pt-BR")}. Datas e horas do aspecto exato,
          no fuso deste aparelho.
        </p>
        {#if transits.active.length}
          <div class="card stack">
            <h3>Já em vigor (dentro de 1°)</h3>
            <ul class="plain">
              {#each transits.active as a}
                <li>
                  <span class="glyph g">{POINTS[a.transiting].glyph}</span>
                  {POINTS[a.transiting].name}{a.retrograde ? " (R)" : ""}
                  <span class={`glyph asp ${ASPECTS[a.kind].tone}`}>{ASPECTS[a.kind].glyph}</span>
                  <span class="muted">{ASPECTS[a.kind].name.toLowerCase()}</span>
                  {POINTS[a.natal].name} natal
                  <span class="muted">· {a.orb.toFixed(1).replace(".", ",")}°</span>
                </li>
              {/each}
            </ul>
          </div>
        {/if}
        {#each months as m}
          <div class="card stack">
            <h3 class="month">{m.label}</h3>
            <ul class="plain">
              {#each m.hits as h}
                <li class="hit">
                  <span class="when">{dayFmt.format(jdToDate(h.jd_ut))}</span>
                  <span>
                    <span class="glyph g">{POINTS[h.transiting].glyph}</span>
                    {POINTS[h.transiting].name}{h.retrograde ? " (R)" : ""}
                    <span class={`glyph asp ${ASPECTS[h.kind].tone}`}>{ASPECTS[h.kind].glyph}</span>
                    <span class="muted">{ASPECTS[h.kind].name.toLowerCase()}</span>
                    {POINTS[h.natal].name} natal
                    {#if h.passes > 1}<span class="muted small">· passagem {h.pass} de {h.passes}</span>{/if}
                  </span>
                </li>
              {/each}
            </ul>
          </div>
        {:else}
          <p class="muted">Nenhum aspecto exato de planeta lento no período.</p>
        {/each}
      {/if}
    {:else}
      <div class="card stack">
        <h3>Nova leitura</h3>
        <div class="chips">
          {#each THEMES as t}
            <button class="chip" aria-pressed={themes.includes(t.key)} disabled={streaming} onclick={() => toggleTheme(t.key)}>
              {t.label}
            </button>
          {/each}
        </div>
        <div class="row">
          {#if streaming}
            <button class="btn" onclick={() => api.cancelReading(requestId)}>Parar</button>
          {:else}
            <button class="btn primary" onclick={generate}>Gerar leitura</button>
          {/if}
          <button class="btn" disabled={streaming} onclick={copyPrompt}>{copied ? "Prompt copiado" : "Copiar prompt"}</button>
          <button class="btn ghost" disabled={streaming} onclick={() => (pasting = !pasting)}>Colar leitura pronta</button>
        </div>
        <p class="muted small">
          {provider ? `Usando ${provider}.` : ""}
          <button class="link" onclick={onsettings}>Trocar provedor ou chave</button>
        </p>
        {#if readError}
          <p class="notice error" role="alert">{readError}</p>
        {/if}
        {#if pasting}
          <label class="field">
            <span>Cole aqui a resposta obtida em outro chat (com o prompt copiado) para guardar no histórico</span>
            <textarea bind:value={pasted}></textarea>
          </label>
          <div class="row"><button class="btn primary" disabled={!pasted.trim()} onclick={savePasted}>Salvar leitura</button></div>
        {/if}
      </div>

      {#if streaming}
        <div class="card stack" aria-live="polite">
          {#if draft}<Markdown source={draft} />{:else}<p class="muted">Lendo o mapa…</p>{/if}
        </div>
      {/if}

      {#each saved.readings as r (r.id)}
        <article class="card stack">
          <header class="reading-head">
            <button class="link strong" aria-expanded={openReading === r.id} onclick={() => (openReading = openReading === r.id ? null : r.id)}>
              {stampFmt.format(new Date(r.created_at))} · {r.themes.map(themeLabel).join(", ") || "visão geral"}
            </button>
            <span class="muted small">{r.provider === "manual" ? "colada à mão" : `${r.provider}${r.model ? ` · ${r.model}` : ""}`}</span>
            <button class="btn ghost danger small-btn" onclick={() => removeReading(r)}>Excluir</button>
          </header>
          {#if openReading === r.id}<Markdown source={r.content} />{/if}
        </article>
      {:else}
        {#if !streaming}
          <p class="muted">Nenhuma leitura ainda. As que você gerar ficam guardadas aqui, junto do mapa.</p>
        {/if}
      {/each}

      <p class="muted small">
        A astrologia é uma linguagem simbólica para reflexão. Não é previsão garantida nem substitui orientação médica,
        financeira ou jurídica.
      </p>
    {/if}
  </div>
{/if}

<style>
  .page {
    max-width: 68rem;
    margin: 0 auto;
    container-type: inline-size;
  }

  .top {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.75rem 1rem;
  }

  .title {
    flex: 1;
    min-width: 12rem;
  }

  .back {
    display: none;
  }

  .tabs {
    display: flex;
    gap: 0.25rem;
    border-bottom: 1px solid var(--line);
    overflow-x: auto;
  }

  .tabs button {
    min-height: 42px;
    padding: 0 0.9rem;
    border: 0;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
    white-space: nowrap;
  }

  .tabs button[aria-selected="true"] {
    color: var(--text);
    border-bottom-color: var(--accent);
    font-weight: 600;
  }

  .count {
    margin-left: 0.4rem;
    padding: 0 0.4rem;
    border-radius: 999px;
    background: var(--surface-2);
    font-size: 0.75rem;
  }

  .map {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 25.5rem);
    gap: 1.25rem;
    align-items: start;
  }

  .pillars {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.6rem;
  }

  .pillar {
    display: grid;
    gap: 0.15rem;
    padding: 0.7rem 0.6rem;
    min-width: 0;
  }

  .big {
    font-family: var(--serif);
    font-size: 1.05rem;
    white-space: nowrap;
  }

  /* As tabelas do mapa são densas: menos respiro que o padrão. */
  .map .table-wrap {
    padding: 0.4rem 0.6rem;
  }

  .map :global(th),
  .map :global(td) {
    padding: 0.45rem 0.4rem;
  }

  .big .glyph {
    color: var(--accent);
  }

  abbr {
    text-decoration: none;
  }

  .g {
    display: inline-block;
    min-width: 1.3em;
    text-align: center;
    font-size: 1.1em;
  }

  .asp {
    margin: 0 0.25rem;
  }

  .asp.soft {
    color: var(--soft);
  }

  .asp.hard {
    color: var(--hard);
  }

  .narrow {
    max-width: 44rem;
  }

  .grid2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.25rem;
    align-items: start;
  }

  .bar {
    display: grid;
    grid-template-columns: 5rem 1fr 1.5rem;
    gap: 0.6rem;
    align-items: center;
  }

  .track {
    height: 8px;
    border-radius: 4px;
    background: var(--surface-2);
    overflow: hidden;
  }

  .fill {
    display: block;
    height: 100%;
    border-radius: 4px;
    background: currentColor;
  }

  .fill:not(.fire, .earth, .air, .water) {
    color: var(--accent);
  }

  .plain {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.45rem;
  }

  .month::first-letter {
    text-transform: uppercase;
  }

  .hit {
    display: grid;
    grid-template-columns: 6.5rem 1fr;
    gap: 0.75rem;
  }

  .when {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }

  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 3px;
    text-align: left;
  }

  .link.strong {
    font-weight: 600;
    text-decoration: none;
    color: var(--text);
  }

  .reading-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem 1rem;
  }

  .reading-head .link {
    flex: 1;
    min-width: 12rem;
  }

  .small-btn {
    min-height: 32px;
    padding: 0 0.6rem;
    font-size: 0.85rem;
  }

  /* Pela largura da área do mapa, não da janela: a barra lateral varia. */
  @container (max-width: 800px) {
    .map,
    .grid2 {
      grid-template-columns: 1fr;
    }
  }

  @container (max-width: 420px) {
    .map .table-wrap {
      padding: 0.25rem 0.35rem;
      font-size: 0.9rem;
    }

    .map :global(th),
    .map :global(td) {
      padding: 0.45rem 0.25rem;
    }
  }

  @media (max-width: 760px) {
    .back {
      display: inline-flex;
    }

    .hit {
      grid-template-columns: 1fr;
      gap: 0;
    }
  }
</style>
