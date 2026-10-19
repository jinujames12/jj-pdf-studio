<script lang="ts">
  import House from 'lucide-svelte/icons/house';
  import Info from 'lucide-svelte/icons/info';
  import PanelLeftClose from 'lucide-svelte/icons/panel-left-close';
  import PanelLeftOpen from 'lucide-svelte/icons/panel-left-open';
  import { go, nav, type ViewId } from '../stores/nav.svelte';
  import { sidebarTools, toolGroups } from '../tools';

  const STORAGE_KEY = 'jj-pdf-studio.sidebar-collapsed';

  function readCollapsed(): boolean {
    try {
      return localStorage.getItem(STORAGE_KEY) === '1';
    } catch {
      return false;
    }
  }

  let collapsed = $state(readCollapsed());

  function toggle() {
    collapsed = !collapsed;
    try {
      localStorage.setItem(STORAGE_KEY, collapsed ? '1' : '0');
    } catch {
      // Ignore: the state just won't persist.
    }
  }

  // When collapsed the labels are hidden, so each item needs a tooltip.
  const tip = (label: string) => (collapsed ? label : undefined);
</script>

{#snippet item(id: ViewId, label: string)}
  <button
    class="item"
    class:active={nav.view === id}
    aria-current={nav.view === id ? 'page' : undefined}
    title={tip(label)}
    onclick={() => go(id)}
  >
    {#if id === 'home'}<House size={18} strokeWidth={1.75} />
    {:else if id === 'about'}<Info size={18} strokeWidth={1.75} />
    {:else}
      {@const Icon = sidebarTools.find((t) => t.view === id)?.icon}
      {#if Icon}<Icon size={18} strokeWidth={1.75} />{/if}
    {/if}
    <span class="label">{label}</span>
  </button>
{/snippet}

<nav class="sidebar" class:collapsed aria-label="Tools">
  {@render item('home', 'Home')}

  {#each toolGroups as group (group)}
    <div class="group" role="group" aria-labelledby="group-{group}">
      <div class="heading" id="group-{group}">{group}</div>
      {#each sidebarTools.filter((t) => t.group === group) as t (t.id)}
        {@render item(t.view, t.name)}
      {/each}
    </div>
  {/each}

  <div class="grow"></div>

  {@render item('about', 'About')}
  <button
    class="item"
    onclick={toggle}
    aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
    title={tip('Expand sidebar')}
  >
    {#if collapsed}<PanelLeftOpen size={18} strokeWidth={1.75} />{:else}<PanelLeftClose
        size={18}
        strokeWidth={1.75}
      />{/if}
    <span class="label">Collapse</span>
  </button>
</nav>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: none;
    width: var(--sidebar-width);
    padding: var(--space-3) var(--space-2);
    background: var(--surface-2);
    border-right: 1px solid var(--border);
    /* 10 entries and five headings do not fit a 640 px window; scroll instead of clipping. */
    overflow-x: hidden;
    overflow-y: auto;
    transition: width var(--duration-base) var(--ease);
  }
  .sidebar.collapsed {
    width: var(--sidebar-width-collapsed);
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .heading {
    height: 28px;
    padding: var(--space-3) var(--space-3) 0;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    transition: opacity var(--duration-fast) var(--ease);
  }
  .collapsed .heading {
    height: 12px;
    padding: 0;
    font-size: 0;
    border-top: 1px solid var(--border);
    margin-top: var(--space-2);
  }
  .grow {
    flex: 1;
  }
  .item {
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    height: 36px;
    padding: 0 var(--space-3);
    border: 0;
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--text);
    font-weight: 500;
    text-align: left;
    white-space: nowrap;
    cursor: pointer;
    transition:
      background var(--duration-fast) var(--ease),
      color var(--duration-fast) var(--ease);
  }
  .item :global(svg) {
    flex: none;
  }
  .item:hover {
    background: var(--hover-bg);
  }
  .item.active {
    background: var(--brand-600);
    color: #fff;
  }
  :global([data-theme='dark']) .item.active {
    background: var(--selected-bg);
    color: var(--primary);
  }
  .label {
    overflow: hidden;
    transition: opacity var(--duration-fast) var(--ease);
  }
  .collapsed .label {
    opacity: 0;
  }
</style>
