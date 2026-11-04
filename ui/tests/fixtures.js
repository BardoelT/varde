// Canned answers of the fake Tauri bridge. The example project is the synthetic one of the golden
// tests (`src-tauri/golden/ipc-reads.json`), so the screens are fed what Rust really returns.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const golden = JSON.parse(readFileSync(fileURLToPath(new URL('../../src-tauri/golden/ipc-reads.json', import.meta.url)), 'utf8'));

/** The commands every page calls while it starts, apart from the reads of the golden file. */
const startup = {
  get_preferences: { theme: 'system', language: 'en' },
  take_startup_notice: null,
  // Writes that only echo what was sent, as Rust does when the save succeeds (CSD-008, CSD-022).
  set_language: ({ language }) => ({ language }),
  set_theme: ({ theme }) => ({ theme }),
};

/** A copy of what Rust returned for every read command of the example project (a project with data). */
export function exampleReads() {
  return { ...structuredClone(golden), ...startup, get_preferences: { ...startup.get_preferences } };
}

const emptyList = { total: '0.00' };

/** The answers of a new, empty project: no jobs, accounts, houses, debts or expenses, and no years. */
export function emptyReads() {
  const reads = exampleReads();
  const year = golden.get_dashboard.year;
  return {
    ...reads,
    get_dashboard: { has_jobs: false, last_month: null, last_month_totals: null, year, year_to_date: null },
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
    yearly_expense_totals: [],
    get_expense_insights: { categories: golden.get_expense_insights.categories, years: [] },
    get_net_worth_insights: { years: [] },
    get_ratios: { years: [] },
  };
}
