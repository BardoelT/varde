// Canned answers of the fake Tauri bridge. The example project is the synthetic one of the golden
// tests (`src-tauri/golden/ipc-reads.json`), so the screens are fed what Rust really returns.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const golden = JSON.parse(readFileSync(fileURLToPath(new URL('../../src-tauri/golden/ipc-reads.json', import.meta.url)), 'utf8'));

/** The commands every page calls while it starts, apart from the reads of the golden file. */
const startup = {
  get_preferences: { theme: 'system', language: 'en', number_format: 'point_comma', discrete_mode: false, autosave: true },
  take_startup_notice: null,
  app_version: '0.1.0',
  // The transactions chart is not part of the golden files: its category ids would renumber the ids of the others.
  get_transaction_insights: {
    granularity: 'yearly',
    categories: [
      ...golden.list_transaction_categories.map(({ id, name }, index) => ({ id, name, colour_slot: index % 8 })),
      { id: 'uncategorized', name: 'Uncategorized', colour_slot: 7 },
    ],
    axis: { ticks: [{ value: '0.00', label: '0' }, { value: '1.00', label: '1' }] },
    years: [],
  },
  // The donut charts ask for one period; the golden file holds one year and one month (CSD-032 REQ-35).
  transaction_spending_shares: ({ period }) => (period.length === 4 ? golden.transaction_spending_shares_year : golden.transaction_spending_shares_month),
  // The budgets are not part of the golden files either (CSD-037 CON-5): a month without any budget.
  get_budgets: {
    month: '2026-10',
    is_partial: true,
    earliest_month: '2026-10',
    rows: golden.list_transaction_categories.map(({ id, name }) => ({ category: id, name, budget: null, single_month: false, spent: '0.00', remaining: null, bar_percent: null, over: false })),
    not_budgeted: '0.00',
    total: { budget: '0.00', spent: '0.00', remaining: '0.00' },
  },
  get_budget_year: {
    year: 2026,
    is_partial: true,
    earliest_year: 2026,
    figures: { budget: '0.00', spent: '0.00', remaining: '0.00' },
    rows: golden.list_transaction_categories.map(({ id, name }) => ({ category: id, name, months: Array.from({ length: 12 }, () => ({ budget: null, single_month: false })), total: null })),
    months: Array.from({ length: 12 }, () => null),
    total: null,
  },
  // Writes that only echo what was sent, as Rust does when the save succeeds (CSD-008, CSD-022).
  set_language: ({ language }) => ({ language }),
  set_theme: ({ theme }) => ({ theme }),
  set_number_format: ({ numberFormat }) => ({ number_format: numberFormat }),
  set_discrete_mode: ({ discreteMode }) => ({ discrete_mode: discreteMode }),
  set_autosave: ({ autosave }) => ({ autosave }),
};

/** A copy of what Rust returned for every read command of the example project (a project with data). */
export function exampleReads() {
  const reads = { ...structuredClone(golden), ...startup, get_preferences: { ...startup.get_preferences } };
  // The page loads these four reads in one answer; the golden file holds each of them (CSD-032 REQ-34).
  reads.get_transactions_overview = {
    accounts: reads.list_transaction_accounts,
    categories: reads.list_transaction_categories,
    rules: reads.list_transaction_rules,
    summary: reads.get_categorisation_summary,
    totals: reads.yearly_transaction_totals,
  };
  return reads;
}

const emptyList = { total: '0.00' };

/** What Rust returns for the dashboard of an empty project: the eight tiles, none with data (CSD-004 REQ-14). */
export function emptyDashboard() {
  const none = () => ({ has_data: false, value: null, as_of: null, change: null });
  return {
    net_worth: none(),
    income: none(),
    expenses: none(),
    bank_accounts: none(),
    investments: none(),
    houses: none(),
    debts: none(),
  };
}

/** The answers of a new, empty project: no jobs, accounts, houses, debts or expenses, and no years. */
export function emptyReads() {
  const reads = exampleReads();
  return {
    ...reads,
    get_dashboard: emptyDashboard(),
    get_settings: { currency: 'EUR', entry_count: 0 },
    project_status: { has_path: false, unsaved_changes: false },
    list_jobs: [],
    list_entries: [],
    yearly_totals: [],
    get_income_insights: { years: [] },
    list_investment_accounts: { accounts: [], ...emptyList },
    list_investment_statements: [],
    yearly_investment_totals: [],
    get_investment_insights: { years: [] },
    list_bank_accounts: { accounts: [], ...emptyList },
    list_bank_entries: [],
    yearly_bank_totals: [],
    get_bank_insights: { banks: [], years: [] },
    list_debts: { debts: [], ...emptyList },
    list_debt_statements: [],
    yearly_debt_totals: [],
    get_debt_insights: { debts: [], years: [] },
    list_houses: { houses: [], ...emptyList },
    list_house_valuations: [],
    yearly_house_values: [],
    get_house_insights: { houses: [], years: [] },
    list_expenses: { expenses: [], ...emptyList },
    list_transactions: { rows: [], count: 0, total: '0.00' },
    yearly_transaction_totals: [],
    get_transactions_overview: { ...reads.get_transactions_overview, accounts: [], summary: { total: 0, categorized: 0, percent_label: null }, totals: [] },
    yearly_expense_totals: [],
    get_expense_insights: { categories: golden.get_expense_insights.categories, years: [] },
    get_net_worth_insights: { years: [] },
    get_ratios: { years: [] },
  };
}
