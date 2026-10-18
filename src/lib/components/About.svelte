<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import logoUrl from '../../../assets/brand/logo.svg';
  import { inTauri, pingCore } from '../api';

  let appVersion = $state('…');
  let coreVersion = $state('…');

  onMount(async () => {
    if (!inTauri()) {
      appVersion = coreVersion = 'n/a (not running in Tauri)';
      return;
    }
    try {
      [appVersion, coreVersion] = await Promise.all([getVersion(), pingCore()]);
    } catch {
      appVersion = coreVersion = 'unavailable';
    }
  });
</script>

<section class="about">
  <img src={logoUrl} alt="JJ PDF Studio logo" width="96" height="96" />
  <h1>JJ PDF Studio</h1>
  <p class="tagline">Free, offline PDF tools for Windows.</p>

  <dl>
    <dt>Version</dt>
    <dd>{appVersion}</dd>
    <dt>Core library</dt>
    <dd>pdf-core {coreVersion}</dd>
    <dt>Made by</dt>
    <dd>JJ Global</dd>
    <dt>License</dt>
    <dd>Apache-2.0</dd>
  </dl>

  <!-- TODO(M6): links to the repository and issue tracker, third-party licenses button. -->
</section>

<style>
  .about {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: var(--space-8) var(--space-6);
    text-align: center;
  }
  img {
    border-radius: 22px;
    box-shadow: var(--shadow-card-hover);
  }
  h1 {
    margin: var(--space-4) 0 0;
    font-size: var(--text-title);
    font-weight: 600;
  }
  .tagline {
    margin: var(--space-1) 0 var(--space-6);
    color: var(--text-muted);
  }
  dl {
    display: grid;
    grid-template-columns: auto auto;
    gap: var(--space-2) var(--space-6);
    margin: 0;
    padding: var(--space-4) var(--space-6);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
    box-shadow: var(--shadow-card);
    text-align: left;
    user-select: text;
  }
  dt {
    color: var(--text-muted);
  }
  dd {
    margin: 0;
    font-weight: 500;
  }
</style>
