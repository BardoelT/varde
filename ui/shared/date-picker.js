// The date picker of every date, month and year field (CSD-036): the days of a month, the months of a year
// and the years of a decade, opened from the field or its calendar button. Display only: a pick writes the
// same text the user could type (`dd/mm/yyyy`, `yyyy-mm` or `yyyy`) and fires the same `input` event, and
// whether that text is acceptable is Rust's decision. A date is built from year, month and day, never from
// a time stamp, so no time zone moves a day (CSD-036 CON-3).
import { h } from './dom.js';
import { svg } from './chart.js';
import { t } from './i18n.js';
import { monthName, shortMonthName, weekdayNames } from './month-names.js';

const FIRST_YEAR = 1;
const LAST_YEAR = 9999;
const GAP = 4;
const MARGIN = 8;

const pad = (value, size = 2) => String(value).padStart(size, '0');

/** A date from its parts. `Date` maps the years 0 to 99 to 1900 to 1999, so the year is set afterwards. */
function makeDate(year, month, day) {
  const date = new Date(2000, 0, 1);
  date.setFullYear(year, month, day);
  return date;
}

const inRange = (year) => year >= FIRST_YEAR && year <= LAST_YEAR;

/** The kind of a field: `date`, `month` or `year`, by its mask (a month field limited to 4 characters is a year). */
function kindOf(input) {
  if (input.dataset.mask === 'date') return 'date';
  return input.maxLength === 4 ? 'year' : 'month';
}

/** What the text of a field says, as `{ year, month, day }` (month 0 to 11), or `null` while it is not a real date, month or year. */
function parse(kind, text) {
  const value = text.trim();
  if (kind === 'date') {
    const match = /^(\d{2})\/(\d{2})\/(\d{4})$/.exec(value);
    if (!match) return null;
    const [day, month, year] = [Number(match[1]), Number(match[2]) - 1, Number(match[3])];
    if (!inRange(year)) return null;
    const date = makeDate(year, month, day);
    return date.getFullYear() === year && date.getMonth() === month && date.getDate() === day ? { year, month, day } : null;
  }
  if (kind === 'month') {
    const match = /^(\d{4})-(\d{2})$/.exec(value);
    const year = match ? Number(match[1]) : 0;
    const month = match ? Number(match[2]) - 1 : -1;
    return inRange(year) && month >= 0 && month <= 11 ? { year, month, day: 1 } : null;
  }
  const year = /^\d{4}$/.test(value) ? Number(value) : 0;
  return inRange(year) ? { year, month: 0, day: 1 } : null;
}

/** The text a pick writes. */
function write(kind, { year, month, day }) {
  if (kind === 'date') return `${pad(day)}/${pad(month + 1)}/${pad(year, 4)}`;
  if (kind === 'month') return `${pad(year, 4)}-${pad(month + 1)}`;
  return pad(year, 4);
}

const decadeOf = (year) => Math.floor(year / 10) * 10;

/** The calendar button: drawn, named by the field and kept out of the tab order (the keyboard opens with Down). */
function calendarButton() {
  return h(
    'button',
    { type: 'button', class: 'date-open', tabindex: '-1', 'aria-hidden': 'true' },
    svg('svg', { viewBox: '0 0 16 16', width: '14', height: '14', 'aria-hidden': 'true' }, svg('rect', { x: '2', y: '3', width: '12', height: '11', rx: '2', fill: 'none', stroke: 'currentColor', 'stroke-width': '1.4' }), svg('path', { d: 'M2 6.5h12M5 1.8v2.4M11 1.8v2.4', fill: 'none', stroke: 'currentColor', 'stroke-width': '1.4', 'stroke-linecap': 'round' })),
  );
}

/** The picker that is open, if any; only one is open at a time. */
let current = null;

function closeCurrent() {
  current?.close();
}

/** The one picker of a field: the state of its view and the elements it draws. */
function createPicker(input, options) {
  // The kind of a field can change after the picker is made: a statement period is a month, or a year for a yearly statement.
  let kind = options.kind ?? kindOf(input);
  const popup = h('div', { class: 'date-picker', role: 'dialog', 'aria-modal': 'false' });
  const view = { level: kind === 'date' ? 'days' : kind === 'month' ? 'months' : 'years', year: 0, month: 0, focus: null };
  const self = { popup, input, close, open, isOpen: () => popup.isConnected };

  const today = () => {
    const now = new Date();
    return { year: now.getFullYear(), month: now.getMonth(), day: now.getDate() };
  };
  const selected = () => parse(kind, options.current ? options.current() : input.value);

  // A field that offers only some periods (CSD-036 REQ-12): the months as `yyyy-mm`, the years as numbers.
  const offered = () => options.offers?.() ?? null;
  const offeredYears = () => {
    const offer = offered();
    if (!offer) return [];
    const years = kind === 'year' ? offer.years : offer.months.map((month) => Number(month.slice(0, 4)));
    return [...new Set(years.map(Number))].sort((a, b) => a - b);
  };
  const monthOffered = (year, month) => !offered() || offered().months.includes(`${pad(year, 4)}-${pad(month + 1)}`);
  const yearOffered = (year) => !offered() || offeredYears().includes(year);
  const firstOffered = () => offeredYears()[0] ?? FIRST_YEAR;
  const lastOffered = () => offeredYears().at(-1) ?? LAST_YEAR;
  const container = () => input.closest('dialog, .filter-panel') ?? document.body;

  /** Moves the view to the date in the field when it is valid, else to today. */
  function syncToField() {
    const found = selected() ?? latestOffered() ?? today();
    view.year = found.year;
    view.month = found.month;
  }

  /** The latest period a limited field offers, for an empty field to open on. */
  function latestOffered() {
    const offer = offered();
    if (!offer) return null;
    const latest = kind === 'year' ? offer.years.map(String).sort().at(-1) : [...offer.months].sort().at(-1);
    return latest ? parse(kind, latest) : null;
  }

  function place() {
    const box = input.getBoundingClientRect();
    const width = popup.offsetWidth || 264;
    const height = popup.offsetHeight || 0;
    const below = box.bottom + GAP;
    const fitsBelow = below + height <= window.innerHeight - MARGIN;
    const above = box.top - GAP - height;
    const top = fitsBelow || above < MARGIN ? below : above;
    popup.classList.toggle('above', !(fitsBelow || above < MARGIN));
    popup.style.top = `${Math.max(MARGIN, top)}px`;
    popup.style.left = `${Math.min(Math.max(MARGIN, box.left), Math.max(MARGIN, window.innerWidth - width - MARGIN))}px`;
  }

  /** Writes a pick into the field, as typing would, and closes. */
  function pick(parts) {
    if (options.onPick) options.onPick(write(kind, parts));
    else {
      input.value = write(kind, parts);
      input.dispatchEvent(new Event('input', { bubbles: true }));
    }
    close();
    input.focus();
  }

  /** An arrow button of a header. */
  const arrow = (label, text, disabled, onClick) => {
    const button = h('button', { type: 'button', class: 'dp-arrow', 'aria-label': label, disabled }, text);
    button.addEventListener('click', onClick);
    return button;
  };

  function header(title, titleLabel, onTitle, previous, next) {
    const heading = onTitle
      ? h('button', { type: 'button', class: 'dp-title', 'aria-label': titleLabel }, title)
      : h('span', { class: 'dp-title', 'aria-live': 'polite' }, title);
    if (onTitle) heading.addEventListener('click', onTitle);
    heading.id = `${popup.id}-title`;
    return h('div', { class: 'dp-header' }, previous, heading, next);
  }

  function renderDays() {
    const picked = selected();
    const now = today();
    const first = makeDate(view.year, view.month, 1);
    const offset = (first.getDay() + 6) % 7;
    const cells = [];
    for (let index = 0; index < 42; index += 1) {
      const date = makeDate(view.year, view.month, 1 - offset + index);
      const parts = { year: date.getFullYear(), month: date.getMonth(), day: date.getDate() };
      const same = (other) => other && other.year === parts.year && other.month === parts.month && other.day === parts.day;
      const outside = parts.month !== view.month;
      const label = `${parts.day} ${monthName(parts.month)} ${parts.year}`;
      const cell = h(
        'button',
        {
          type: 'button',
          role: 'gridcell',
          class: `dp-cell${outside ? ' outside' : ''}${same(now) ? ' today' : ''}${same(picked) ? ' selected' : ''}`,
          'data-date': `${pad(parts.year, 4)}-${pad(parts.month + 1)}-${pad(parts.day)}`,
          'aria-label': label,
          'aria-selected': String(Boolean(same(picked))),
          'aria-current': same(now) ? 'date' : null,
          disabled: !inRange(parts.year),
          tabindex: '-1',
        },
        String(parts.day),
      );
      cell.addEventListener('click', () => pick(parts));
      cells.push(cell);
    }
    const rows = [];
    for (let row = 0; row < 6; row += 1) rows.push(h('div', { role: 'row', class: 'dp-row' }, cells.slice(row * 7, row * 7 + 7)));
    const atStart = view.year === FIRST_YEAR && view.month === 0;
    const atEnd = view.year === LAST_YEAR && view.month === 11;
    const step = (delta) => () => {
      const moved = makeDate(view.year, view.month + delta, 1);
      view.year = moved.getFullYear();
      view.month = moved.getMonth();
      render();
    };
    return [
      header(`${monthName(view.month)} ${view.year}`, t('Choose a month'), () => { view.level = 'months'; render(); }, arrow(t('Previous month'), '‹', atStart, step(-1)), arrow(t('Next month'), '›', atEnd, step(1))),
      h('div', { class: 'dp-weekdays', 'aria-hidden': 'true' }, weekdayNames().map((name) => h('span', {}, name))),
      h('div', { role: 'grid', class: 'dp-grid dp-days', 'aria-labelledby': `${popup.id}-title` }, rows),
    ];
  }

  /** A grid of 12 months or 10 years: `entries` are `{ label, value, current, selected, disabled }`. */
  function renderChoices(entries, columns, onPick) {
    const buttons = entries.map((entry) => {
      const button = h(
        'button',
        { type: 'button', role: 'gridcell', class: `dp-cell dp-choice${entry.current ? ' today' : ''}${entry.selected ? ' selected' : ''}`, 'aria-selected': String(entry.selected), 'aria-current': entry.current ? 'date' : null, 'aria-label': entry.disabled ? t('{name} (not available)', { name: entry.name ?? entry.label }) : null, disabled: entry.disabled, tabindex: '-1', 'data-choice': String(entry.value) },
        entry.label,
      );
      button.addEventListener('click', () => onPick(entry.value));
      return button;
    });
    const rows = [];
    for (let row = 0; row < buttons.length / columns; row += 1) rows.push(h('div', { role: 'row', class: 'dp-row' }, buttons.slice(row * columns, row * columns + columns)));
    return h('div', { role: 'grid', class: `dp-grid dp-choices cols-${columns}`, 'aria-labelledby': `${popup.id}-title`, 'data-columns': String(columns) }, rows);
  }

  function renderMonths() {
    const picked = selected();
    const now = today();
    const entries = Array.from({ length: 12 }, (_, month) => ({
      label: shortMonthName(month),
      name: `${monthName(month)} ${view.year}`,
      value: month,
      current: now.year === view.year && now.month === month,
      selected: Boolean(picked) && picked.year === view.year && picked.month === month,
      disabled: !monthOffered(view.year, month),
    }));
    const step = (delta) => () => { view.year += delta; render(); };
    return [
      header(String(view.year), t('Choose a year'), () => { view.level = 'years'; render(); }, arrow(t('Previous year'), '‹', view.year <= Math.max(FIRST_YEAR, firstOffered()), step(-1)), arrow(t('Next year'), '›', view.year >= Math.min(LAST_YEAR, lastOffered()), step(1))),
      renderChoices(entries, 4, (month) => {
        if (kind === 'month') {
          pick({ year: view.year, month, day: 1 });
          return;
        }
        view.month = month;
        view.level = 'days';
        render();
      }),
    ];
  }

  function renderYears() {
    const picked = selected();
    const start = decadeOf(view.year);
    const entries = Array.from({ length: 10 }, (_, index) => ({
      label: String(start + index),
      value: start + index,
      current: today().year === start + index,
      selected: Boolean(picked) && picked.year === start + index,
      disabled: !inRange(start + index) || !yearOffered(start + index),
    }));
    const step = (delta) => () => { view.year += delta; render(); };
    return [
      header(`${start} – ${start + 9}`, '', null, arrow(t('Previous decade'), '‹', start <= 0 || (Boolean(offered()) && decadeOf(firstOffered()) >= start), step(-10)), arrow(t('Next decade'), '›', start + 10 > LAST_YEAR || (Boolean(offered()) && decadeOf(lastOffered()) <= start), step(10))),
      renderChoices(entries, 5, (year) => {
        view.year = year;
        if (kind === 'year') {
          pick({ year, month: 0, day: 1 });
          return;
        }
        view.level = 'months';
        render();
      }),
    ];
  }

  /** Draws the level of the view; focus stays where it was unless a cell asks for it. */
  function render() {
    popup.setAttribute('aria-labelledby', `${popup.id}-title`);
    popup.replaceChildren(...(view.level === 'days' ? renderDays() : view.level === 'months' ? renderMonths() : renderYears()));
    if (popup.isConnected) place();
    if (view.focus) {
      const target = view.focus;
      view.focus = null;
      const cell = view.level === 'days' ? popup.querySelector(`[data-date="${target}"]`) : popup.querySelector(`[data-choice="${target}"]`);
      cell?.focus();
    }
  }

  /** The day, month or year the focus starts on: the one of the field, else today. */
  function startCell() {
    const found = selected() ?? today();
    if (view.level === 'days') return `${pad(found.year, 4)}-${pad(found.month + 1)}-${pad(found.day)}`;
    return view.level === 'months' ? String(found.month) : String(found.year);
  }

  function open({ focusGrid = false } = {}) {
    if (current && current !== self) closeCurrent();
    kind = options.kind ?? kindOf(input);
    syncToField();
    view.level = kind === 'date' ? 'days' : kind === 'month' ? 'months' : 'years';
    if (!popup.id) popup.id = `date-picker-${++counter}`;
    if (!popup.isConnected) container().append(popup);
    current = self;
    view.focus = focusGrid ? startCell() : null;
    render();
    input.setAttribute('aria-expanded', 'true');
  }

  function close() {
    if (!popup.isConnected) return;
    popup.remove();
    input.setAttribute('aria-expanded', 'false');
    if (current === self) current = null;
  }

  /** Moves the focus in the grid by `delta` cells, changing the month, year or decade when it leaves the grid. */
  function move(delta, unit) {
    const active = popup.querySelector('.dp-cell:focus');
    if (!active) return;
    if (view.level === 'days') {
      const [year, month, day] = active.dataset.date.split('-').map(Number);
      const moved = unit === 'month' ? makeDate(year, month - 1 + delta, day) : makeDate(year, month - 1, day + delta);
      if (unit === 'month') {
        // A shorter month keeps the last day: 31 March plus one month is 30 April, not 1 May.
        const wanted = makeDate(year, month - 1 + delta, 1);
        const last = makeDate(wanted.getFullYear(), wanted.getMonth() + 1, 0).getDate();
        moved.setFullYear(wanted.getFullYear(), wanted.getMonth(), Math.min(day, last));
      }
      if (!inRange(moved.getFullYear())) return;
      view.year = moved.getFullYear();
      view.month = moved.getMonth();
      view.focus = `${pad(moved.getFullYear(), 4)}-${pad(moved.getMonth() + 1)}-${pad(moved.getDate())}`;
      render();
      return;
    }
    const cells = [...popup.querySelectorAll('.dp-choice')];
    const next = cells[cells.indexOf(active) + delta];
    next?.focus();
  }

  function onPopupKey(event) {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      close();
      input.focus();
      return;
    }
    if (!event.target.classList?.contains('dp-cell')) return;
    const columns = Number(popup.querySelector('.dp-choices')?.dataset.columns ?? 7);
    const moves = view.level === 'days'
      ? { ArrowLeft: [-1, 'day'], ArrowRight: [1, 'day'], ArrowUp: [-7, 'day'], ArrowDown: [7, 'day'], PageUp: [-1, 'month'], PageDown: [1, 'month'] }
      : { ArrowLeft: [-1], ArrowRight: [1], ArrowUp: [-columns], ArrowDown: [columns] };
    if (event.key in moves) {
      event.preventDefault();
      move(...moves[event.key]);
    } else if (view.level === 'days' && (event.key === 'Home' || event.key === 'End')) {
      event.preventDefault();
      const [year, month, day] = event.target.dataset.date.split('-').map(Number);
      const weekday = (makeDate(year, month - 1, day).getDay() + 6) % 7;
      move(event.key === 'Home' ? -weekday : 6 - weekday, 'day');
    } else if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      event.target.click();
    }
  }

  // Clicking inside the picker must not take the focus from the field, so typing goes on.
  popup.addEventListener('mousedown', (event) => event.preventDefault());
  popup.addEventListener('keydown', onPopupKey);
  popup.addEventListener('focusout', (event) => {
    if (event.relatedTarget && !popup.contains(event.relatedTarget) && event.relatedTarget !== input) close();
  });

  /** The field's own keys: Down or Alt+Down opens and moves into the grid, Tab and Escape close. */
  input.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && self.isOpen()) {
      event.preventDefault();
      event.stopPropagation();
      close();
      input.focus();
    } else if (event.key === 'Tab') {
      close();
    } else if (event.key === 'ArrowDown') {
      event.preventDefault();
      if (!self.isOpen()) open({ focusGrid: true });
      else {
        view.focus = startCell();
        render();
      }
    }
  });
  input.addEventListener('click', () => {
    if (!self.isOpen()) open();
  });
  // Typing moves the picker to the text once it is a real date, month or year, and to today while it is not.
  input.addEventListener('input', () => {
    if (!self.isOpen()) return;
    kind = options.kind ?? kindOf(input);
    syncToField();
    view.level = kind === 'date' ? 'days' : kind === 'month' ? 'months' : 'years';
    render();
  });

  return self;
}

let counter = 0;
let documentListening = false;

/** One set of listeners for the whole document: an outside click, a scroll or a resize closes the open picker. */
function listenOnDocument() {
  if (documentListening) return;
  documentListening = true;
  document.addEventListener('mousedown', (event) => {
    if (current && !current.popup.contains(event.target) && event.target !== current.input && !event.target.closest?.('.date-open')) closeCurrent();
  }, true);
  document.addEventListener('scroll', (event) => {
    if (current && !current.popup.contains(event.target)) closeCurrent();
  }, true);
  window.addEventListener('resize', closeCurrent);
}

/** Puts the calendar button on a field once it is in the page. */
function wrap(input, picker) {
  const wrapper = h('span', { class: 'date-field' });
  // Moving the field in the page drops its focus; a panel may have focused it already.
  const focused = document.activeElement === input;
  input.replaceWith(wrapper);
  const button = calendarButton();
  wrapper.append(input, button);
  if (focused) input.focus();
  button.addEventListener('click', () => {
    if (picker.isOpen()) picker.close();
    else {
      input.focus();
      picker.open();
    }
  });
  const dialog = input.closest('dialog');
  if (dialog && !dialog.dataset.datePicker) {
    dialog.dataset.datePicker = 'on';
    dialog.addEventListener('cancel', (event) => {
      if (current && dialog.contains(current.popup)) {
        event.preventDefault();
        closeCurrent();
      }
    });
    dialog.addEventListener('close', closeCurrent);
  }
}

/**
 * Gives a masked field its picker and calendar button (CSD-036 REQ-1). A field made in script is not in the page
 * yet, so its button is added once the script that built it has finished.
 */
export function attachDatePicker(input, options = {}) {
  if (input.dataset.picker) return;
  input.dataset.picker = 'on';
  input.setAttribute('aria-haspopup', 'dialog');
  input.setAttribute('aria-expanded', 'false');
  listenOnDocument();
  // A field that offers only some periods shows its period and cannot be typed in (CSD-036 REQ-12).
  if (options.offers) input.readOnly = true;
  const picker = createPicker(input, options);
  const add = () => {
    if (input.parentNode && !input.parentNode.classList?.contains('date-field')) wrap(input, picker);
  };
  if (input.parentNode) add();
  else queueMicrotask(add);
}
