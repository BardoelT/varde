// Tabs of a page (CSD-035): one tab selected at a time, arrow keys to move between them, and an
// optional action when a tab opens. The selection lives in the DOM only, so it is kept while the
// app runs and never stored; `resetTabs` brings a page (or every page) back to its first tab.
import { errorText } from './ipc.js';
import { $, showToast } from './dom.js';

/** The way back to the opening tab of every wired page, by page key (`income`). */
const resetters = new Map();

/** Selects the opening tab of the page `key`, or of every page without a key. It never runs an `onOpen`. */
export function resetTabs(key) {
  if (key === undefined) resetters.forEach((reset) => reset());
  else resetters.get(key)?.();
}

/**
 * Wires the tabs `[{ tab, panel, onOpen }]`, where `tab` and `panel` are element
 * ids and `onOpen` (optional) runs each time the tab is opened, for example to
 * recompute a chart so it never shows stale figures. `key` names the page for
 * `resetTabs`; `initial` is the index of the tab it opens on (default the first).
 */
export function initTabs(tabs, { key, initial = 0 } = {}) {
  function select(index, { focus = false, open = true } = {}) {
    tabs.forEach(({ tab, panel }, i) => {
      const selected = i === index;
      $(tab).setAttribute('aria-selected', String(selected));
      $(tab).tabIndex = selected ? 0 : -1;
      $(panel).hidden = !selected;
    });
    if (focus) $(tabs[index].tab).focus();
    const { onOpen } = tabs[index];
    if (open && onOpen) Promise.resolve(onOpen()).catch((error) => showToast(errorText(error)));
  }

  tabs.forEach(({ tab }, index) => {
    $(tab).addEventListener('click', () => select(index));
    $(tab).addEventListener('keydown', (event) => {
      const step = { ArrowRight: 1, ArrowLeft: -1 }[event.key];
      if (step === undefined) return;
      event.preventDefault();
      select((index + step + tabs.length) % tabs.length, { focus: true });
    });
  });
  if (key !== undefined) resetters.set(key, () => select(initial, { open: false }));
}

/**
 * Wires every tab of the page `prefix` from the markup: the tablist that holds `<prefix>-tab-insights`,
 * each tab pointing at its panel with `aria-controls`. `onOpen` maps a panel id to its action, and
 * `initial` is the id of the tab the page opens on (default the first).
 */
export function initPageTabs(prefix, { onOpen = {}, initial } = {}) {
  const buttons = [...$(`${prefix}-tab-insights`).parentElement.querySelectorAll('[role="tab"]')];
  const tabs = buttons.map((button) => {
    const panel = button.getAttribute('aria-controls');
    return { tab: button.id, panel, onOpen: onOpen[panel] };
  });
  const index = tabs.findIndex(({ panel }) => panel === initial);
  initTabs(tabs, { key: prefix, initial: Math.max(index, 0) });
}
