<script lang="ts">
  import { onMount } from 'svelte';
  import CircleCheck from 'lucide-svelte/icons/circle-check';
  import Info from 'lucide-svelte/icons/info';
  import { openSaved, revealSaved, type CompressResult } from '../api';
  import { fmtSize, plural } from '../format';
  import { report } from '../saved';

  let { result, onclose }: { result: CompressResult; onclose: () => void } = $props();

  let dialog: HTMLElement;

  /** Share of the original that was saved, 0 to 100. */
  const percent = $derived(
    result.written && result.before > 0 ? Math.round((1 - result.after / result.before) * 100) : 0
  );
  const little = $derived(result.written && 1 - result.after / result.before < 0.05);

  // Take focus so Escape works, and give it back when the dialog closes.
  onMount(() => {
    const before = document.activeElement as HTMLElement | null;
    dialog.focus();
    return () => before?.focus?.();
  });

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.stopPropagation();
      onclose();
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onclose}>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="compress-result-title"
    tabindex="-1"
    bind:this={dialog}
    {onkeydown}
    onclick={(e) => e.stopPropagation()}
  >
    {#if result.written}
      <span class="badge ok"><CircleCheck size={28} strokeWidth={1.75} /></span>
      <h2 id="compress-result-title">{percent}% smaller</h2>
      <p class="sizes">
        <span>{fmtSize(result.before)}</span>
        <span aria-hidden="true">→</span>
        <strong>{fmtSize(result.after)}</strong>
      </p>
      {#if result.images_recompressed > 0}
        <p class="note">{plural(result.images_recompressed, 'image')} recompressed.</p>
      {/if}
      {#if little}
        <p class="note">
          This PDF is mostly text or vector graphics, so there was little to compress.
        </p>
      {/if}
    {:else}
      <span class="badge"><Info size={28} strokeWidth={1.75} /></span>
      <h2 id="compress-result-title">This PDF is already well optimised</h2>
      <p class="note">
        Compressing it would not make it smaller, so no file was saved. Keep using the original.
      </p>
    {/if}
    <div class="buttons">
      {#if result.written && result.path}
        {@const path = result.path}
        <button class="btn primary" onclick={() => void openSaved(path).catch(report)}>
          Open file
        </button>
        <button class="btn" onclick={() => void revealSaved(path).catch(report)}>
          Show in folder
        </button>
      {/if}
      <button class="btn" onclick={onclose}>Close</button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    background: rgba(10, 12, 24, 0.6);
  }
  .dialog {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    width: min(92vw, 440px);
    padding: var(--space-6);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
    box-shadow: var(--shadow-dialog);
    outline: none;
    text-align: center;
  }
  .badge {
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    border-radius: var(--radius-card);
    background: var(--selected-bg);
    color: var(--primary);
  }
  .badge.ok {
    color: var(--success);
  }
  h2 {
    margin: var(--space-2) 0 0;
    font-size: var(--text-header);
    font-weight: 600;
  }
  .sizes {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    margin: 0;
    font-size: var(--text-title);
    font-variant-numeric: tabular-nums;
  }
  .sizes span {
    color: var(--text-muted);
  }
  .note {
    margin: 0;
    color: var(--text-muted);
  }
  .buttons {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: var(--space-2);
    margin-top: var(--space-4);
  }
</style>
