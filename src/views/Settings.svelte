<script lang="ts">
  import { onMount } from "svelte";
  import { api, message, type LlmSettings, type Provider } from "../lib/api";

  let { onback }: { onback: () => void } = $props();

  let settings = $state<LlmSettings | null>(null);
  let withKey = $state<string[]>([]);
  let keyInput = $state("");
  let models = $state<string[]>([]);
  let status = $state("");
  let error = $state("");
  let busy = $state(false);

  const active = $derived(settings?.providers.find((p) => p.id === settings!.active) ?? null);

  async function load() {
    const s = await api.getSettings();
    settings = s.llm;
    withKey = s.keys;
  }

  onMount(() => load().catch((e) => (error = message(e))));

  async function persist() {
    if (!settings) return;
    try {
      await api.saveSettings($state.snapshot(settings));
      error = "";
    } catch (e) {
      error = message(e);
    }
  }

  function choose(id: string) {
    settings!.active = id;
    keyInput = "";
    models = [];
    status = "";
    error = "";
    persist();
  }

  async function saveKey(p: Provider, value: string) {
    busy = true;
    error = "";
    try {
      const inVault = await api.setApiKey(p.id, value);
      keyInput = "";
      await load();
      status = !value
        ? "Chave removida."
        : inVault
          ? "Chave guardada no cofre do sistema."
          : "O cofre do sistema não está disponível; a chave ficou no banco de dados local do app, sem criptografia.";
    } catch (e) {
      error = message(e);
    }
    busy = false;
  }

  async function fetchModels(p: Provider) {
    busy = true;
    error = "";
    status = "";
    try {
      await persist();
      models = await api.listModels($state.snapshot(p));
      status = models.length ? `${models.length} modelos disponíveis; escolha na lista do campo.` : "O provedor não listou modelos.";
    } catch (e) {
      error = message(e);
    }
    busy = false;
  }
</script>

<div class="stack page">
  <header class="row">
    <button class="btn ghost" onclick={onback}>Voltar</button>
    <h2>Configurações</h2>
  </header>

  {#if settings && active}
    <section class="card stack">
      <h3>Quem escreve a interpretação</h3>
      <p class="muted">
        O cálculo do mapa é sempre local. Só a leitura em texto usa um modelo de linguagem: o app envia a ele as
        posições já calculadas, o nome que você deu ao mapa, a data e a cidade.
      </p>
      <div class="chips" role="group" aria-label="Provedor">
        {#each settings.providers as p}
          <button class="chip" aria-pressed={p.id === settings.active} onclick={() => choose(p.id)}>{p.label}</button>
        {/each}
      </div>

      <label class="field">
        <span>Endereço da API</span>
        <input
          type="url"
          placeholder="https://…/v1"
          autocapitalize="off"
          spellcheck="false"
          bind:value={active.base_url}
          onchange={persist}
        />
      </label>

      <label class="field">
        <span>Modelo</span>
        <input
          type="text"
          list="models"
          autocapitalize="off"
          spellcheck="false"
          placeholder={active.kind === "anthropic" ? "claude-opus-5-5" : "nome do modelo"}
          bind:value={active.model}
          onchange={persist}
        />
        <datalist id="models">
          {#each models as m}<option value={m}></option>{/each}
        </datalist>
      </label>
      <div class="row">
        <button class="btn" disabled={busy} onclick={() => fetchModels(active)}>Buscar modelos disponíveis</button>
      </div>

      {#if active.needs_key}
        <label class="field">
          <span>
            Chave de API
            {#if withKey.includes(active.id)}· já existe uma chave guardada{/if}
          </span>
          <input
            type="password"
            autocomplete="off"
            placeholder={withKey.includes(active.id) ? "•••••••• (digite para substituir)" : "cole a chave aqui"}
            bind:value={keyInput}
          />
        </label>
        <div class="row">
          <button class="btn primary" disabled={busy || !keyInput.trim()} onclick={() => saveKey(active, keyInput)}>
            Guardar chave
          </button>
          {#if withKey.includes(active.id)}
            <button class="btn danger" disabled={busy} onclick={() => saveKey(active, "")}>Remover chave</button>
          {/if}
        </div>
      {:else}
        <p class="muted small">Este provedor roda na sua máquina e não usa chave.</p>
      {/if}

      {#if active.kind === "anthropic"}
        <p class="muted small">
          Nos modelos Claude mais recentes o app ativa o recurso de "fallback" da API: se o modelo escolhido recusar um
          pedido, a própria Anthropic refaz a resposta com outro modelo, cobrado pela tabela desse outro modelo.
        </p>
      {/if}
      {#if status}<p class="notice" role="status">{status}</p>{/if}
      {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    </section>
  {:else if error}
    <p class="notice error" role="alert">{error}</p>
  {/if}

  <section class="card stack">
    <h3>Sobre os cálculos</h3>
    <p class="muted">
      Sol e planetas pela teoria VSOP87, Lua pela ELP-2000/82, Plutão e Quíron por posições do JPL Horizons
      interpoladas (1800 a 2199). Conferidas com o JPL, as posições diferem em menos de 2″ para os planetas e cerca de
      6″ para a Lua. O Nodo verdadeiro é o osculador (plano instantâneo da órbita da Lua) e confere com o JPL dentro de 2′. Fusos e horário de verão históricos vêm do banco IANA; cidades, do GeoNames (CC BY 4.0).
    </p>
    <p class="muted">
      A astrologia é uma linguagem simbólica para reflexão. Não é previsão garantida nem substitui orientação médica,
      financeira ou jurídica.
    </p>
  </section>
</div>

<style>
  .page {
    max-width: 40rem;
    margin: 0 auto;
  }
</style>
