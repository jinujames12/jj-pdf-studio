<script lang="ts">
  import { go } from '../stores/nav.svelte';
  import { tools } from '../tools';
  import Dropzone from './Dropzone.svelte';
</script>

<section class="home">
  <h1>What would you like to do?</h1>
  <p class="lead">Free, offline PDF tools. Pick a tool, or drop a file to get started.</p>

  <div class="grid">
    {#each tools as t (t.id)}
      <button class="card" onclick={() => go(t.id)}>
        <span class="icon"><t.icon size={20} strokeWidth={1.75} /></span>
        <span class="name">{t.name}</span>
        <span class="desc">{t.description}</span>
      </button>
    {/each}
  </div>

  <Dropzone />
</section>

<style>
  .home {
    max-width: 1040px;
    margin: 0 auto;
    padding: var(--space-8) var(--space-6);
  }
  h1 {
    margin: 0;
    font-size: var(--text-title);
    font-weight: 600;
  }
  .lead {
    margin: var(--space-1) 0 var(--space-6);
    color: var(--text-muted);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: var(--space-4);
    margin-bottom: var(--space-6);
  }
  .card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-1);
    padding: var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
    box-shadow: var(--shadow-card);
    text-align: left;
    cursor: pointer;
    transition:
      box-shadow var(--duration-base) var(--ease),
      transform var(--duration-base) var(--ease),
      border-color var(--duration-base) var(--ease);
  }
  .card:hover {
    border-color: var(--brand-300);
    box-shadow: var(--shadow-card-hover);
    transform: translateY(-1px);
  }
  .icon {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    margin-bottom: var(--space-2);
    border-radius: var(--radius-control);
    background: var(--selected-bg);
    color: var(--primary);
  }
  .name {
    font-size: var(--text-header);
    font-weight: 600;
  }
  .desc {
    color: var(--text-muted);
    font-size: var(--text-caption);
  }
</style>
