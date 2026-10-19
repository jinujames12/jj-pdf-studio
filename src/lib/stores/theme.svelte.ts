// Theme: follows the OS unless the user picks one with the toggle.
// The manual choice is stored in settings.json by the Rust side.
import { errorMessage, getSettings, inTauri, setThemeSetting } from '../api';
import { toast } from './toast.svelte';

type Theme = 'light' | 'dark';

const osQuery = window.matchMedia('(prefers-color-scheme: dark)');

const state = $state<{ manual: Theme | null; os: Theme }>({
  manual: null,
  os: osQuery.matches ? 'dark' : 'light'
});

/** The theme currently in effect. */
export const theme = {
  get current(): Theme {
    return state.manual ?? state.os;
  }
};

function apply() {
  document.documentElement.dataset.theme = theme.current;
}

/** Flip between light and dark and remember the choice. */
export function toggleTheme() {
  state.manual = theme.current === 'dark' ? 'light' : 'dark';
  apply();
  if (inTauri()) {
    setThemeSetting(state.manual).catch((e) => toast('error', errorMessage(e)));
  }
}

/**
 * Applies the saved theme. Awaited before the app mounts so a saved dark theme
 * on a light Windows does not flash; a missing settings file just follows the OS.
 */
export async function initTheme() {
  if (inTauri()) {
    try {
      state.manual = (await getSettings()).theme;
    } catch {
      // Unreadable settings: follow the OS.
    }
  }
  apply();
  // Keep following Windows when the user has not chosen manually.
  osQuery.addEventListener('change', (e) => {
    state.os = e.matches ? 'dark' : 'light';
    if (state.manual === null) apply();
  });
}
