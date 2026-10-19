// Minimal view router: the app has a handful of screens and no URLs.

export type ViewId =
  | 'home'
  | 'merge'
  | 'pages'
  | 'split'
  | 'extract'
  | 'rotate'
  | 'resize'
  | 'compress'
  | 'images'
  | 'unlock'
  | 'protect'
  | 'toimages'
  | 'pagenumbers'
  | 'watermark'
  | 'properties'
  | 'about';

export const nav = $state<{ view: ViewId }>({ view: 'home' });

export function go(view: ViewId) {
  nav.view = view;
}
