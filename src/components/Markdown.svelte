<script lang="ts">
  import DOMPurify from "dompurify";
  import { marked } from "marked";

  let { source }: { source: string } = $props();

  // O texto vem de um modelo: passa pelo sanitizador antes de virar HTML.
  const html = $derived(DOMPurify.sanitize(marked.parse(source, { async: false, gfm: true, breaks: false })));
</script>

<div class="md">{@html html}</div>

<style>
  .md {
    max-width: 68ch;
    line-height: 1.65;
    overflow-wrap: break-word;
  }

  .md :global(h1),
  .md :global(h2) {
    font-size: 1.25rem;
    margin: 1.6rem 0 0.6rem;
    color: var(--accent);
  }

  .md :global(h3),
  .md :global(h4) {
    font-size: 1.05rem;
    margin: 1.2rem 0 0.4rem;
  }

  .md :global(:first-child) {
    margin-top: 0;
  }

  .md :global(p),
  .md :global(ul),
  .md :global(ol) {
    margin: 0 0 0.8rem;
  }

  .md :global(ul),
  .md :global(ol) {
    padding-left: 1.3rem;
  }

  .md :global(li) {
    margin-bottom: 0.3rem;
  }

  .md :global(hr) {
    border: 0;
    border-top: 1px solid var(--line);
    margin: 1.2rem 0;
  }

  .md :global(table) {
    display: block;
    overflow-x: auto;
    margin-bottom: 0.8rem;
  }

  .md :global(blockquote) {
    margin: 0 0 0.8rem;
    padding-left: 0.9rem;
    border-left: 3px solid var(--line);
    color: var(--muted);
  }
</style>
