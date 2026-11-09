// Houses: things the user owns with a monthly or yearly valuation, each a value and a note (CSD-015),
// bought once and possibly sold (CSD-019 REQ-1, REQ-2).
import { invoke, errorText } from '../shared/ipc.js';
import { $, h, confirmAction, showError, setHint, submitOnce, showToast } from '../shared/dom.js';
import { amountForInput, formatAmount, formatDate, noteSummary } from '../shared/format.js';
import { askSettlements } from '../shared/settle.js';
import { t } from '../shared/i18n.js';
import { createStatementFilter, renderFilteredTable } from '../shared/statement-filter.js';
import { openDialog, saveDialog, deleteWithConfirm } from '../shared/crud.js';
import { createRecordForm } from '../shared/record-form.js';
import { accountRow, latestAmount, numCell, recordActions, renderTable, rowHeader, showTotal, toggleEmpty } from '../shared/list-rows.js';

let houses = [];
let onChanged = async () => {};

/** An amount that Rust may leave out (a change of none): a dash then. */
function optionalAmount(value) {
  return value === null ? '—' : formatAmount(value);
}

// ---- Rendering ----

/** Only the valuation period and Held or Sold; the purchase and sale are in the edit form (CSD-019 REQ-8). */
function holdingText(house) {
  return `${t(house.period)} · ${house.held ? t('Held') : t('Sold')}`;
}

function renderHouses(overview) {
  houses = overview.houses;
  const hasHouses = houses.length > 0;
  toggleEmpty('houses-empty', 'houses-content', hasHouses);
  $('house-list').replaceChildren(
    ...houses.map((house) =>
      accountRow({
        name: house.name,
        meta: holdingText(house),
        amount: latestAmount(house.latest_value, house.latest_period),
        edit: { label: t('Edit {name}', { name: house.name }), onClick: () => openHouseDialog(house) },
        remove: { label: t('Delete {name}', { name: house.name }), onClick: () => deleteHouse(house) },
      }),
    ),
  );
  showTotal('house-total', 'house-total-value', overview.total, hasHouses);
}

/** Yearly values over all houses, as returned by Rust (CSD-015 REQ-19). */
function renderTotals(totals) {
  renderTable({ empty: 'house-totals-empty', table: 'house-totals-table', body: 'house-totals-rows' }, totals, (total) =>
    h(
      'tr',
      {},
      rowHeader(String(total.year)),
      numCell(formatAmount(total.closing_value)),
      numCell(optionalAmount(total.change)),
    ),
  );
}

const valuationsFilter = createStatementFilter({
  periodHost: 'house-valuation-period-filter',
  entityHost: 'house-valuation-entity-filter',
  entityTitle: 'House',
  entities: () => houses.map((house) => ({ id: house.id, label: house.name })),
  onChange: () => reloadValuations().catch((error) => showToast(errorText(error))),
});

/** Loads the rows through the filter and shows them (CSD-028). */
async function reloadValuations() {
  showValuations(await valuationsFilter.load('list_house_valuations', (rows) => rows.length));
}

function showValuations({ result, hasData }) {
  valuationsFilter.show(hasData);
  renderValuations(result, hasData);
}

function renderValuations(valuations, hasData) {
  renderFilteredTable({ empty: 'house-valuations-empty', nomatch: 'house-valuations-nomatch', table: 'house-valuations-table', body: 'house-valuations-rows' }, { rows: valuations, hasData }, (s) =>
    h(
      'tr',
      {},
      rowHeader(s.period),
      h('td', { class: 'wrap' }, s.house_name),
      numCell(formatAmount(s.value)),
      numCell(optionalAmount(s.change)),
      h('td', { class: 'note-cell' }, noteSummary(s.note)),
      recordActions({
        edit: { label: t('Edit {name} {period}', { name: s.house_name, period: s.period }), onClick: () => valuationForm.open(s) },
        remove: { label: t('Delete {name} {period}', { name: s.house_name, period: s.period }), onClick: () => deleteValuation(s) },
      }),
    ),
  );
}

/** Reloads everything from Rust. */
export async function reloadHousesOverview() {
  const overview = await invoke('list_houses');
  renderHouses(overview);
  if (overview.houses.length === 0) {
    valuationsFilter.show(false);
    return;
  }
  const [totals, loaded] = await Promise.all([invoke('yearly_house_values'), valuationsFilter.load('list_house_valuations', (rows) => rows.length)]);
  renderTotals(totals);
  showValuations(loaded);
}

async function changed() {
  await reloadHousesOverview();
  await onChanged();
}

// ---- House dialog ----

let editingHouse = null;

/** The debts linked to the house as a read-only table: they are linked in the debt form (CSD-026 REQ-12). */
function fillLinkedDebts(house) {
  const linked = house ? house.linked_debts : [];
  $('house-debts').hidden = linked.length === 0;
  $('house-debt-rows').replaceChildren(
    ...linked.map((debt) =>
      h(
        'tr',
        {},
        h('th', { scope: 'row', class: 'wrap' }, debt.debt_name),
        h('td', {}, t(debt.category)),
        h('td', { class: 'num' }, debt.latest_balance === null ? '—' : formatAmount(debt.latest_balance)),
        h('td', {}, debt.settlement ? `${t(debt.settlement.outcome)} ${formatAmount(debt.settlement.amount)}` : '—'),
      ),
    ),
  );
}

function openHouseDialog(house) {
  editingHouse = house;
  openDialog({
    dialog: 'house-dialog',
    form: 'house-form',
    error: 'house-error',
    title: 'house-dialog-title',
    titleText: house ? t('Edit house') : t('Add house'),
    focus: 'name',
    fill(form) {
      // The valuation period is locked once the house has valuations (CSD-015 REQ-5).
      const locked = Boolean(house && house.valuation_count > 0);
      form.elements.period.disabled = locked;
      setHint('house-period-hint', locked);
      if (house) {
        form.elements.name.value = house.name;
        form.elements.period.value = house.period;
        form.elements.purchase_date.value = formatDate(house.purchase_date);
        form.elements.purchase_amount.value = amountForInput(house.purchase_amount);
        form.elements.sell_date.value = house.sell_date === null ? '' : formatDate(house.sell_date);
        form.elements.sell_amount.value = amountForInput(house.sell_amount);
      }
      fillLinkedDebts(house);
    },
  });
}

function submitHouse() {
  const form = $('house-form');
  const input = {
    name: form.elements.name.value,
    purchase_date: form.elements.purchase_date.value,
    purchase_amount: form.elements.purchase_amount.value,
    sell_date: form.elements.sell_date.value,
    sell_amount: form.elements.sell_amount.value,
  };
  if (!form.elements.period.disabled) {
    input.period = form.elements.period.value;
  }
  const links = { settlements: [], confirm_undo: false };
  return saveDialog({
    dialog: 'house-dialog',
    error: 'house-error',
    changed,
    async save() {
      const sellDate = input.sell_date.trim();
      const sold = editingHouse !== null && !editingHouse.held;
      if (sellDate && !sold) {
        // A new sale: linked debts with an open balance need Repaid or Cancelled (CSD-026 REQ-1).
        const open = editingHouse ? await invoke('list_open_debts', { houseId: editingHouse.id, sellDate }) : [];
        if (open.length > 0) {
          const choices = await askSettlements(open);
          if (choices === null) return false;
          links.settlements = choices;
        }
      } else if (!sellDate && sold && editingHouse.linked_debts.some((debt) => debt.settlement)) {
        // Undoing the sale also undoes the settled debts (CSD-026 REQ-4).
        const confirmed = await confirmAction({
          title: t('Undo the sale?'),
          message: t('The debts settled on this sale go back to their earlier balances.'),
          confirmLabel: t('Undo sale'),
          danger: true,
        });
        if (!confirmed) return false;
        links.confirm_undo = true;
      }
      return editingHouse
        ? invoke('update_house', { houseId: editingHouse.id, input, links })
        : invoke('create_house', { input, links });
    },
  });
}

function deleteHouse(house) {
  return deleteWithConfirm({
    title: t('Delete {name}?', { name: house.name }),
    message: t('This house and all its valuations will be deleted.'),
    confirmLabel: t('Delete house'),
    command: 'delete_house',
    args: { houseId: house.id },
    changed,
  });
}

// ---- Valuation dialog ----

const valuationForm = createRecordForm({
  form: 'house-valuation-form',
  dialog: 'house-valuation-dialog',
  title: 'house-valuation-dialog-title',
  error: 'house-valuation-error',
  periodLabel: 'house-valuation-period-label',
  copy: 'house-valuation-copy',
  parentField: 'house_id',
  parents: () => houses,
  valueFields: ['value', 'note'],
  focusField: 'value',
  titleText: (valuation) => (valuation ? t('Edit valuation {period}', { period: valuation.period }) : t('Add valuation')),
});

function submitValuation() {
  const form = $('house-valuation-form');
  const args = {
    houseId: form.elements.house_id.value,
    period: form.elements.period.value,
    value: form.elements.value.value,
    note: form.elements.note.value,
  };
  const editing = valuationForm.editing;
  return saveDialog({
    dialog: 'house-valuation-dialog',
    error: 'house-valuation-error',
    changed,
    save: () =>
      editing
        ? // The valuation is found by its current period; the field holds the new one (CSD-015 REQ-12).
          invoke('update_house_valuation', { ...args, period: editing.period, newPeriod: args.period })
        : invoke('add_house_valuation', args),
  });
}

/** Fills the value from the latest valuation of the house chosen in the form (CSD-015 REQ-15). */
async function copyPrevious() {
  const form = $('house-valuation-form');
  try {
    const prefill = await invoke('copy_previous_house_valuation', { houseId: form.elements.house_id.value });
    form.elements.value.value = amountForInput(prefill.value);
    form.elements.note.value = '';
  } catch (error) {
    showError('house-valuation-error', error);
  }
}

function deleteValuation(valuation) {
  return deleteWithConfirm({
    title: t('Delete the valuation of {period}?', { period: valuation.period }),
    message: t('The valuation of {period} of {name} and its note will be deleted.', { period: valuation.period, name: valuation.house_name }),
    confirmLabel: t('Delete valuation'),
    command: 'delete_house_valuation',
    args: { houseId: valuation.house_id, period: valuation.period },
    changed,
  });
}

export function initHousesOverview({ onChanged: callback }) {
  onChanged = callback;
  $('house-add').addEventListener('click', () => openHouseDialog(null));
  $('house-add-first').addEventListener('click', () => openHouseDialog(null));
  submitOnce($('house-form'), submitHouse);
  $('house-valuation-add').addEventListener('click', () => valuationForm.open());
  $('house-valuation-copy').addEventListener('click', copyPrevious);
  submitOnce($('house-valuation-form'), submitValuation);
  $('house-valuation-form').elements.house_id.addEventListener('change', () => valuationForm.apply({ resetPeriod: true }));
}
