import { beforeEach, describe, expect, it } from 'vitest';
import { sidebarTools, toolGroups, tools } from '../tools';
import { go, nav, resolveRoute } from './nav.svelte';

beforeEach(() => {
  nav.view = 'home';
  nav.panel = null;
});

describe('old routes redirect to the Page Editor', () => {
  it.each([
    ['/pages', null],
    ['/split', 'split'],
    ['/extract', 'extract'],
    ['/rotate', 'rotate'],
    ['/page-numbers', 'numbers'],
    ['pages', null],
    ['split', 'split'],
    ['pagenumbers', 'numbers']
  ])('%s', (route, panel) => {
    expect(resolveRoute(route)).toEqual({ view: 'page-editor', panel });
    go(route);
    expect(nav.view).toBe('page-editor');
    expect(nav.panel).toBe(panel);
  });

  it('knows the new route', () => {
    expect(resolveRoute('/page-editor')).toEqual({ view: 'page-editor', panel: null });
  });

  it('leaves other screens alone and ignores unknown routes', () => {
    go('merge');
    expect(nav.view).toBe('merge');
    go('/nowhere');
    expect(nav.view).toBe('merge');
  });
});

describe('opening panels', () => {
  it('opens the panel a home card asks for', () => {
    go('page-editor', 'split');
    expect(nav.panel).toBe('split');
  });

  it('keeps the open panel when the sidebar item is clicked again', () => {
    go('split');
    go('page-editor');
    expect(nav.panel).toBe('split');
  });

  it('closes the panel when leaving the Page Editor', () => {
    go('extract');
    go('merge');
    go('page-editor');
    expect(nav.panel).toBeNull();
  });

  it('switches panels when another card is used inside the Page Editor', () => {
    go('split');
    go('rotate');
    expect(nav.panel).toBe('rotate');
  });
});

describe('sidebar and home screen', () => {
  it('has 10 sidebar entries in the groups of the spec', () => {
    expect(toolGroups).toEqual(['Organise', 'Convert', 'Optimise', 'Security', 'Edit']);
    const byGroup = (g: string) => sidebarTools.filter((t) => t.group === g).map((t) => t.name);
    expect(byGroup('Organise')).toEqual(['Page Editor', 'Merge']);
    expect(byGroup('Convert')).toEqual(['Images to PDF', 'PDF to Images']);
    expect(byGroup('Optimise')).toEqual(['Resize', 'Compress']);
    expect(byGroup('Security')).toEqual(['Unlock', 'Protect']);
    expect(byGroup('Edit')).toEqual(['Watermark', 'Properties']);
    expect(sidebarTools).toHaveLength(10);
  });

  it('has no sidebar entry for Pages, Rotate, Split, Extract or Page numbers', () => {
    const names = sidebarTools.map((t) => t.name);
    for (const gone of ['Pages', 'Rotate', 'Split', 'Extract', 'Page numbers']) {
      expect(names).not.toContain(gone);
    }
  });

  it('keeps a home card for every task, opening the matching panel', () => {
    const card = (name: string) => tools.find((t) => t.name === name);
    expect(card('Split')).toMatchObject({ view: 'page-editor', panel: 'split' });
    expect(card('Extract')).toMatchObject({ view: 'page-editor', panel: 'extract' });
    expect(card('Rotate')).toMatchObject({ view: 'page-editor', panel: 'rotate' });
    expect(card('Page numbers')).toMatchObject({ view: 'page-editor', panel: 'numbers' });
    expect(card('Page Editor')).toMatchObject({ view: 'page-editor' });
    expect(tools).toHaveLength(14);
  });

  it('every card lands on a real screen', () => {
    for (const t of tools) {
      go('home');
      go(t.view, t.panel);
      expect(nav.view).toBe(t.view);
      expect(nav.panel ?? null).toBe(t.panel ?? null);
    }
  });
});
