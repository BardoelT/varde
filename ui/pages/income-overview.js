// Employment income: jobs, yearly totals over all jobs, and the paychecks and annual statements of all jobs (CSD-001).
import { invoke, errorText } from '../shared/ipc.js';
import { $, h, showError, showToast, setHint, submitOnce } from '../shared/dom.js';
import { amountForInput, formatAmount, formatDate, currentYear, currentMonth } from '../shared/format.js';
import { t } from '../shared/i18n.js';
import { createStatementFilter, renderFilteredTable } from '../shared/statement-filter.js';
import { openDialog, saveDialog, deleteWithConfirm } from '../shared/crud.js';
import { accountRow, numCell, recordActions, renderTable, rowHeader, toggleEmpty } from '../shared/list-rows.js';

let jobs = [];
let onChanged = async () => {};

function jobDates(job) {
  return `${formatDate(job.start_date)} – ${job.end_date ? formatDate(job.end_date) : t('present')}`;
}

/** The pay period as Rust names it, in the chosen language. */
function payPeriodName(payPeriod) {
  return payPeriod === 'Yearly' ? t('Yearly') : t('Monthly');
}

function jobMeta(job) {
  return [job.title, jobDates(job), payPeriodName(job.period)].filter(Boolean).join(' · ');
}

const amountCell = (value) => numCell(formatAmount(value));

// ---- Rendering ----

function renderJobs() {
  const hasJobs = jobs.length > 0;
  toggleEmpty('income-empty', 'income-content', hasJobs);
  $('job-add').hidden = !hasJobs;
  $('job-list').replaceChildren(
    ...jobs.map((job) =>
      accountRow({
        name: job.employer,
        meta: jobMeta(job),
        edit: { label: t('Edit {name}', { name: job.employer }), onClick: () => openJobDialog(job) },
        remove: { label: t('Delete {name}', { name: job.employer }), onClick: () => deleteJob(job) },
      }),
    ),
  );
}

function renderTotals(totals) {
  renderTable({ empty: 'totals-empty', table: 'totals-table', body: 'totals-rows' }, totals, (total) =>
    h(
      'tr',
      {},
      rowHeader(String(total.year)),
      amountCell(total.gross_salary),
      amountCell(total.holiday_pay),
      amountCell(total.bonus),
      amountCell(total.total_gross),
    ),
  );
}

const entriesFilter = createStatementFilter({
  periodHost: 'paycheck-period-filter',
  entityHost: 'paycheck-entity-filter',
  entityTitle: 'Job',
  entities: () => jobs.filter((job) => job.entry_count > 0).map((job) => ({ id: job.id, label: job.employer })),
  onChange: () => reloadEntries().catch((error) => showToast(errorText(error))),
});

/** Loads the rows through the filter and shows them (CSD-028). */
async function reloadEntries() {
  showEntries(await entriesFilter.load('list_entries', (rows) => rows.length));
}

function showEntries({ result, hasData }) {
  entriesFilter.show(hasData);
  renderEntries(result, hasData);
}

function renderEntries(entries, hasData) {
  renderFilteredTable({ empty: 'paycheck-empty', nomatch: 'paycheck-nomatch', table: 'paycheck-table', body: 'paycheck-rows' }, { rows: entries, hasData }, (e) =>
    h(
      'tr',
      {},
      rowHeader(e.period),
      h('td', { class: 'wrap' }, e.employer),
      amountCell(e.gross_salary),
      amountCell(e.holiday_pay),
      amountCell(e.bonus),
      amountCell(e.total_gross),
      recordActions({
        edit: { label: t('Edit {name} {period}', { name: e.employer, period: e.period }), onClick: () => openEntryDialog(e) },
        remove: { label: t('Delete {name} {period}', { name: e.employer, period: e.period }), onClick: () => deleteEntry(e) },
      }),
    ),
  );
}

/** Reloads everything from Rust. */
export async function reloadIncomeOverview() {
  jobs = await invoke('list_jobs');
  renderJobs();
  if (jobs.length === 0) {
    entriesFilter.show(false);
    return;
  }
  const [totals, loaded] = await Promise.all([invoke('yearly_totals'), entriesFilter.load('list_entries', (rows) => rows.length)]);
  renderTotals(totals);
  showEntries(loaded);
}

async function changed() {
  await reloadIncomeOverview();
  await onChanged();
}

// ---- Job dialog ----

let editingJob = null;

function openJobDialog(job) {
  editingJob = job;
  openDialog({
    dialog: 'job-dialog',
    form: 'job-form',
    error: 'job-error',
    title: 'job-dialog-title',
    titleText: job ? t('Edit job') : t('Add job'),
    focus: 'employer',
    fill(form) {
      // The pay period is locked once the job has entries (CSD-001 REQ-24).
      const locked = Boolean(job && job.entry_count > 0);
      form.elements.period.disabled = locked;
      setHint('job-period-hint', locked);
      if (job) {
        form.elements.period.value = job.period;
        form.elements.employer.value = job.employer;
        form.elements.title.value = job.title ?? '';
        form.elements.start_date.value = formatDate(job.start_date);
        form.elements.end_date.value = job.end_date ? formatDate(job.end_date) : '';
      }
    },
  });
}

function submitJob() {
  const form = $('job-form');
  const input = {
    employer: form.elements.employer.value,
    title: form.elements.title.value,
    start_date: form.elements.start_date.value,
    end_date: form.elements.end_date.value,
  };
  if (!form.elements.period.disabled) input.period = form.elements.period.value;
  return saveDialog({
    dialog: 'job-dialog',
    error: 'job-error',
    changed,
    save: () =>
      editingJob ? invoke('update_job', { jobId: editingJob.id, input }) : invoke('create_job', { input }),
  });
}

function deleteJob(job) {
  return deleteWithConfirm({
    title: t('Delete {name}?', { name: job.employer }),
    message: t('This job and all its paychecks and statements will be deleted.'),
    confirmLabel: t('Delete job'),
    command: 'delete_job',
    args: { jobId: job.id },
    changed,
  });
}

// ---- Entry dialog: a paycheck for a monthly job, an annual statement for a yearly job ----

let editingEntry = null;
/** Whether the form currently asks for a year, so the period resets only when it changes. */
let formIsYearly = false;

function fillJobChoices(selectedId) {
  // The list is in job order, so the first option is the newest job (CSD-001 AC-5.6).
  const select = $('paycheck-form').elements.job_id;
  select.replaceChildren(
    ...jobs.map((job) => h('option', { value: job.id }, t('{employer} · from {date}', { employer: job.employer, date: formatDate(job.start_date) }))),
  );
  select.value = selectedId ?? jobs[0]?.id ?? '';
}

function chosenJob() {
  return jobs.find((job) => job.id === $('paycheck-form').elements.job_id.value);
}

/** Titles and asks for a month or a year, following the chosen job (CSD-001 REQ-28). */
function applyPayPeriod({ resetPeriod }) {
  const form = $('paycheck-form');
  const yearly = chosenJob()?.period === 'Yearly';
  $('paycheck-dialog-title').textContent = yearly ? t('Add annual statement') : t('Add paycheck');
  $('paycheck-period-label').textContent = yearly ? t('Year') : t('Month');
  form.elements.period.placeholder = yearly ? String(new Date().getFullYear() - 1) : 'yyyy-mm';
  form.elements.period.maxLength = yearly ? 4 : 7;
  $('paycheck-copy').textContent = yearly ? t('Copy previous year') : t('Copy previous month');
  if (resetPeriod && yearly !== formIsYearly) {
    form.elements.period.value = yearly ? currentYear() : currentMonth();
  }
  formIsYearly = yearly;
}

/** Opens the form to add an entry, or to edit `entry` when given. */
function openEntryDialog(entry = null) {
  editingEntry = entry;
  const form = $('paycheck-form');
  form.reset();
  $('paycheck-error').hidden = true;
  fillJobChoices(entry?.job_id);
  form.elements.job_id.disabled = Boolean(entry);
  $('paycheck-copy').hidden = Boolean(entry);
  if (entry) {
    const statement = entry.kind === 'statement';
    $('paycheck-dialog-title').textContent = statement
      ? t('Edit annual statement {period}', { period: entry.period })
      : t('Edit paycheck {period}', { period: entry.period });
    $('paycheck-period-label').textContent = statement ? t('Year') : t('Month');
    form.elements.period.maxLength = statement ? 4 : 7;
    form.elements.period.value = entry.period;
    form.elements.gross_salary.value = amountForInput(entry.gross_salary);
    form.elements.holiday_pay.value = amountForInput(entry.holiday_pay);
    form.elements.bonus.value = amountForInput(entry.bonus);
  } else {
    formIsYearly = null; // force the period to its default for the first job
    form.elements.period.value = '';
    applyPayPeriod({ resetPeriod: true });
  }
  $('paycheck-dialog').showModal();
  (entry ? form.elements.gross_salary : form.elements.period).focus();
}

function submitEntry() {
  const form = $('paycheck-form');
  const amounts = {
    gross_salary: form.elements.gross_salary.value,
    holiday_pay: form.elements.holiday_pay.value,
    bonus: form.elements.bonus.value,
  };
  const editing = editingEntry;
  return saveDialog({
    dialog: 'paycheck-dialog',
    error: 'paycheck-error',
    changed,
    save() {
      if (editing) {
        // The entry is found by its current month or year; the field holds the new one (CSD-001 REQ-29).
        const { job_id: jobId, period } = editing;
        const newPeriod = form.elements.period.value;
        return editing.kind === 'statement'
          ? invoke('update_statement', { jobId, year: period, newYear: newPeriod, amounts })
          : invoke('update_paycheck', { jobId, month: period, newMonth: newPeriod, amounts });
      }
      const jobId = form.elements.job_id.value;
      const period = form.elements.period.value;
      return chosenJob()?.period === 'Yearly'
        ? invoke('add_statement', { jobId, year: period, amounts })
        : invoke('add_paycheck', { jobId, month: period, amounts });
    },
  });
}

/** Fills the form from the latest entry of the job chosen in it (CSD-001 REQ-9). */
async function copyPrevious() {
  const form = $('paycheck-form');
  try {
    const prefill = await invoke('copy_previous_entry', { jobId: form.elements.job_id.value });
    form.elements.gross_salary.value = amountForInput(prefill.gross_salary);
    form.elements.holiday_pay.value = amountForInput(prefill.holiday_pay);
    form.elements.bonus.value = amountForInput(prefill.bonus);
  } catch (error) {
    showError('paycheck-error', error);
  }
}

function deleteEntry(entry) {
  const statement = entry.kind === 'statement';
  const values = { period: entry.period, employer: entry.employer };
  return deleteWithConfirm({
    title: t(statement ? 'Delete the annual statement of {period}?' : 'Delete the paycheck of {period}?', values),
    message: t(
      statement
        ? 'The annual statement of {period} of {employer} will be deleted.'
        : 'The paycheck of {period} of {employer} will be deleted.',
      values,
    ),
    confirmLabel: t(statement ? 'Delete annual statement' : 'Delete paycheck'),
    command: statement ? 'delete_statement' : 'delete_paycheck',
    args: statement ? { jobId: entry.job_id, year: entry.period } : { jobId: entry.job_id, month: entry.period },
    changed,
  });
}

export function initIncomeOverview({ onChanged: callback }) {
  onChanged = callback;
  $('job-add').addEventListener('click', () => openJobDialog(null));
  $('job-add-first').addEventListener('click', () => openJobDialog(null));
  submitOnce($('job-form'), submitJob);
  $('paycheck-add').addEventListener('click', () => openEntryDialog());
  $('paycheck-copy').addEventListener('click', copyPrevious);
  submitOnce($('paycheck-form'), submitEntry);
  $('paycheck-form').elements.job_id.addEventListener('change', () => applyPayPeriod({ resetPeriod: true }));
}
