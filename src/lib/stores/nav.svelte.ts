// Minimal view router: the app has a handful of screens and no URLs.

/** The options panels of the Page Editor (SPEC section 5.15). */
export type PanelId = 'split' | 'extract' | 'rotate' | 'numbers';

export type ViewId =
  | 'home'
  | 'merge'
  | 'page-editor'
  | 'resize'
  | 'compress'
  | 'images'
  | 'unlock'
  | 'protect'
  | 'toimages'
  | 'watermark'
  | 'properties'
  | 'about';

interface Route {
  view: ViewId;
  panel: PanelId | null;
}

/**
 * Routes of the screens that moved into the Page Editor. They keep working and
 * land on the Page Editor with the matching panel open, so old links, saved
 * habits and the home screen cards do not break.
 */
const LEGACY_ROUTES: Record<string, PanelId | null> = {
  pages: null,
  split: 'split',
  extract: 'extract',
  rotate: 'rotate',
  pagenumbers: 'numbers',
  'page-numbers': 'numbers'
};

const VIEWS: readonly string[] = [
  'home',
  'merge',
  'page-editor',
  'resize',
  'compress',
  'images',
  'unlock',
  'protect',
  'toimages',
  'watermark',
  'properties',
  'about'
];

/** Resolves a route such as `/split`, `pages` or `page-editor` to a screen and panel. */
export function resolveRoute(route: string): Route | null {
  const name = route.replace(/^\/+/, '');
  if (name in LEGACY_ROUTES) return { view: 'page-editor', panel: LEGACY_ROUTES[name] };
  if (VIEWS.includes(name)) return { view: name as ViewId, panel: null };
  return null;
}

export const nav = $state<{ view: ViewId; panel: PanelId | null }>({
  view: 'home',
  panel: null
});

/**
 * Opens a screen. Accepts the old routes too. `panel` opens that Page Editor
 * panel; going to the Page Editor without one keeps whatever panel is open.
 */
export function go(route: string, panel?: PanelId | null) {
  const target = resolveRoute(route);
  if (!target) return;
  const requested = target.panel ?? panel ?? null;
  if (target.view !== nav.view) nav.panel = requested;
  else if (requested) nav.panel = requested;
  nav.view = target.view;
}
