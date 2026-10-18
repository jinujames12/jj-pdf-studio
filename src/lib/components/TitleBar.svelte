<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import Sun from 'lucide-svelte/icons/sun';
  import Moon from 'lucide-svelte/icons/moon';
  import Minus from 'lucide-svelte/icons/minus';
  import Square from 'lucide-svelte/icons/square';
  import Copy from 'lucide-svelte/icons/copy';
  import X from 'lucide-svelte/icons/x';
  import logoUrl from '../../../assets/brand/logo.svg';
  import { inTauri } from '../api';
  import { go } from '../stores/nav.svelte';
  import { theme, toggleTheme } from '../stores/theme.svelte';

  // The native frame is off (decorations: false), so we draw the window controls.
  // Outside Tauri (plain browser during UI work) there is no window to control.
  const win = inTauri() ? getCurrentWindow() : null;
  let maximized = $state(false);

  onMount(() => {
    if (!win) return;
    const sync = () => win.isMaximized().then((m) => (maximized = m));
    sync();
    const unlisten = win.onResized(sync);
    return () => {
      unlisten.then((fn) => fn());
    };
  });
</script>

<!-- data-tauri-drag-region makes the bar draggable and double-click maximises. -->
<header class="titlebar" data-tauri-drag-region>
  <button class="brand" onclick={() => go('home')} aria-label="Go to home">
    <img src={logoUrl} alt="" width="24" height="24" />
    <span>JJ PDF Studio</span>
  </button>

  <!-- Empty drag area between brand and controls -->
  <div class="spacer" data-tauri-drag-region></div>

  <button
    class="icon-btn"
    onclick={toggleTheme}
    aria-label={theme.current === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}
    title={theme.current === 'dark' ? 'Light theme' : 'Dark theme'}
  >
    {#if theme.current === 'dark'}<Sun size={18} strokeWidth={1.75} />{:else}<Moon
        size={18}
        strokeWidth={1.75}
      />{/if}
  </button>

  <div class="controls">
    <button class="win-btn" onclick={() => win?.minimize()} aria-label="Minimise">
      <Minus size={16} strokeWidth={1.75} />
    </button>
    <button
      class="win-btn"
      onclick={() => win?.toggleMaximize()}
      aria-label={maximized ? 'Restore' : 'Maximise'}
    >
      {#if maximized}<Copy size={14} strokeWidth={1.75} />{:else}<Square
          size={13}
          strokeWidth={1.75}
        />{/if}
    </button>
    <button class="win-btn close" onclick={() => win?.close()} aria-label="Close">
      <X size={16} strokeWidth={1.75} />
    </button>
  </div>
</header>

<style>
  .titlebar {
    display: flex;
    align-items: center;
    height: var(--titlebar-height);
    flex: none;
    padding-left: var(--space-3);
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-2);
    border: 0;
    border-radius: var(--radius-control);
    background: transparent;
    font-size: var(--text-header);
    font-weight: 600;
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease);
  }
  .brand:hover {
    background: var(--hover-bg);
  }
  .brand img {
    display: block;
    border-radius: 6px;
  }
  .spacer {
    flex: 1;
    align-self: stretch;
  }
  .icon-btn {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    margin-right: var(--space-2);
    border: 0;
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    transition:
      background var(--duration-fast) var(--ease),
      color var(--duration-fast) var(--ease);
  }
  .icon-btn:hover {
    background: var(--hover-bg);
    color: var(--text);
  }
  .controls {
    display: flex;
    align-self: stretch;
  }
  .win-btn {
    display: grid;
    place-items: center;
    width: 46px;
    border: 0;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    transition:
      background var(--duration-fast) var(--ease),
      color var(--duration-fast) var(--ease);
  }
  .win-btn:hover {
    background: var(--hover-bg);
    color: var(--text);
  }
  .win-btn.close:hover {
    background: var(--danger);
    color: #fff;
  }
  /* Square focus ring inside the full-height window buttons */
  .win-btn:focus-visible {
    outline-offset: -2px;
  }
</style>
