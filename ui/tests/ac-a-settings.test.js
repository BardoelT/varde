// Display ACs of CSD-003 (project settings): the Settings page, opening it, the currency
// confirmation and the pages without a currency. Rust decides how many entries a project has
// (`get_settings.entry_count`); the page only asks for a confirmation when it is above zero.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { exampleReads } from './fixtures.js';

const MESSAGE = 'Existing amounts keep their values and only the currency changes. Amounts are not converted.';

/** An app whose project is in EUR with the given number of entries; `set_currency` answers as Rust does. */
async function appWithEntries(entryCount, currency = 'EUR') {
  const reads = exampleReads();
  reads.get_settings = { currency, entry_count: entryCount };
  const handlers = { set_currency: ({ currency: requested }) => ({ currency: requested, entry_count: entryCount }) };
  const app = await openApp({ reads, handlers });
  await app.click(app.item('settings'));
  return app;
}

test('test_csd003_ac_1_1_settings_is_the_last_item_in_the_left_pane', async () => {
  const app = await openApp();
  const sidebar = app.document.querySelector('.sidebar');
  const order = [...sidebar.children].map((node) => node.className.split(' ')[0]);
  const settings = order.indexOf('sidebar-settings');
  assert.ok(settings > order.indexOf('sidebar-nav'), 'below the other navigation items');
  assert.equal(settings, order.length - 1, 'last in the left pane');
  assert.equal(app.text(sidebar.querySelector('.sidebar-settings .nav-item')), 'Settings');
  assert.equal(app.document.querySelectorAll('.sidebar-nav .nav-item[data-section="settings"]').length, 0, 'not among the other items');
  app.close();
});

test('test_csd003_ac_1_2_the_page_shows_the_currency_setting', async () => {
  const reads = exampleReads();
  reads.get_settings = { currency: 'GBP', entry_count: 3 };
  const app = await openApp({ reads });
  await app.click(app.item('settings'));
  assert.equal(app.textOf('settings-title'), 'Settings');
  const general = [...app.$('settings').querySelectorAll('.panel')].find((panel) => app.text(panel.querySelector('.panel-title')) === 'General');
  assert.ok(general, 'a General panel');
  assert.equal(app.text(general.querySelector('label[for="setting-currency"]')), 'Currency');
  assert.equal(app.$('setting-currency').value, 'GBP', 'the current project currency');
  assert.deepEqual([...app.$('setting-currency').options].map((option) => option.value), ['EUR', 'USD', 'GBP']);
  app.close();
});

for (const [page, label] of [['dashboard', 'Dashboard'], ['income', 'Income']]) {
  test(`test_csd003_ac_2_2_opening_settings_from_the_menu_while_viewing_${label.toLowerCase()}`, async () => {
    const app = await openApp();
    await app.click(app.item(page));
    assert.equal(app.isShown(app.$(page)), true);
    await app.emit('menu-action', 'open-settings');
    assert.equal(app.isShown(app.$('settings')), true, 'the Settings page is shown');
    assert.equal(app.isShown(app.$(page)), false);
    app.close();
  });
}

test('test_csd003_ac_5_1_a_confirmation_says_the_amounts_are_not_converted_and_cancel_keeps_the_currency', async () => {
  const app = await appWithEntries(1);
  await app.choose(app.$('setting-currency'), 'USD');
  assert.equal(app.$('confirm-dialog').open, true);
  assert.equal(app.textOf('confirm-message'), MESSAGE);
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.$('confirm-dialog').open, false);
  assert.equal(app.callsOf('set_currency').length, 0, 'nothing is sent to Rust');
  assert.equal(app.$('setting-currency').value, 'EUR', 'EUR is shown on the page again');
  app.close();
});

test('test_csd003_ac_5_2_confirming_applies_the_change', async () => {
  const app = await appWithEntries(1);
  await app.choose(app.$('setting-currency'), 'USD');
  await app.click(app.$('confirm-ok'));
  assert.deepEqual(app.callsOf('set_currency').map((call) => call.args), [{ currency: 'USD' }]);
  assert.equal(app.$('setting-currency').value, 'USD');
  app.close();
});

test('test_csd003_ac_5_3_no_confirmation_without_entries', async () => {
  const app = await appWithEntries(0);
  await app.choose(app.$('setting-currency'), 'USD');
  assert.equal(app.$('confirm-dialog').open, false, 'no confirmation is asked');
  assert.deepEqual(app.callsOf('set_currency').map((call) => call.args), [{ currency: 'USD' }]);
  assert.equal(app.$('setting-currency').value, 'USD');
  app.close();
});

for (const entries of [1, 5]) {
  test(`test_csd003_ac_5_15_the_confirmation_states_no_number_of_entries_with_${entries}_entries`, async () => {
    const app = await appWithEntries(entries);
    await app.choose(app.$('setting-currency'), 'USD');
    assert.equal(app.textOf('confirm-message'), MESSAGE);
    assert.doesNotMatch(app.textOf('confirm-title') + app.textOf('confirm-message'), /\d+\s*(entries|paychecks|statements|valuations)|\b[15]\b/, 'no number of entries');
    app.close();
  });
}

test('test_csd003_ac_8_1_other_views_show_no_currency', async () => {
  const app = await appWithEntries(2, 'USD');
  await app.click(app.item('income'));
  const titles = [...app.$('income-overview').querySelectorAll('.panel-title')].map((title) => app.text(title));
  assert.deepEqual(titles, ['Jobs', 'Yearly totals', 'Paychecks and statements']);
  for (const id of ['job-list', 'paycheck-table', 'totals-table']) {
    assert.doesNotMatch(app.textOf(id), /EUR|USD|GBP|[$€£]/, `${id} shows no currency`);
  }
  for (const section of app.sections().filter((node) => node.id !== 'settings')) {
    assert.doesNotMatch(app.text(section), /\b(EUR|USD|GBP)\b|[$€£]/, `${section.id} shows no currency`);
  }
  assert.equal(app.$('setting-currency').value, 'USD', 'only the Settings page shows USD');
  app.close();
});
