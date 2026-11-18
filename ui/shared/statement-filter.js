// The filter of a statement table (CSD-028). It has two parts, each opened from an icon button
// in its own column header: the period range next to the period title and the entities next to
// the entity title. While a part is on, its icon is filled and the panel can reset that part. The page
// keeps the filter in memory and sends it to Rust with the list command; Rust decides which rows
// match and adds up any total. Nothing here is stored (CON-4).
import { $, h, iconButton } from './dom.js';
import { invoke } from './ipc.js';
import { t } from './i18n.js';
import { maskInput } from './date-mask.js';

const MONTH = /^\d{4}-(0[1-9]|1[0-2])$/;
const DAY = /^(\d{2})\/(\d{2})\/(\d{4})$/;

/** The ISO day of a typed `dd/mm/yyyy` that is a real calendar day, else `null`. Display-level only: Rust reads the ISO day. */
function isoDay(text) {
  const found = DAY.exec(text);
  if (!found) return null;
  const [, day, month, year] = found;
  const date = new Date(Date.UTC(Number(year), Number(month) - 1, Number(day)));
  const real = date.getUTCFullYear() === Number(year) && date.getUTCMonth() === Number(month) - 1 && date.getUTCDate() === Number(day);
  return real ? `${year}-${month}-${day}` : null;
}
/** How long typing in a filter field has to pause before the table is asked again. */
const TYPING_DELAY_MS = 200;
const PANEL_WIDTH = 280;
/** Space kept between the panel and the icon, and between the panel and the window edge. */
const GAP = 4;
const MARGIN = 8;
/** The least height a panel that has to scroll gets. */
const MIN_HEIGHT = 120;

/** Every filter on the page, so a new or opened project can clear them all (REQ-13). */
const allFilters = new Set();
/** The column whose panel is open; at most one panel is open at a time. */
let openColumn = null;

/** Clears every filter without reloading; the caller reloads the pages afterwards. */
export function resetAllFilters() {
  for (const filter of allFilters) filter.clear();
}

/**
 * Makes the filter of one table.
 * `periodHost` and `entityHost` are the ids of the empty elements in the column headers that
 * hold the buttons, `entityTitle` the English title of the entity column (a text with a Dutch
 * version) and `entities()` gives the `{ id, label }` choices.
 * `period` is `'month'` (the default) or, for the Transactions list only, `'day'`: the period panel then asks for
 * `dd/mm/yyyy` days and Rust gets `YYYY-MM-DD` (CSD-032 REQ-15).
 * `amountHost` is the id of the header element of an amount column; with it the filter also has an amount
 * range, typed in the number format of the app and read by Rust (for the Transactions list only, CSD-032 REQ-15).
 * `textHosts` (`{ counterparty, description }`, the ids of the header elements of those columns) adds a text
 * filter on each: exact, or with `*` as a wildcard, ignoring case; Rust reads the text (for the Transactions list only, CSD-032 REQ-15).
 * `choices` adds more columns with a choice of their own, like the entity column: `[{ host, key, title, choices }]`, where
 * `key` is the payload field for the chosen ids and `choices()` gives the `{ id, label }` choices (the Account column of
 * the Transactions list, CSD-034 REQ-6).
 * `onChange()` reloads the table. It is called at once for a click and, for typed text, after a short pause in
 * the typing (`TYPING_DELAY_MS`); a change that leaves what Rust is asked the same, such as half a date, calls
 * nothing (REQ-2).
 */
export function createStatementFilter({ periodHost, entityHost, amountHost = null, textHosts = null, choices = [], entityTitle, entities, onChange, period = 'month' }) {
  const days = period === 'day';
  /** The value Rust gets for a typed bound, or null while the text is not a valid month (or day). */
  const bound = (text) => (days ? isoDay(text) : MONTH.test(text) ? text : null);
  const amountText = (text) => (/\d/.test(text) ? text.trim() : null);

  /** One text field of a panel; typing asks the table again after a pause (see `changed`). */
  function typedField(label, attributes, value, onValue) {
    const input = h('input', { type: 'text', autocomplete: 'off', ...attributes, value });
    if (attributes['data-mask']) maskInput(input);
    input.addEventListener('input', () => {
      onValue(input.value);
      changed({ typing: true });
    });
    return h('label', { class: 'filter-field' }, h('span', {}, label), input);
  }

  const panelOf = (label, ...content) => h('div', { class: 'filter-panel', role: 'group', 'aria-label': label }, ...content);

  // The parts of the filter, one per column header. Each has the state of its own and says what it is
  // (`host`, the labels), whether it filters (`active`), what it adds to the payload (`fields`), how it is
  // cleared (`clear`) and how its panel is drawn (`panel`). A part an entity needs no longer can drop it (`prune`).

  function periodPart() {
    let from = '';
    let to = '';
    const mask = days
      ? { 'data-mask': 'date', inputmode: 'numeric', placeholder: 'dd/mm/yyyy', maxlength: '10' }
      : { 'data-mask': 'month', inputmode: 'numeric', placeholder: 'yyyy-mm', maxlength: '7' };
    return {
      host: periodHost,
      filterLabel: () => t('Filter by period'),
      activeLabel: () => t('Filter by period (active)'),
      active: () => bound(from) !== null || bound(to) !== null,
      fields: () => ({ from: bound(from), to: bound(to) }),
      clear: () => { from = ''; to = ''; },
      panel: () =>
        panelOf(
          t('Filter by period'),
          typedField(days ? t('From date') : t('From month'), mask, from, (value) => { from = value; }),
          typedField(days ? t('To date') : t('To month'), mask, to, (value) => { to = value; }),
        ),
    };
  }

  /**
   * A column whose rows each have one of a few choices (a category, an account): one checkbox per choice.
   * `key` is the field of the payload that holds the chosen ids, `title` the English column title and
   * `choices()` gives the `{ id, label }` of every choice.
   */
  function choicePart({ host, key, title, choices }) {
    const ids = new Set();
    const name = () => t(title).toLocaleLowerCase();
    return {
      host,
      filterLabel: () => t('Filter by {entity}', { entity: name() }),
      activeLabel: () => t('Filter by {entity} (active)', { entity: name() }),
      active: () => ids.size > 0,
      fields: () => ({ [key]: [...ids] }),
      clear: () => ids.clear(),
      /** Drops the choices that no longer exist, without a message (REQ-12). */
      prune() {
        const known = new Set(choices().map((choice) => choice.id));
        for (const id of [...ids]) if (!known.has(id)) ids.delete(id);
      },
      panel() {
        const checks = choices().map((choice) => {
          const box = h('input', { type: 'checkbox', value: choice.id });
          box.checked = ids.has(choice.id);
          box.addEventListener('change', () => {
            if (box.checked) ids.add(choice.id);
            else ids.delete(choice.id);
            changed();
          });
          return h('label', { class: 'filter-check' }, box, h('span', { class: 'wrap' }, choice.label));
        });
        return panelOf(t('Filter by {entity}', { entity: name() }), h('div', { class: 'filter-checks' }, checks));
      },
    };
  }

  /** The range of an amount column, typed in the number format of the app and read by Rust. */
  function amountPart() {
    let min = '';
    let max = '';
    const attributes = { inputmode: 'decimal' };
    return {
      host: amountHost,
      filterLabel: () => t('Filter by amount'),
      activeLabel: () => t('Filter by amount (active)'),
      active: () => amountText(min) !== null || amountText(max) !== null,
      fields: () => ({ min_amount: amountText(min), max_amount: amountText(max) }),
      clear: () => { min = ''; max = ''; },
      panel: () =>
        panelOf(
          t('Filter by amount'),
          typedField(t('From amount'), attributes, min, (value) => { min = value; }),
          typedField(t('To amount'), attributes, max, (value) => { max = value; }),
        ),
    };
  }

  /** The text filter of a text column: `key` is its field in the payload, `title` the English column title. */
  function textPart(key, host, title) {
    let text = '';
    const name = () => t(title).toLocaleLowerCase();
    const trimmed = () => text.trim();
    return {
      host,
      filterLabel: () => t('Filter by {entity}', { entity: name() }),
      activeLabel: () => t('Filter by {entity} (active)', { entity: name() }),
      active: () => trimmed() !== '',
      fields: () => ({ [key]: trimmed() === '' ? null : trimmed() }),
      clear: () => { text = ''; },
      panel: () =>
        panelOf(
          t('Filter by {entity}', { entity: name() }),
          typedField(t('{entity} (* is any text)', { entity: t(title) }), { maxlength: '100' }, text, (value) => { text = value; }),
        ),
    };
  }

  const parts = [
    periodPart(),
    choicePart({ host: entityHost, key: 'ids', title: entityTitle, choices: entities }),
    ...choices.map(choicePart),
    ...(amountHost === null ? [] : [amountPart()]),
    ...(textHosts === null ? [] : [textPart('counterparty', textHosts.counterparty, 'Counterparty'), textPart('description', textHosts.description, 'Description')]),
  ];

  /** What Rust gets, or null when no part of the filter is active. */
  function payload() {
    return parts.some((part) => part.active()) ? Object.assign({}, ...parts.map((part) => part.fields())) : null;
  }

  const prune = () => parts.forEach((part) => part.prune?.());

  /** One column header's buttons and panel. */
  function column({ host, filterLabel, activeLabel, active, clear: clearPart, panel: buildPanel }) {
    let panel = null;
    let resetButton = null;
    let button = null;
    // The buttons live in one element of their own, so redrawing them never touches the open
    // panel and the focus stays in the field the user is typing in.
    const controls = h('span', { class: 'filter-controls' });
    const root = () => $(host);

    const self = {
      closePanel({ restoreFocus = false } = {}) {
        if (panel === null) return;
        panel.remove();
        panel = null;
        resetButton = null;
        if (openColumn === self) openColumn = null;
        document.removeEventListener('click', outsideClick, true);
        window.removeEventListener('scroll', onScroll, true);
        window.removeEventListener('resize', onScroll);
        button?.setAttribute('aria-expanded', 'false');
        if (restoreFocus) button?.focus();
      },

      /**
       * Draws the icon, filled while this column is filtered (REQ-6), and the state of the reset
       * button. The button stays the same element, so it keeps the focus through a reload.
       */
      render() {
        const on = active();
        const label = on ? activeLabel() : filterLabel();
        const fresh = iconButton(on ? 'filter-active' : 'filter', label, () => (panel === null ? open() : self.closePanel()));
        if (!button) {
          button = fresh;
          button.classList.add('filter-btn');
          controls.replaceChildren(button);
        } else {
          button.setAttribute('aria-label', label);
          button.setAttribute('title', label);
          button.replaceChildren(...fresh.childNodes);
        }
        button.setAttribute('aria-expanded', String(panel !== null));
        if (!controls.isConnected) root().prepend(controls);
        if (resetButton) resetButton.disabled = !on;
      },
    };

    function outsideClick(event) {
      if (!root().contains(event.target)) self.closePanel();
    }

    /**
     * The panel follows its icon when the page or the table moves. It never closes on a scroll:
     * clicking the text of a checkbox focuses it, which can scroll the table (REQ-2).
     */
    function onScroll() {
      if (panel) place();
    }

    /**
     * The panel hangs under the header icon; the table scrolls sideways, so it is placed by the
     * icon. It always stays inside the window (REQ-2): below the icon when it fits, else above it
     * when there is more room there, and if neither fits it gets the room of the larger side and
     * scrolls inside.
     */
    function place() {
      const rect = root().getBoundingClientRect();
      panel.style.maxHeight = '';
      panel.style.left = `${Math.max(MARGIN, Math.min(rect.left, window.innerWidth - PANEL_WIDTH - MARGIN))}px`;
      panel.style.top = `${rect.bottom + GAP}px`;
      const height = panel.offsetHeight;
      const below = window.innerHeight - rect.bottom - GAP - MARGIN;
      const above = rect.top - GAP - MARGIN;
      if (height <= below) return;
      if (above > below) {
        panel.style.maxHeight = `${above}px`;
        panel.style.top = `${rect.top - GAP - Math.min(height, above)}px`;
      } else {
        panel.style.maxHeight = `${Math.max(below, MIN_HEIGHT)}px`;
      }
    }

    function open() {
      openColumn?.closePanel();
      prune();
      panel = buildPanel();
      resetButton = h('button', { type: 'button', class: 'btn filter-reset' }, t('Reset filter'));
      resetButton.addEventListener('click', () => {
        clearPart();
        changed();
        self.closePanel({ restoreFocus: true });
      });
      panel.append(resetButton);
      panel.addEventListener('keydown', (event) => {
        if (event.key === 'Escape') {
          event.stopPropagation();
          self.closePanel({ restoreFocus: true });
        }
      });
      root().append(panel);
      place();
      openColumn = self;
      document.addEventListener('click', outsideClick, true);
      window.addEventListener('scroll', onScroll, true);
      window.addEventListener('resize', onScroll);
      panel.querySelector('input')?.focus();
      self.render();
    }

    return self;
  }

  const columns = parts.map(column);

  /** The payload the table was last asked for, as text; the same payload asks nothing again. */
  let asked = JSON.stringify(null);
  let typingTimer = null;

  function changed({ typing = false } = {}) {
    for (const each of columns) each.render();
    clearTimeout(typingTimer);
    const ask = () => {
      const now = JSON.stringify(payload());
      if (now === asked) return;
      asked = now;
      onChange();
    };
    if (typing) typingTimer = setTimeout(ask, TYPING_DELAY_MS);
    else ask();
  }

  const filter = {
    payload,

    /** Clears every part without reloading. */
    clear() {
      clearTimeout(typingTimer);
      asked = JSON.stringify(null);
      for (const part of parts) part.clear();
      for (const each of columns) each.closePanel();
      if ($(periodHost)?.querySelector('.filter-controls')) for (const each of columns) each.render();
    },

    /**
     * Lists through the filter. `count(result)` is the number of rows in an answer. When the
     * filter leaves no rows, the list is asked again without it to tell "no data yet" from "no
     * matches" (REQ-8). `extra` holds the other arguments of the command, such as a row limit.
     * `narrowing` holds arguments that also hide rows, such as "manual only"; they count as a
     * filter and are left out of the second question.
     * Resolves to `{ result, hasData }`.
     */
    async load(command, count, extra = {}, narrowing = {}) {
      prune();
      const current = payload();
      asked = JSON.stringify(current);
      const args = (more) => (Object.keys(more).length > 0 ? more : undefined);
      const narrowed = Object.keys(narrowing).length > 0;
      const result = await invoke(command, args({ ...extra, ...narrowing, ...(current ? { filter: current } : {}) }));
      let hasData = count(result) > 0;
      if (!hasData && (current || narrowed)) hasData = count(await invoke(command, args(extra))) > 0;
      return { result, hasData };
    },

    /** Draws the buttons while the table holds data (REQ-1). Without data the table is hidden. */
    show(hasData) {
      for (const each of columns) {
        if (hasData) each.render();
        else each.closePanel();
      }
    },
  };
  allFilters.add(filter);
  return filter;
}

/**
 * Shows the rows of a filtered list, or the note of an empty table: "no data yet" when the table
 * holds nothing, "no matches" when the filter hides everything. In that case the header row
 * stays, so the filter icons stay reachable (REQ-8).
 * `ids` names the elements `{ empty, nomatch, table, body }`.
 */
export function renderFilteredTable({ empty, nomatch, table, body }, { rows, hasData }, buildRow) {
  $(empty).hidden = hasData;
  $(nomatch).hidden = !hasData || rows.length > 0;
  $(table).hidden = !hasData;
  $(body).replaceChildren(...rows.map(buildRow));
}
