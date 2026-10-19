import { mount } from 'svelte';
import './styles/tokens.css';
import { initTheme } from './lib/stores/theme.svelte';
import App from './App.svelte';

// Set data-theme before the first paint so there is no light/dark flash.
await initTheme();

mount(App, { target: document.getElementById('app')! });
