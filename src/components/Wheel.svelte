<script lang="ts">
  import type { Chart } from "../lib/api";
  import { ASPECTS, ELEMENT_KEYS, POINTS, SIGN_GLYPHS, SIGNS, position } from "../lib/astro";

  let { chart }: { chart: Chart } = $props();

  const C = 300;
  const R = { outer: 292, signs: 250, glyph: 216, tick: 240, houseNum: 166, inner: 150 };

  // Com casas, o Ascendente fica à esquerda; sem elas, 0° de Áries.
  const rotation = $derived(chart.houses ? chart.houses.asc : 0);

  function xy(lon: number, r: number): [number, number] {
    const a = ((180 + lon - rotation) * Math.PI) / 180;
    return [C + r * Math.cos(a), C - r * Math.sin(a)];
  }

  function arc(from: number, to: number, r1: number, r2: number): string {
    const [ax, ay] = xy(from, r1);
    const [bx, by] = xy(to, r1);
    const [cx, cy] = xy(to, r2);
    const [dx, dy] = xy(from, r2);
    return `M${ax} ${ay} A${r1} ${r1} 0 0 0 ${bx} ${by} L${cx} ${cy} A${r2} ${r2} 0 0 1 ${dx} ${dy} Z`;
  }

  const bodies = $derived(chart.points.filter((p) => p.key !== "asc" && p.key !== "mc"));

  /** Afasta os glifos que cairiam uns sobre os outros; o traço continua na posição real. */
  const placed = $derived.by(() => {
    const MIN = 9;
    const items = bodies.map((p) => ({ p, at: p.lon })).sort((a, b) => a.at - b.at);
    for (let pass = 0; pass < 60; pass++) {
      let moved = false;
      for (let i = 0; i < items.length; i++) {
        const a = items[i];
        const b = items[(i + 1) % items.length];
        const gap = (((b.at - a.at) % 360) + 360) % 360;
        if (items.length > 1 && gap < MIN) {
          const push = (MIN - gap) / 2;
          a.at -= push;
          b.at += push;
          moved = true;
        }
      }
      if (!moved) break;
    }
    return items;
  });

  const lines = $derived(
    chart.aspects
      .filter((a) => a.kind !== "conjunction" && !["asc", "mc"].includes(a.a) && !["asc", "mc"].includes(a.b))
      .map((a) => {
        const lon = (key: string) => chart.points.find((p) => p.key === key)!.lon;
        const [x1, y1] = xy(lon(a.a), R.inner);
        const [x2, y2] = xy(lon(a.b), R.inner);
        return { x1, y1, x2, y2, tone: ASPECTS[a.kind].tone, opacity: Math.max(0.25, 1 - a.orb / 8) };
      }),
  );

  const summary = $derived(
    chart.points.map((p) => `${POINTS[p.key].name} em ${position(p.lon)}`).join("; "),
  );
</script>

<svg viewBox="0 0 600 600" role="img" aria-label={`Roda do mapa astral. ${summary}`}>
  {#each SIGNS as _, i}
    {@const [gx, gy] = xy(i * 30 + 15, (R.outer + R.signs) / 2)}
    <path class={`sign ${ELEMENT_KEYS[i % 4]}`} d={arc(i * 30, i * 30 + 30, R.outer, R.signs)} />
    <text class={`glyph sign-glyph ${ELEMENT_KEYS[i % 4]}`} x={gx} y={gy}>{SIGN_GLYPHS[i]}</text>
  {/each}
  <circle class="ring" cx={C} cy={C} r={R.outer} />
  <circle class="ring" cx={C} cy={C} r={R.signs} />
  <circle class="ring" cx={C} cy={C} r={R.inner} />

  {#if chart.houses}
    {#each chart.houses.cusps as cusp, i}
      {@const angular = i % 3 === 0}
      {@const [x1, y1] = xy(cusp, R.signs)}
      {@const [x2, y2] = xy(cusp, R.inner)}
      {@const next = chart.houses.cusps[(i + 1) % 12]}
      {@const mid = cusp + ((((next - cusp) % 360) + 360) % 360) / 2}
      {@const [nx, ny] = xy(mid, R.houseNum)}
      <line class="cusp" class:angular {x1} {y1} {x2} {y2} />
      <text class="house-num" x={nx} y={ny}>{i + 1}</text>
    {/each}
  {/if}

  {#each lines as l}
    <line class={`aspect ${l.tone}`} x1={l.x1} y1={l.y1} x2={l.x2} y2={l.y2} opacity={l.opacity} />
  {/each}

  {#each placed as { p, at }}
    {@const [tx1, ty1] = xy(p.lon, R.signs)}
    {@const [tx2, ty2] = xy(p.lon, R.tick)}
    {@const [gx, gy] = xy(at, R.glyph)}
    {@const [lx, ly] = xy(at, R.glyph + 15)}
    {@const [dx, dy] = xy(p.lon, R.inner)}
    <line class="tick" x1={tx1} y1={ty1} x2={tx2} y2={ty2} />
    <line class="leader" x1={tx2} y1={ty2} x2={lx} y2={ly} />
    <circle class="dot" cx={dx} cy={dy} r="2.5" />
    <text class="glyph body" x={gx} y={gy}>
      <title>{POINTS[p.key].name}: {position(p.lon)}{p.retrograde ? " (retrógrado)" : ""}</title>
      {POINTS[p.key].glyph}
    </text>
    {#if p.retrograde && p.key !== "north_node"}
      {@const [rx, ry] = xy(at, R.glyph - 21)}
      <text class="retro" x={rx} y={ry}>R</text>
    {/if}
  {/each}
</svg>

<style>
  svg {
    display: block;
    width: 100%;
    max-width: 600px;
    height: auto;
    margin: 0 auto;
  }

  text {
    text-anchor: middle;
    dominant-baseline: central;
    fill: var(--text);
  }

  .ring {
    fill: none;
    stroke: var(--line);
    stroke-width: 1.2;
  }

  .sign {
    fill: currentColor;
    fill-opacity: 0.1;
    stroke: var(--line);
    stroke-width: 1;
  }

  .sign-glyph {
    font-size: 23px;
    fill: currentColor;
  }

  .cusp {
    stroke: var(--line);
    stroke-width: 1;
  }

  .cusp.angular {
    stroke: var(--accent);
    stroke-width: 2;
  }

  .house-num {
    font-size: 12px;
    fill: var(--muted);
  }

  .aspect {
    stroke-width: 1.3;
  }

  .aspect.soft {
    stroke: var(--soft);
  }

  .aspect.hard {
    stroke: var(--hard);
  }

  .tick {
    stroke: var(--text);
    stroke-width: 1.5;
  }

  .leader {
    stroke: var(--muted);
    stroke-width: 0.7;
  }

  .dot {
    fill: var(--muted);
  }

  .body {
    font-size: 25px;
  }

  .retro {
    font-size: 10px;
    fill: var(--hard);
    font-weight: 700;
  }
</style>
