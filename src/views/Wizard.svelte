<script lang="ts">
  import CitySearch from "../components/CitySearch.svelte";
  import { onMount } from "svelte";
  import { api, message, type BirthData, type City, type Engine, type SavedChart, type TimeResolution } from "../lib/api";
  import { ENGINES, HOUSE_SYSTEMS, THEMES, ZODIACS, brDate, coords, countryName, utcOffset } from "../lib/astro";

  let {
    initial,
    onsaved,
    oncancel,
  }: { initial?: SavedChart; onsaved: (chart: SavedChart) => void; oncancel: () => void } = $props();

  const STEPS = ["Nome", "Data", "Horário", "Local", "Temas", "Opções", "Conferência"];
  const CONFIRM = STEPS.length - 1;

  // svelte-ignore state_referenced_locally
  const from = initial?.birth;
  let birth = $state<BirthData>(
    from
      ? { ...from, engine: from.engine ?? "native", themes: [...from.themes] }
      : {
          name: "",
          date: "",
          time: "",
          time_source: "certidao",
          time_approximate: false,
          place: "",
          lat: 0,
          lon: 0,
          tz: "",
          offset_override_min: null,
          house_system: "placidus",
          zodiac: "tropical",
          node_type: "true",
          engine: "native",
          themes: ["personalidade"],
        },
  );
  let unknownTime = $state(from ? from.time === null : false);
  let placeChosen = $state(!!from);
  let manualPlace = $state(false);
  let step = $state(from ? CONFIRM : 0);
  let reached = $state(from ? CONFIRM : 0);

  let preview = $state<TimeResolution | null>(null);
  let overrideOn = $state(from ? from.offset_override_min !== null : false);
  let overrideHours = $state(from?.offset_override_min != null ? from.offset_override_min / 60 : -3);
  let error = $state("");
  let saving = $state(false);
  // A escolha do motor só aparece quando o app foi compilado com mais de um.
  let engines = $state<Engine[]>(["native"]);

  onMount(async () => {
    engines = await api.listEngines().catch(() => ["native"] as Engine[]);
  });

  const year = $derived(Number(birth.date.slice(0, 4)));
  const problem = $derived.by(() => {
    switch (step) {
      case 1:
        if (!/^\d{4}-\d{2}-\d{2}$/.test(birth.date)) return "Informe a data de nascimento.";
        if (year < 1800 || year > 2199) return "As efemérides embutidas cobrem de 1800 a 2199.";
        return "";
      case 2:
        return unknownTime || /^\d{2}:\d{2}$/.test(birth.time ?? "") ? "" : "Informe o horário, ou marque que não sabe.";
      case 3:
        if (!placeChosen && !manualPlace) return "Escolha a cidade na lista.";
        if (manualPlace) {
          if (!birth.place.trim()) return "Dê um nome ao local.";
          if (!(Math.abs(birth.lat) <= 90) || !(Math.abs(birth.lon) <= 180)) return "Coordenadas fora do intervalo.";
          if (!birth.tz.trim()) return "Informe o fuso horário (ex.: America/Sao_Paulo).";
        }
        return "";
      default:
        return "";
    }
  });

  function payload(): BirthData {
    return {
      ...birth,
      name: birth.name.trim(),
      time: unknownTime ? null : birth.time,
      time_source: unknownTime ? "desconhecido" : birth.time_source === "desconhecido" ? "certidao" : birth.time_source,
      time_approximate: unknownTime ? false : birth.time_approximate,
      offset_override_min: overrideOn ? Math.round(overrideHours * 60) : null,
    };
  }

  async function loadPreview() {
    error = "";
    try {
      const b = payload();
      preview = await api.previewTime({ date: b.date, time: b.time, tz: b.tz, offset_override_min: b.offset_override_min });
    } catch (e) {
      preview = null;
      error = message(e);
    }
  }

  function go(to: number) {
    error = "";
    step = to;
    reached = Math.max(reached, to);
    if (to === CONFIRM) loadPreview();
  }

  function next() {
    if (problem) {
      error = problem;
      return;
    }
    go(step + 1);
  }

  function pick(city: City) {
    birth.place = [city.name, city.admin, countryName(city.country)].filter(Boolean).join(", ");
    birth.lat = city.lat;
    birth.lon = city.lon;
    birth.tz = city.tz;
    placeChosen = true;
    manualPlace = false;
  }

  function startManual() {
    manualPlace = true;
    placeChosen = false;
    if (!birth.tz) birth.tz = Intl.DateTimeFormat().resolvedOptions().timeZone;
  }

  function toggleTheme(key: string) {
    birth.themes = birth.themes.includes(key) ? birth.themes.filter((t) => t !== key) : [...birth.themes, key];
  }

  async function confirm() {
    saving = true;
    error = "";
    try {
      onsaved(initial ? await api.updateChart(initial.id, payload()) : await api.createChart(payload()));
    } catch (e) {
      error = message(e);
    }
    saving = false;
  }

  if (from) loadPreview();
</script>

<form
  class="wizard stack"
  onsubmit={(e) => {
    e.preventDefault();
    if (step === CONFIRM) confirm();
    else next();
  }}
>
  <header class="row">
    <button type="button" class="btn ghost" onclick={oncancel}>Cancelar</button>
    <h2>{initial ? "Editar mapa" : "Novo mapa"}</h2>
  </header>

  <ol class="steps" aria-label="Etapas">
    {#each STEPS as label, i}
      <li>
        <button
          type="button"
          aria-label={`Etapa ${i + 1}: ${label}`}
          aria-current={i === step ? "step" : undefined}
          disabled={i > reached}
          onclick={() => go(i)}
        >
          <span class="n">{i + 1}</span><span class="l">{label}</span>
        </button>
      </li>
    {/each}
  </ol>

  <section class="card stack">
    {#if step === 0}
      <h3>Como devo chamar este mapa?</h3>
      <label class="field">
        <span>Nome (opcional, só para identificar no histórico)</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input type="text" autocapitalize="words" autofocus bind:value={birth.name} />
      </label>
    {:else if step === 1}
      <h3>Qual é a data de nascimento?</h3>
      <label class="field">
        <span>Dia, mês e ano</span>
        <input type="date" min="1800-01-01" max="2199-12-31" bind:value={birth.date} />
      </label>
    {:else if step === 2}
      <h3>A que horas foi o nascimento?</h3>
      <p class="muted">Quanto mais exato, melhor: o Ascendente anda cerca de 1° a cada 4 minutos.</p>
      {#if !unknownTime}
        <label class="field">
          <span>Horário local (o do relógio no lugar do nascimento)</span>
          <input type="time" bind:value={birth.time} />
        </label>
        <fieldset>
          <legend>De onde vem esse horário?</legend>
          <label><input type="radio" value="certidao" bind:group={birth.time_source} /> Certidão de nascimento</label>
          <label><input type="radio" value="memoria" bind:group={birth.time_source} /> Memória da família</label>
        </fieldset>
        <label><input type="checkbox" bind:checked={birth.time_approximate} /> É um horário aproximado</label>
      {:else}
        <p class="notice">
          Sem o horário, calculo o mapa para o meio-dia local. Ficam de fora o Ascendente, o Meio do Céu e as casas. A
          Lua anda cerca de 13° por dia, então a posição dela é aproximada e o signo pode ser incerto; o app avisa
          quando for o caso.
        </p>
      {/if}
      <label><input type="checkbox" bind:checked={unknownTime} /> Não sei o horário</label>
    {:else if step === 3}
      <h3>Onde foi o nascimento?</h3>
      {#if placeChosen && !manualPlace}
        <div class="picked">
          <div>
            <strong>{birth.place}</strong>
            <p class="muted small">{coords(birth.lat, birth.lon)} · {birth.tz}</p>
          </div>
          <button type="button" class="btn" onclick={() => (placeChosen = false)}>Trocar</button>
        </div>
      {:else if manualPlace}
        <label class="field"><span>Nome do local</span><input type="text" bind:value={birth.place} /></label>
        <div class="two">
          <label class="field">
            <span>Latitude (norte positivo)</span>
            <input type="number" step="any" min="-90" max="90" bind:value={birth.lat} />
          </label>
          <label class="field">
            <span>Longitude (leste positivo)</span>
            <input type="number" step="any" min="-180" max="180" bind:value={birth.lon} />
          </label>
        </div>
        <label class="field">
          <span>Fuso horário IANA</span>
          <input type="text" placeholder="America/Sao_Paulo" autocapitalize="off" spellcheck="false" bind:value={birth.tz} />
        </label>
        <button type="button" class="btn ghost" onclick={() => (manualPlace = false)}>Buscar cidade na lista</button>
      {:else}
        <CitySearch onpick={pick} />
        <button type="button" class="btn ghost" onclick={startManual}>Informar coordenadas manualmente</button>
      {/if}
    {:else if step === 4}
      <h3>Quais temas você quer aprofundar?</h3>
      <p class="muted">Eles orientam a leitura. Dá para mudar depois, a cada leitura gerada.</p>
      <div class="chips">
        {#each THEMES as t}
          <button type="button" class="chip" aria-pressed={birth.themes.includes(t.key)} onclick={() => toggleTheme(t.key)}>
            {t.label}
          </button>
        {/each}
      </div>
    {:else if step === 5}
      <h3>Opções de cálculo</h3>
      <p class="muted">Se você não tem preferência, os padrões (Placidus e tropical) são os mais usados.</p>
      <label class="field">
        <span>Sistema de casas</span>
        <select bind:value={birth.house_system} disabled={unknownTime}>
          {#each Object.entries(HOUSE_SYSTEMS) as [value, label]}<option {value}>{label}</option>{/each}
        </select>
      </label>
      <label class="field">
        <span>Zodíaco</span>
        <select bind:value={birth.zodiac}>
          {#each Object.entries(ZODIACS) as [value, label]}<option {value}>{label}</option>{/each}
        </select>
      </label>
      <label class="field">
        <span>Nodo lunar</span>
        <select bind:value={birth.node_type}>
          <option value="true">Verdadeiro</option>
          <option value="mean">Médio</option>
        </select>
      </label>
      {#if engines.length > 1 || birth.engine !== "native"}
        <label class="field">
          <span>Motor de efemérides</span>
          <select bind:value={birth.engine}>
            {#each Object.entries(ENGINES) as [value, label]}
              <option {value} disabled={!engines.includes(value as Engine)}>
                {label}{engines.includes(value as Engine) ? "" : " (não incluído nesta versão)"}
              </option>
            {/each}
          </select>
        </label>
      {/if}
    {:else}
      <h3>Confira antes de calcular</h3>
      {#if preview}
        <dl>
          {#if birth.name.trim()}<dt>Nome</dt><dd>{birth.name}</dd>{/if}
          <dt>Data</dt>
          <dd>{brDate(birth.date)}</dd>
          <dt>Hora local</dt>
          <dd>
            {#if unknownTime}
              desconhecida (meio-dia, sem casas)
            {:else}
              {birth.time} · {birth.time_source === "certidao" ? "certidão" : "memória"}{birth.time_approximate
                ? ", aproximada"
                : ""}
            {/if}
          </dd>
          <dt>Local</dt>
          <dd>{birth.place}</dd>
          <dt>Coordenadas</dt>
          <dd>{coords(birth.lat, birth.lon)}</dd>
          <dt>Fuso naquela data</dt>
          <dd>
            {utcOffset(preview.offset_seconds)}
            {#if !preview.overridden}
              <span class="muted">({birth.tz}{preview.abbreviation ? `, ${preview.abbreviation}` : ""})</span>
            {/if}
          </dd>
          <dt>Horário de verão</dt>
          <dd>{preview.overridden ? "definido manualmente" : preview.dst ? "sim, em vigor" : "não"}</dd>
          <dt>Hora em UTC</dt>
          <dd>{brDate(preview.utc)} {preview.utc.slice(11)}</dd>
        </dl>
        {#if preview.local_mean_time}
          <p class="notice">
            Nesta data ainda não havia hora legal neste fuso: o banco de fusos usa a hora média local da cidade de
            referência. Se a certidão segue outro padrão, corrija o deslocamento abaixo.
          </p>
        {/if}
        {#if preview.ambiguous}
          <p class="notice">
            Este horário aconteceu duas vezes (fim do horário de verão). Usei a primeira ocorrência, ainda em horário
            de verão; se foi a segunda, corrija o deslocamento abaixo.
          </p>
        {/if}
        {#if preview.nonexistent}
          <p class="notice">
            Este horário não existiu no relógio local (o horário de verão começou e pulou essa hora). Usei o
            deslocamento de antes do salto; confira o horário.
          </p>
        {/if}
        <label><input type="checkbox" bind:checked={overrideOn} onchange={loadPreview} /> Corrigir o deslocamento manualmente</label>
        {#if overrideOn}
          <label class="field">
            <span>Horas em relação a UTC (ex.: −3 para Brasília, −2 no horário de verão)</span>
            <input type="number" step="0.25" min="-14" max="14" bind:value={overrideHours} oninput={loadPreview} />
          </label>
        {/if}
      {:else if !error}
        <p class="muted">Verificando o fuso horário…</p>
      {/if}
    {/if}

    {#if error}<p class="notice error" role="alert">{error}</p>{/if}

    <footer class="row">
      {#if step > 0}<button type="button" class="btn" onclick={() => go(step - 1)}>Voltar</button>{/if}
      <span class="grow"></span>
      {#if step === CONFIRM}
        <button class="btn primary" disabled={!preview || saving}>
          {saving ? "Calculando…" : initial ? "Confirmar e recalcular" : "Confirmar e calcular"}
        </button>
      {:else}
        <button class="btn primary">Continuar</button>
      {/if}
    </footer>
  </section>
</form>

<style>
  .wizard {
    max-width: 38rem;
    margin: 0 auto;
  }

  .steps {
    display: flex;
    gap: 0.25rem;
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-x: auto;
  }

  .steps button {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    min-height: 36px;
    padding: 0 0.55rem;
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
    white-space: nowrap;
  }

  .steps button:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .steps .n {
    display: grid;
    place-items: center;
    width: 1.5rem;
    height: 1.5rem;
    border: 1px solid var(--line);
    border-radius: 50%;
    font-size: 0.8rem;
  }

  .steps button[aria-current="step"] {
    color: var(--text);
    background: var(--surface-2);
  }

  .steps button[aria-current="step"] .n {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
    font-weight: 700;
  }

  /* Só a etapa atual mostra o nome; as outras ficam no número (o nome vai no rótulo acessível). */
  .steps button:not([aria-current="step"]) .l {
    display: none;
  }

  fieldset {
    display: grid;
    gap: 0.4rem;
    margin: 0;
    padding: 0;
    border: 0;
  }

  legend {
    padding: 0;
    margin-bottom: 0.35rem;
    font-size: 0.85rem;
    color: var(--muted);
  }

  .two {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
  }

  .picked {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    padding: 0.8rem;
    border: 1px solid var(--accent);
    border-radius: 10px;
  }

  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.45rem 1.25rem;
    margin: 0;
  }

  dt {
    color: var(--muted);
  }

  dd {
    margin: 0;
    font-variant-numeric: tabular-nums;
  }

  .grow {
    flex: 1;
  }

  footer {
    margin-top: 0.5rem;
  }
</style>
