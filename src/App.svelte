<script lang="ts">
  import { onMount } from "svelte";
  import { api, message, type ChartSummary, type SavedChart } from "./lib/api";
  import { SIGN_GLYPHS, SIGNS, brDate } from "./lib/astro";
  import ChartView from "./views/ChartView.svelte";
  import Settings from "./views/Settings.svelte";
  import Wizard from "./views/Wizard.svelte";

  type View =
    | { name: "home" }
    | { name: "new" }
    | { name: "edit"; chart: SavedChart }
    | { name: "chart"; id: number }
    | { name: "settings" };

  let view = $state<View>({ name: "home" });
  let charts = $state<ChartSummary[]>([]);
  let query = $state("");
  let error = $state("");
  let loaded = $state(false);

  async function refresh() {
    try {
      charts = await api.listCharts(query);
      error = "";
    } catch (e) {
      error = message(e);
    }
    loaded = true;
  }

  onMount(refresh);

  function open(id: number) {
    view = { name: "chart", id };
  }

  function saved(chart: SavedChart) {
    refresh();
    open(chart.id);
  }
</script>

<div class="shell" data-view={view.name}>
  <aside>
    <header>
      <h1>Astrall</h1>
      <button class="btn ghost icon" aria-label="Configurações" onclick={() => (view = { name: "settings" })}>⚙</button>
    </header>
    <button class="btn primary" onclick={() => (view = { name: "new" })}>Novo mapa</button>
    <input type="search" placeholder="Buscar por nome, cidade ou data" bind:value={query} oninput={refresh} />
    {#if error}<p class="notice error">{error}</p>{/if}
    <nav aria-label="Mapas salvos">
      {#each charts as c (c.id)}
        <button class="item" aria-current={view.name === "chart" && view.id === c.id} onclick={() => open(c.id)}>
          <span class="name">{c.name || "Sem nome"}</span>
          <span class="signs glyph" title="Sol, Lua e Ascendente">
            {#each [c.sun_sign, c.moon_sign, c.asc_sign] as s}
              {#if s !== null}<span title={SIGNS[s]}>{SIGN_GLYPHS[s]}</span>{/if}
            {/each}
          </span>
          <span class="meta muted small">
            {brDate(c.date)}{c.time ? ` · ${c.time}` : ""} · {c.place}
          </span>
        </button>
      {:else}
        {#if loaded}
          <p class="muted small empty">
            {query ? "Nenhum mapa encontrado." : "Cada mapa que você calcular fica salvo aqui."}
          </p>
        {/if}
      {/each}
    </nav>
  </aside>

  <main>
    {#if view.name === "new"}
      <Wizard onsaved={saved} oncancel={() => (view = { name: "home" })} />
    {:else if view.name === "edit"}
      {@const editing = view.chart}
      <Wizard initial={editing} onsaved={saved} oncancel={() => open(editing.id)} />
    {:else if view.name === "chart"}
      {#key view.id}
        <ChartView
          id={view.id}
          onback={() => (view = { name: "home" })}
          onedit={(chart) => (view = { name: "edit", chart })}
          ondeleted={() => {
            view = { name: "home" };
            refresh();
          }}
          onsettings={() => (view = { name: "settings" })}
          onchanged={refresh}
        />
      {/key}
    {:else if view.name === "settings"}
      <Settings onback={() => (view = { name: "home" })} />
    {:else}
      <div class="welcome">
        <p class="glyph mark" aria-hidden="true">☉ ☽ ↑</p>
        <h2>Seu mapa astral, calculado aqui mesmo</h2>
        <p class="muted">
          Posições calculadas com efemérides astronômicas no seu aparelho, sem depender de internet. Cada mapa e cada
          leitura ficam guardados no histórico.
        </p>
        <button class="btn primary" onclick={() => (view = { name: "new" })}>Novo mapa</button>
      </div>
    {/if}
  </main>
</div>

<style>
  .shell {
    display: grid;
    grid-template-columns: 300px 1fr;
    height: 100%;
  }

  aside {
    display: grid;
    grid-template-rows: auto auto auto auto 1fr;
    gap: 0.75rem;
    padding: calc(1rem + env(safe-area-inset-top)) 1rem 0 calc(1rem + env(safe-area-inset-left));
    border-right: 1px solid var(--line);
    background: var(--surface);
    min-height: 0;
  }

  aside header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  aside h1 {
    color: var(--accent);
    letter-spacing: 0.02em;
  }

  .icon {
    width: 40px;
    padding: 0;
    font-size: 1.1rem;
  }

  nav {
    overflow-y: auto;
    margin: 0 -1rem;
    padding: 0 0.5rem calc(1rem + env(safe-area-inset-bottom));
  }

  .item {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0.1rem 0.5rem;
    width: 100%;
    padding: 0.6rem 0.6rem;
    border: 0;
    border-radius: 10px;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .item:hover {
    background: var(--surface-2);
  }

  .item[aria-current="true"] {
    background: var(--surface-2);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .signs {
    color: var(--accent);
    letter-spacing: 0.15em;
  }

  .meta {
    grid-column: 1 / -1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .empty {
    padding: 0.5rem 0.6rem;
  }

  main {
    min-width: 0;
    overflow-y: auto;
    padding: calc(1.5rem + env(safe-area-inset-top)) calc(1.5rem + env(safe-area-inset-right))
      calc(2rem + env(safe-area-inset-bottom)) 1.5rem;
  }

  .welcome {
    display: grid;
    gap: 1rem;
    justify-items: start;
    max-width: 30rem;
    margin: 12vh auto 0;
  }

  .mark {
    font-size: 2rem;
    color: var(--accent);
    letter-spacing: 0.3em;
  }

  /* Telas estreitas: a lista é a tela inicial e cada visão ocupa a tela toda. */
  @media (max-width: 760px) {
    .shell {
      grid-template-columns: 1fr;
    }

    aside {
      border-right: 0;
      padding-right: calc(1rem + env(safe-area-inset-right));
    }

    .shell:not([data-view="home"]) aside,
    .shell[data-view="home"] main {
      display: none;
    }

    main {
      padding: calc(1rem + env(safe-area-inset-top)) calc(1rem + env(safe-area-inset-right))
        calc(1.5rem + env(safe-area-inset-bottom)) calc(1rem + env(safe-area-inset-left));
    }
  }
</style>
