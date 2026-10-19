<script lang="ts">
  import type { Snippet } from 'svelte';
  import { onMount } from 'svelte';
  import X from 'lucide-svelte/icons/x';

  interface Props {
    title: string;
    onclose: () => void;
    wide?: boolean;
    children: Snippet;
  }

  let { title, onclose, wide = false, children }: Props = $props();

  let dialog: HTMLElement;
  const titleId = `modal-${Math.random().toString(36).slice(2, 8)}`;

  // Focus moves into the dialog, stays inside while it is open, and goes back afterwards.
  onMount(() => {
    const before = document.activeElement as HTMLElement | null;
    dialog.focus();
    return () => before?.focus?.();
  });

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.stopPropagation();
      onclose();
    } else if (e.key === 'Tab') {
      const items = [...dialog.querySelectorAll<HTMLElement>('button, a[href], [tabindex="0"]')];
      if (!items.length) return;
      const first = items[0];
      const last = items[items.length - 1];
      if (e.shiftKey && (document.activeElement === first || document.activeElement === dialog)) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onclose}>
  <div
    class="dialog"
    class:wide
    role="dialog"
    aria-modal="true"
    aria-labelledby={titleId}
    tabindex="-1"
    bind:this={dialog}
    {onkeydown}
    onclick={(e) => e.stopPropagation()}
  >
    <header>
      <h2 id={titleId}>{title}</h2>
      <button class="close" aria-label="Close" onclick={onclose}>
        <X size={16} strokeWidth={1.75} />
      </button>
    </header>
    <div class="content">{@render children()}</div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 70;
    display: grid;
    place-items: center;
    padding: var(--space-4);
    background: rgb(10 13 30 / 50%);
  }
  .dialog {
    display: flex;
    flex-direction: column;
    width: min(460px, 100%);
    max-height: 100%;
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
    box-shadow: var(--shadow-card-hover);
    outline: none;
  }
  .dialog.wide {
    width: min(820px, 100%);
    height: 100%;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--border);
  }
  h2 {
    margin: 0;
    font-size: var(--text-header);
    font-weight: 600;
  }
  .close {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: var(--radius-control);
    background: transparent;
    cursor: pointer;
  }
  .close:hover {
    background: var(--hover-bg);
  }
  .content {
    flex: 1;
    min-height: 0;
    padding: var(--space-4);
    overflow: auto;
  }
</style>
