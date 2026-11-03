// Entry point: wires the project controls, the income, investments, bank accounts, houses, debts and expenses views (each with an insights chart), the net worth chart, the ratios table, the settings and the dashboard.
import { errorText, listen } from './shared/ipc.js';
import { showToast } from './shared/dom.js';
import { collectStaticTexts, onLanguageChange } from './shared/i18n.js';
import { initDateMasks } from './shared/date-mask.js';
import { initProject, refreshProjectStatus } from './pages/project.js';
import { initIncomeOverview, reloadIncomeOverview } from './pages/income-overview.js';
import { initIncomeInsights, reloadIncomeInsightsIfOpen } from './pages/income-insights.js';
import { initInvestmentsInsights, reloadInvestmentsInsightsIfOpen } from './pages/investments-insights.js';
import { initInvestmentsOverview, reloadInvestmentsOverview } from './pages/investments-overview.js';
import { initBankInsights, reloadBankInsightsIfOpen } from './pages/bank-insights.js';
import { initBankOverview, reloadBankOverview } from './pages/bank-overview.js';
import { initHousesInsights, reloadHousesInsightsIfOpen } from './pages/houses-insights.js';
import { initHousesOverview, reloadHousesOverview } from './pages/houses-overview.js';
import { initDebtsInsights, reloadDebtsInsightsIfOpen } from './pages/debts-insights.js';
import { initDebtsOverview, reloadDebtsOverview } from './pages/debts-overview.js';
import { initExpensesInsights, reloadExpensesInsightsIfOpen } from './pages/expenses-insights.js';
import { initExpensesOverview, reloadExpensesOverview } from './pages/expenses-overview.js';
import { initSettings, loadPreferences, reloadSettings } from './pages/settings.js';
import { reloadDashboard } from './pages/dashboard.js';
import { initNetWorth, reloadNetWorthIfOpen } from './pages/net-worth.js';
import { initRatios, reloadRatiosIfOpen } from './pages/ratios.js';

async function refreshStatusAndSettings() {
  await refreshProjectStatus();
  await reloadSettings();
}

async function start() {
  // The texts of the page are noted before any project data is written into it (CSD-022 CON-3).
  collectStaticTexts();

  // The theme comes first: the page stays hidden until it is applied (CSD-008 REQ-6).
  // If it cannot be loaded the app still starts, with the theme macOS gives it.
  try {
    await loadPreferences();
  } catch (error) {
    showToast(errorText(error));
  } finally {
    document.documentElement.classList.add('theme-ready');
  }

  initDateMasks();

  document.querySelectorAll('dialog [data-cancel]').forEach((button) => {
    button.addEventListener('click', () => button.closest('dialog').close());
  });

  // Income changes alter the paycheck count the settings page confirms with,
  // and the dashboard figures.
  initIncomeOverview({
    onChanged: async () => {
      await refreshProjectStatus();
      await reloadSettings();
      await reloadDashboard();
    },
  });
  initIncomeInsights();
  initInvestmentsInsights();
  // Investment statements count for the currency confirmation of the settings page.
  initInvestmentsOverview({
    onChanged: refreshStatusAndSettings,
  });
  initBankInsights();
  // Bank entries count for the currency confirmation of the settings page (CSD-009 REQ-24).
  initBankOverview({
    onChanged: refreshStatusAndSettings,
  });
  initDebtsInsights();
  // Debt statements count for the currency confirmation of the settings page (CSD-011 REQ-24).
  // A debt's category and house show in the Houses overview and its Edit form (CSD-020 REQ-29).
  initDebtsOverview({
    onChanged: async () => {
      await refreshStatusAndSettings();
      await reloadHousesOverview();
    },
  });
  initHousesInsights();
  // House valuations count for the currency confirmation of the settings page (CSD-015 REQ-25).
  // A sale or its undo settles or unsettles the linked debts, so the Debts overview follows (CSD-020 REQ-20, REQ-21).
  initHousesOverview({
    onChanged: async () => {
      await refreshStatusAndSettings();
      await reloadDebtsOverview();
    },
  });
  initExpensesInsights();
  // Expenses count for the currency confirmation of the settings page (CSD-013 REQ-17).
  initExpensesOverview({
    onChanged: refreshStatusAndSettings,
  });
  initNetWorth();
  initRatios();
  initSettings({
    onChanged: async () => {
      await refreshProjectStatus();
    },
  });
  // Everything is drawn again after a project replaced the data, and after a language change (CSD-022 REQ-3).
  const reloadEverything = async () => {
    await reloadSettings();
    await reloadIncomeOverview();
    await reloadIncomeInsightsIfOpen();
    await reloadInvestmentsOverview();
    await reloadInvestmentsInsightsIfOpen();
    await reloadBankOverview();
    await reloadBankInsightsIfOpen();
    await reloadDebtsOverview();
    await reloadDebtsInsightsIfOpen();
    await reloadHousesOverview();
    await reloadHousesInsightsIfOpen();
    await reloadExpensesOverview();
    await reloadExpensesInsightsIfOpen();
    await reloadNetWorthIfOpen();
    await reloadRatiosIfOpen();
    await reloadDashboard();
  };
  await initProject({ onReplaced: reloadEverything });
  onLanguageChange(async () => {
    await refreshProjectStatus();
    await reloadEverything();
  });

  // Settings… in the app menu, and ⌘, (CSD-003 REQ-2).
  await listen('menu-action', (event) => {
    if (event.payload === 'open-settings') {
      document.querySelector('[data-section="settings"]').click();
    }
  });

  // Opening the dashboard recomputes it, so it never shows stale months.
  document
    .querySelector('[data-section="dashboard"]')
    .addEventListener('click', () => reloadDashboard().catch((e) => showToast(errorText(e))));

  await reloadSettings();
  await reloadIncomeOverview();
  await reloadInvestmentsOverview();
  await reloadBankOverview();
  await reloadDebtsOverview();
  await reloadHousesOverview();
  await reloadExpensesOverview();
  await reloadDashboard();
}

start().catch((error) => showToast(errorText(error)));
