// Acceptance tests (batch D) for the screens of CSD-002 (project menu and close), CSD-008 (theme)
// and CSD-022 (language) that no test named yet. Rust decides everything here; these tests check
// what the screen shows and which commands it calls. jsdom cannot evaluate `prefers-color-scheme`,
// so the system theme is proven through what the app writes on <html> and through the CSS rules
// that react to the Mac (see the notes at AC-4.1 to AC-4.3).
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { openApp, commandError } from './harness.js';
import { exampleReads } from './fixtures.js';

const uiDir = fileURLToPath(new URL('..', import.meta.url));
const baseCss = readFileSync(`${uiDir}styles/base.css`, 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');

const unsaved = () => {
  const reads = exampleReads();
  reads.project_status = { has_path: true, unsaved_changes: true };
  return reads;
};
const savedStatus = { has_path: true, unsaved_changes: false };

const readsWithTheme = (theme, language = 'en') => {
  const reads = exampleReads();
  reads.get_preferences = { theme, language };
  return reads;
};
const dataTheme = (app) => app.document.documentElement.dataset.theme;

// ---------------------------------------------------------------- CSD-002

test('test_csd002_ac_9_2_choosing_save_runs_the_save_and_clears_the_unsaved_indicator', async () => {
  // The native dialog is shown by Rust inside `save_project` (CSD-002 AC-1.3, proven by cargo test);
  // here the menu item must call it, and the indicator must follow the status Rust returns.
  const reads = exampleReads();
  reads.project_status = { has_path: false, unsaved_changes: true };
  const app = await openApp({ reads, handlers: { save_project: { status: savedStatus, done: true } } });
  assert.equal(app.textOf('project-state-text'), 'Unsaved changes');
  assert.equal(app.$('project-state').classList.contains('is-unsaved'), true);

  await app.emit('menu-action', 'project-save');
  assert.equal(app.callsOf('save_project').length, 1, 'File > Save calls the save command');
  assert.equal(app.$('project-state').classList.contains('is-unsaved'), false, 'the indicator is cleared');
  assert.equal(app.textOf('project-state-text'), 'All changes saved');
  app.close();
});

test('test_csd002_ac_9_4_menu_actions_ask_before_replacing_unsaved_changes', async () => {
  const handlers = {
    new_project: { has_path: false, unsaved_changes: false },
    open_project: { done: true, status: savedStatus },
  };
  for (const [action, command, message] of [
    ['project-new', 'new_project', 'You have unsaved changes. If you start a new project, they will be lost.'],
    ['project-open', 'open_project', 'You have unsaved changes. If you open another file, they will be lost.'],
  ]) {
    const app = await openApp({ reads: unsaved(), handlers });
    await app.emit('menu-action', action);
    assert.equal(app.$('confirm-dialog').open, true, `${action} asks a confirmation`);
    assert.equal(app.textOf('confirm-message'), message);
    assert.equal(app.callsOf(command).length, 0, 'nothing is replaced before the answer');

    await app.click(app.$('confirm-cancel'));
    assert.equal(app.callsOf(command).length, 0, 'cancel keeps the project');

    await app.emit('menu-action', action);
    await app.click(app.$('confirm-ok'));
    assert.equal(app.callsOf(command).length, 1, 'confirming replaces the project');
    app.close();
  }
});

// ---------------------------------------------------------------- CSD-008

test('test_csd008_ac_1_1_the_appearance_panel_and_the_theme_choices', async () => {
  const app = await openApp();
  await app.click(app.item('settings'));
  const panels = [...app.document.querySelectorAll('#settings .panel')];
  const titles = panels.map((panel) => app.text(panel.querySelector('.panel-title')));
  assert.ok(titles.includes('General') && titles.includes('Appearance'), `panels: ${titles}`);
  assert.ok(titles.indexOf('Appearance') > titles.indexOf('General'), 'Appearance is below General');

  const appearance = panels[titles.indexOf('Appearance')];
  const select = app.$('setting-theme');
  assert.ok(appearance.contains(select));
  assert.equal(appearance.querySelector('label[for="setting-theme"]').textContent, 'Theme');
  assert.deepEqual([...select.options].map((option) => [option.value, option.text]), [
    ['system', 'System'],
    ['light', 'Light'],
    ['dark', 'Dark'],
  ]);
  app.close();
});

for (const [theme, label] of [['system', 'System'], ['light', 'Light'], ['dark', 'Dark']]) {
  test(`test_csd008_ac_1_2_the_choice_shows_the_saved_theme_${theme}`, async () => {
    const app = await openApp({ reads: readsWithTheme(theme) });
    await app.click(app.item('settings'));
    const select = app.$('setting-theme');
    assert.equal(select.value, theme);
    assert.equal(select.selectedOptions[0].text, label);
    app.close();
  });
}

test('test_csd008_ac_3_1_choosing_dark_shows_the_dark_theme_at_once_without_a_confirmation', async () => {
  const app = await openApp({ reads: readsWithTheme('system') });
  assert.equal(dataTheme(app), undefined, 'system: the Mac decides');
  await app.click(app.item('settings'));
  await app.choose(app.$('setting-theme'), 'dark');
  assert.deepEqual(app.callsOf('set_theme').map((call) => call.args), [{ theme: 'dark' }]);
  assert.equal(dataTheme(app), 'dark', 'the dark theme is shown');
  assert.equal(app.$('confirm-dialog').open, false, 'no confirmation is asked');
  app.close();
});

test('test_csd008_ac_3_2_the_choice_is_not_an_unsaved_project_change', async () => {
  const reads = exampleReads();
  reads.project_status = savedStatus;
  const app = await openApp({ reads: { ...reads, get_preferences: { theme: 'system', language: 'en' } } });
  await app.click(app.item('settings'));
  await app.choose(app.$('setting-theme'), 'light');
  assert.equal(dataTheme(app), 'light');
  assert.equal(app.$('project-state').classList.contains('is-unsaved'), false, 'no unsaved indicator');
  assert.equal(app.textOf('project-state-text'), 'All changes saved');
  app.close();
});

test('test_csd008_ac_3_3_choosing_the_current_theme_changes_nothing', async () => {
  const app = await openApp({ reads: readsWithTheme('dark') });
  assert.equal(dataTheme(app), 'dark');
  await app.click(app.item('settings'));
  await app.choose(app.$('setting-theme'), 'dark');
  assert.equal(app.callsOf('set_theme').length, 0, 'nothing is saved again');
  assert.equal(dataTheme(app), 'dark');
  assert.equal(app.$('setting-theme').value, 'dark');
  app.close();
});

/** The declaration block that follows `selector` in base.css, as one string. */
function block(selector, from = 0) {
  const start = baseCss.indexOf(selector, from);
  assert.ok(start >= 0, `${selector} exists in base.css`);
  return baseCss.slice(baseCss.indexOf('{', start) + 1, baseCss.indexOf('}', start));
}

test('test_csd008_ac_4_1_system_leaves_the_theme_to_the_mac_through_a_media_query', async () => {
  const app = await openApp({ reads: readsWithTheme('system') });
  assert.equal(dataTheme(app), undefined, 'no theme is forced on the page');
  // The dark colours apply when the Mac is dark (media query) unless the user chose Light...
  const media = baseCss.indexOf('@media (prefers-color-scheme: dark)');
  assert.ok(media > 0, 'the dark colours sit under the macOS appearance');
  assert.match(block(':root:not([data-theme="light"])', media), /--bg:/, 'with the dark colour tokens');
  // ...and the light colours are the plain defaults, so a light Mac shows the light theme.
  assert.match(block(':root {'), /--bg:/);
  app.close();
});

test('test_csd008_ac_4_2_system_follows_a_change_of_the_mac_without_a_restart', async () => {
  // Nothing in the app reads or caches the macOS appearance: the browser re-evaluates the media
  // query by itself, so the page changes while it is open. The real switch is on the manual list.
  const sources = [];
  const walk = (dir) => {
    for (const name of readdirSync(dir)) {
      const path = `${dir}${name}`;
      if (name === 'tests' || name === 'node_modules') continue;
      if (statSync(path).isDirectory()) walk(`${path}/`);
      else if (name.endsWith('.js')) sources.push([path, readFileSync(path, 'utf8')]);
    }
  };
  walk(uiDir);
  assert.ok(sources.length > 10);
  for (const [path, text] of sources) assert.ok(!text.includes('matchMedia'), `${path} does not cache the Mac appearance`);

  const app = await openApp({ reads: readsWithTheme('dark') });
  await app.click(app.item('settings'));
  await app.choose(app.$('setting-theme'), 'system');
  assert.equal(dataTheme(app), undefined, 'back to System, the page follows the Mac again');
  app.close();
});

for (const theme of ['light', 'dark']) {
  test(`test_csd008_ac_4_3_${theme}_ignores_the_mac`, async () => {
    const app = await openApp({ reads: readsWithTheme(theme) });
    assert.equal(dataTheme(app), theme, `the page is forced to ${theme}`);
    // The Mac's dark colours are fenced off from an explicit Light, and an explicit Dark has its own copy.
    assert.ok(baseCss.includes(':root:not([data-theme="light"])'));
    assert.ok(baseCss.includes(':root[data-theme="dark"]'));
    app.close();
  });
}

test('test_csd008_ac_6_1_the_theme_is_applied_before_the_first_page_is_shown', async () => {
  const events = [];
  const changes = [];
  const app = await openApp({
    reads: readsWithTheme('dark'),
    handlers: {
      get_preferences: () => {
        const root = document.documentElement;
        events.push(`before: theme=${root.dataset.theme ?? 'none'} ready=${root.classList.contains('theme-ready')}`);
        const observer = new document.defaultView.MutationObserver((records) => {
          for (const record of records) changes.push(record.attributeName);
        });
        observer.observe(root, { attributes: true, attributeOldValue: true });
        return { theme: 'dark', language: 'en' };
      },
    },
  });
  assert.deepEqual(events, ['before: theme=none ready=false'], 'the page is still hidden when the theme is asked for');
  assert.ok(changes.indexOf('data-theme') >= 0, 'the theme is written');
  const root = app.document.documentElement;
  assert.equal(root.dataset.theme, 'dark');
  assert.equal(root.classList.contains('theme-ready'), true, 'the page is revealed afterwards');
  const firstClass = changes.indexOf('class');
  assert.ok(changes.indexOf('data-theme') < changes.lastIndexOf('class') && firstClass >= 0, `theme first, then reveal: ${changes}`);
  assert.match(baseCss, /html:not\(\.theme-ready\)\s*\{\s*visibility:\s*hidden/, 'until then the page is hidden');
  app.close();
});

// ---------------------------------------------------------------- CSD-022

test('test_csd022_ac_2_2_the_macos_language_is_not_followed', async () => {
  // No preference file: Rust answers English (`test_csd022_ac_2_2_...` in cargo test). The screen must
  // not look at the language of the Mac or of the browser either.
  const app = await openApp();
  assert.equal(app.document.documentElement.lang, 'en');
  assert.equal(app.text(app.item('debts')), 'Debts');
  assert.equal(app.$('setting-language').value, 'en');
  app.close();

  const offenders = [];
  const walk = (dir) => {
    for (const name of readdirSync(dir)) {
      const path = `${dir}${name}`;
      if (name === 'tests' || name === 'node_modules') continue;
      if (statSync(path).isDirectory()) walk(`${path}/`);
      else if (name.endsWith('.js') || name.endsWith('.html')) {
        if (/navigator\.language|navigator\.languages|resolvedOptions\(\)\.locale/.test(readFileSync(path, 'utf8'))) offenders.push(path);
      }
    }
  };
  walk(uiDir);
  assert.deepEqual(offenders, [], 'nothing reads the system language');
});

const FILE_ERRORS = [
  ['PROJECT_SAVE_FAILED', 'The project could not be saved.', 'Het project kon niet worden opgeslagen.'],
  ['PROJECT_OPEN_FAILED', 'The file could not be opened.', 'Het bestand kon niet worden geopend.'],
  ['PROJECT_INVALID_FILE', 'This is not a valid project file.', 'Dit is geen geldig projectbestand.'],
  ['PROJECT_FILE_NEWER', 'This file was made by a newer version of the app.', 'Dit bestand is gemaakt met een nieuwere versie van de app.'],
];

for (const language of ['en', 'nl']) {
  test(`test_csd022_ac_7_3_a_file_error_toast_has_no_path_in_${language}`, async () => {
    for (const [code, english, dutch] of FILE_ERRORS) {
      const handlers = {
        open_project: () => {
          throw commandError(code, english);
        },
      };
      const app = await openApp({ reads: readsWithTheme('system', language), handlers });
      await app.emit('menu-action', 'project-open');
      assert.equal(app.$('toast').hidden, false, `${code} shows a toast`);
      const toast = app.textOf('toast');
      assert.equal(toast, language === 'nl' ? dutch : english, code);
      assert.ok(!/[/\\]|\.varde|\.wealth/.test(toast), `${code}: no path in "${toast}"`);
      app.close();
    }
  });
}
