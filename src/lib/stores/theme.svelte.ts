// Theme: follows the OS unless the user picks one with the toggle.
// The manual choice lives in localStorage for now (moves to settings.json in M1).

type Theme = 'light' | 'dark';

const STORAGE_KEY = 'jj-pdf-studio.theme';
const osQuery = window.matchMedia('(prefers-color-scheme: dark)');

function readStored(): Theme | null {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    return v === 'light' || v === 'dark' ? v : null;
  } catch {
    return null; // storage can be unavailable; fall back to the OS
  }
}

const state = $state<{ manual: Theme | null; os: Theme }>({
  manual: readStored(),
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
  try {
    localStorage.setItem(STORAGE_KEY, state.manual);
  } catch {
    // Ignore: the choice just won't persist.
  }
  apply();
}

export function initTheme() {
  apply();
  // Keep following Windows when the user has not chosen manually.
  osQuery.addEventListener('change', (e) => {
    state.os = e.matches ? 'dark' : 'light';
    if (state.manual === null) apply();
  });
}
