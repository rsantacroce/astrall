<script lang="ts">
  import { api, type City } from "../lib/api";
  import { coords, countryName } from "../lib/astro";

  let { onpick }: { onpick: (city: City) => void } = $props();

  let query = $state("");
  let results = $state<City[]>([]);
  let searched = $state(false);
  let seq = 0;

  async function search() {
    const mine = ++seq;
    const found = query.trim().length >= 2 ? await api.searchCities(query) : [];
    // Respostas chegam fora de ordem enquanto se digita; vale só a da última consulta.
    if (mine === seq) {
      results = found;
      searched = query.trim().length >= 2;
    }
  }
</script>

<div class="stack">
  <label class="field">
    <span>Cidade de nascimento</span>
    <!-- svelte-ignore a11y_autofocus -->
    <input
      type="search"
      placeholder="Ex.: Santos, São Paulo"
      autocomplete="off"
      autocapitalize="words"
      autofocus
      bind:value={query}
      oninput={search}
    />
  </label>
  {#if results.length}
    <ul>
      {#each results as c}
        <li>
          <button type="button" onclick={() => onpick(c)}>
            <span><strong>{c.name}</strong>{c.admin ? `, ${c.admin}` : ""} · {countryName(c.country)}</span>
            <span class="muted small">{coords(c.lat, c.lon)} · {c.tz}</span>
          </button>
        </li>
      {/each}
    </ul>
  {:else if searched}
    <p class="muted small">
      Nenhuma cidade encontrada. Tente o nome sem abreviações, "cidade, estado", ou informe as coordenadas abaixo.
    </p>
  {/if}
</div>

<style>
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: 10px;
    overflow: hidden;
  }

  li + li {
    border-top: 1px solid var(--line);
  }

  li button {
    display: grid;
    gap: 0.1rem;
    width: 100%;
    padding: 0.6rem 0.8rem;
    border: 0;
    background: var(--surface);
    text-align: left;
    cursor: pointer;
  }

  li button:hover {
    background: var(--surface-2);
  }
</style>
