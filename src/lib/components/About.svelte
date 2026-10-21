<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import logoUrl from '../../../assets/brand/logo.svg';
  import companyLogoUrl from '../../../assets/brand/james-and-james-vertical.svg';
  import { inTauri, openLink, pingCore } from '../api';
  import { dialogs } from '../stores/dialogs.svelte';
  import { report } from '../saved';

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
  <p class="tagline">PDF tool developed by James &amp; James LLP</p>

  <dl>
    <dt>Version</dt>
    <dd>{appVersion}</dd>
    <dt>Core library</dt>
    <dd>pdf-core {coreVersion}</dd>
    <dt>Made by</dt>
    <dd>James &amp; James LLP</dd>
    <dt>Build date</dt>
    <dd>{__BUILD_DATE__}</dd>
    <dt>License</dt>
    <dd>Apache-2.0</dd>
  </dl>

  <div
    class="company"
    role="img"
    aria-label="James &amp; James LLP"
    style:--logo="url({companyLogoUrl})"
  ></div>

  <p class="privacy">Fully offline. No telemetry. Your files never leave this computer.</p>

  <div class="buttons">
    <button class="btn" onclick={() => void openLink('repository').catch(report)}>
      Source code on GitHub
    </button>
    <button class="btn" onclick={() => void openLink('issues').catch(report)}>
      Report a problem
    </button>
    <button class="btn" onclick={() => (dialogs.open = 'shortcuts')}>Keyboard shortcuts</button>
    <button class="btn" onclick={() => (dialogs.open = 'notices')}>Third-party licenses</button>
  </div>
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
  /* A mask lets the single-colour logo follow the theme colour. */
  .company {
    width: 180px;
    aspect-ratio: 640 / 264;
    margin-top: var(--space-5, 20px);
    background: var(--primary);
    mask: var(--logo) center / contain no-repeat;
  }
  .privacy {
    margin: var(--space-4) 0 var(--space-3);
    color: var(--text-muted);
  }
  .buttons {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: var(--space-2);
  }
  dt {
    color: var(--text-muted);
  }
  dd {
    margin: 0;
    font-weight: 500;
  }
</style>
