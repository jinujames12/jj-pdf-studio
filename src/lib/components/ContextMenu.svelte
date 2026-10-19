<script lang="ts" module>
  export interface MenuItem {
    label: string;
    run?: () => void;
    shortcut?: string;
    disabled?: boolean;
    /** A thin line above this item. */
    separator?: boolean;
  }
</script>

<script lang="ts">
  import { onMount, tick } from 'svelte';

  interface Props {
    /** Viewport position of the pointer, or of the corner the menu hangs from. */
    x: number;
    y: number;
    items: MenuItem[];
    label?: string;
    onclose: () => void;
  }

  let { x, y, items, label = 'Page actions', onclose }: Props = $props();

  let menu: HTMLElement;
  let left = $state(0);
  let top = $state(0);
  let active = $state(-1);

  const enabled = (i: number) => !items[i].disabled;

  function step(delta: number) {
    if (!items.some((_, i) => enabled(i))) return;
    let next = active;
    do {
      next = (next + delta + items.length) % items.length;
    } while (!enabled(next));
    active = next;
    void tick().then(() => menu.querySelectorAll<HTMLElement>('[role=menuitem]')[next]?.focus());
  }

  function choose(i: number) {
    if (!enabled(i)) return;
    onclose();
    items[i].run?.();
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onclose();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      step(1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      step(-1);
    } else if (e.key === 'Enter' || e.key === ' ') {
      if (active >= 0) {
        e.preventDefault();
        choose(active);
      }
    } else if (e.key === 'Tab') {
      onclose();
    }
  }

  onMount(() => {
    // Keep the menu inside the window.
    const r = menu.getBoundingClientRect();
    left = Math.max(4, Math.min(x, window.innerWidth - r.width - 4));
    top = Math.max(4, Math.min(y, window.innerHeight - r.height - 4));
    const before = document.activeElement as HTMLElement | null;
    step(1);
    // Capture phase, so the click that closes the menu is not also used by the page below it.
    const outside = (e: Event) => {
      if (!menu.contains(e.target as Node)) onclose();
    };
    window.addEventListener('pointerdown', outside, true);
    window.addEventListener('blur', onclose);
    window.addEventListener('resize', onclose);
    window.addEventListener('wheel', onclose, { passive: true });
    return () => {
      window.removeEventListener('pointerdown', outside, true);
      window.removeEventListener('blur', onclose);
      window.removeEventListener('resize', onclose);
      window.removeEventListener('wheel', onclose);
      before?.focus?.();
    };
  });
</script>

<div
  class="menu"
  role="menu"
  aria-label={label}
  tabindex="-1"
  bind:this={menu}
  style:left="{left}px"
  style:top="{top}px"
  {onkeydown}
  oncontextmenu={(e) => e.preventDefault()}
>
  {#each items as item, i (i)}
    {#if item.separator}<div class="sep" role="separator"></div>{/if}
    <button
      class="entry"
      role="menuitem"
      tabindex="-1"
      disabled={item.disabled}
      onclick={() => choose(i)}
      onpointerenter={() => (active = i)}
    >
      <span>{item.label}</span>
      {#if item.shortcut}<kbd>{item.shortcut}</kbd>{/if}
    </button>
  {/each}
</div>

<style>
  .menu {
    position: fixed;
    z-index: 90;
    min-width: 220px;
    padding: var(--space-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
    box-shadow: var(--shadow-card);
  }
  .entry {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    width: 100%;
    height: 30px;
    padding: 0 var(--space-3);
    border: 0;
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .entry:hover:not(:disabled),
  .entry:focus-visible {
    background: var(--hover-bg);
    outline: 2px solid var(--focus-ring);
    outline-offset: -2px;
  }
  .entry:disabled {
    opacity: 0.45;
    cursor: default;
  }
  kbd {
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-caption);
  }
  .sep {
    height: 1px;
    margin: var(--space-1) 0;
    background: var(--border);
  }
</style>
