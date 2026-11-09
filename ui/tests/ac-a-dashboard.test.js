// Display ACs of CSD-004 (the dashboard): the tiles, their order, labels, change indicators, colours,
// "No data yet", the click-through, and the refresh. Rust computes every figure; the fake bridge
// answers what it would.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { exampleReads, emptyDashboard } from './fixtures.js';

const tile = (app, key) => app.document.querySelector(`.tile[data-tile="${key}"]`);
const tileText = (app, key) => app.text(tile(app, key)).replace(/\s+/g, ' ');
const up = { direction: 'up', sentiment: 'good', percent_label: '+5.00%' };

/** A dashboard with data on every tile, built from the empty one. */
function filled(overrides = {}) {
  const board = emptyDashboard();
  const amount = (value, extra = {}) => ({ has_data: true, value, as_of: null, change: null, ...extra });
  Object.assign(board, {
    net_worth: amount('177000.00', { as_of: '2026-10', change: up }),
    income: amount('6000.00', { change: up }),
    expenses: amount('900.00', { change: { direction: 'up', sentiment: 'bad', percent_label: '+12.00%' } }),
    bank_accounts: amount('7000.00', { as_of: '2026-10', change: { direction: 'stable', sentiment: 'neutral', percent_label: '0.00%' } }),
    investments: amount('20000.00', { as_of: '2026-10' }),
    houses: amount('300000.00', { as_of: '2026-10', change: up }),
    debts: amount('150000.00', { as_of: '2026-10', change: { direction: 'down', sentiment: 'good', percent_label: '-6.25%' } }),
  });
  return { ...board, ...overrides };
}

test('test_csd004_ac_6_1_no_currency_in_the_tiles', async () => {
  const reads = exampleReads();
  reads.get_settings = { currency: 'USD', entry_count: 2 };
  reads.get_dashboard = filled();
  const app = await openApp({ reads });
  assert.match(tileText(app, 'income'), /6,000\.00/);
  assert.doesNotMatch(app.textOf('dashboard-tiles'), /USD|EUR|GBP|[$€£]/, 'no tile names a currency');
  app.close();
});

test('test_csd004_ac_7_1_a_new_paycheck_shows_up', async () => {
  let dashboard = filled();
  const handlers = {
    get_dashboard: () => dashboard,
    add_paycheck: () => {
      dashboard = filled({ income: { ...dashboard.income, value: '6500.00' } });
      return {};
    },
  };
  const app = await openApp({ handlers });
  assert.match(tileText(app, 'income'), /6,000\.00/);

  await app.click(app.item('income'));
  await app.click(app.$('paycheck-add'));
  const form = app.$('paycheck-form');
  await app.choose(form.elements.job_id, 'id-9');
  await app.type(form.elements.period, '202610');
  await app.type(form.elements.gross_salary, '500.00');
  form.requestSubmit();
  await app.settle();
  assert.equal(app.callsOf('add_paycheck').length, 1, 'the paycheck is saved');

  await app.click(app.item('dashboard'));
  assert.match(tileText(app, 'income'), /6,500\.00/, 'the Income tile shows 6500.00');
  app.close();
});

test('test_csd004_ac_7_2_a_new_project_refreshes_the_dashboard', async () => {
  let dashboard = filled();
  const handlers = {
    get_dashboard: () => dashboard,
    new_project: () => {
      dashboard = emptyDashboard();
      return { has_path: false, unsaved_changes: false };
    },
  };
  const app = await openApp({ handlers });
  assert.match(tileText(app, 'income'), /6,000\.00/);
  await app.emit('menu-action', 'project-new');
  assert.equal(app.callsOf('new_project').length, 1);
  for (const key of Object.keys(dashboard)) {
    assert.match(tileText(app, key), /No data yet/, `${key} says "No data yet"`);
  }
  app.close();
});

test('test_csd004_ac_8_1_the_tiles_and_their_order', async () => {
  const reads = exampleReads();
  reads.get_dashboard = filled();
  const app = await openApp({ reads });
  const tiles = [...app.document.querySelectorAll('#dashboard-tiles .tile')];
  assert.deepEqual(
    tiles.map((node) => app.text(node.querySelector('.tile-title'))),
    ['Net worth', 'Income', 'Expenses', 'Bank accounts', 'Investments', 'Houses', 'Debts'],
  );
  assert.equal(tiles[0].classList.contains('tile-hero'), true, 'Net worth is the hero tile');
  assert.equal(tiles.filter((node) => node.classList.contains('tile-hero')).length, 1);
  app.close();
});

test('test_csd004_ac_8_2_there_is_no_tile_for_settings_or_ratios', async () => {
  const reads = exampleReads();
  reads.get_dashboard = filled();
  const app = await openApp({ reads });
  const tiles = app.document.querySelectorAll('#dashboard-tiles .tile');
  assert.equal(tiles.length, 7);
  assert.equal([...tiles].some((node) => /Settings|Ratios/.test(app.text(node))), false);
  app.close();
});

test('test_csd004_ac_11_1_the_change_shows_an_arrow_and_the_percentage', async () => {
  const reads = exampleReads();
  reads.get_dashboard = filled();
  const app = await openApp({ reads });
  const change = tile(app, 'income').querySelector('.tile-change');
  assert.equal(app.text(change.querySelector('.tile-arrow')), '▲');
  assert.equal(app.text(change.querySelector('.tile-percent')), '+5.00%');
  const down = tile(app, 'debts').querySelector('.tile-change');
  assert.equal(app.text(down.querySelector('.tile-arrow')), '▼');
  assert.equal(app.text(down.querySelector('.tile-percent')), '-6.25%');
  const stable = tile(app, 'bank_accounts').querySelector('.tile-change');
  assert.equal(app.text(stable.querySelector('.tile-arrow')), '–');
  assert.equal(app.text(stable.querySelector('.tile-percent')), '0.00%');
  app.close();
});

test('test_csd004_ac_11_5_no_comparison_keeps_the_headline_with_a_grey_dash', async () => {
  const reads = exampleReads();
  reads.get_dashboard = filled();
  const app = await openApp({ reads });
  assert.match(tileText(app, 'investments'), /20,000\.00/);
  const change = tile(app, 'investments').querySelector('.tile-change');
  assert.equal(change.classList.contains('neutral'), true);
  assert.equal(app.text(change), '–');
  assert.equal(change.querySelector('.tile-percent'), null, 'no percentage');
  app.close();
});

test('test_csd004_ac_12_1_the_colour_follows_good_and_bad', async () => {
  const reads = exampleReads();
  reads.get_dashboard = filled();
  const app = await openApp({ reads });
  const classOf = (key) => tile(app, key).querySelector('.tile-change').className;
  assert.match(classOf('income'), /\bgood\b/, 'more income is green');
  assert.match(classOf('expenses'), /\bbad\b/, 'more expenses are red');
  assert.match(classOf('debts'), /\bgood\b/, 'less debt is green');
  assert.match(classOf('bank_accounts'), /\bneutral\b/, 'stable is grey');
  // The direction is also in the text, so it does not depend on colour alone.
  assert.match(tile(app, 'expenses').querySelector('.tile-change').getAttribute('aria-label'), /^Up \+12\.00%$/);
  app.close();
});

test('test_csd004_ac_13_1_balance_tiles_show_their_as_of_period', async () => {
  const reads = exampleReads();
  reads.get_dashboard = filled();
  const app = await openApp({ reads });
  assert.match(tileText(app, 'bank_accounts'), /as of 2026-10/);
  assert.doesNotMatch(tileText(app, 'income'), /as of/);
  app.close();
});

test('test_csd004_ac_13_2_flow_tiles_are_labelled', async () => {
  const reads = exampleReads();
  reads.get_dashboard = filled();
  const app = await openApp({ reads });
  assert.match(tileText(app, 'income'), /Year to date/);
  assert.match(tileText(app, 'expenses'), /Year to date/);
  assert.doesNotMatch(tileText(app, 'bank_accounts'), /Year to date/);
  app.close();
});

test('test_csd004_ac_14_1_every_tile_says_no_data_yet_in_an_empty_project', async () => {
  const reads = exampleReads();
  reads.get_dashboard = emptyDashboard();
  const app = await openApp({ reads });
  const tiles = [...app.document.querySelectorAll('#dashboard-tiles .tile')];
  assert.equal(tiles.length, 7);
  for (const node of tiles) {
    assert.match(app.text(node), /No data yet/);
    assert.equal(node.querySelector('.tile-change'), null, 'no change, no figure');
  }
  app.close();
});

test('test_csd004_ac_15_1_a_tile_opens_its_page', async () => {
  const reads = exampleReads();
  reads.get_dashboard = filled();
  const app = await openApp({ reads });
  await app.click(tile(app, 'debts'));
  assert.equal(app.isShown(app.$('debts')), true, 'the Debts page is shown');
  assert.equal(app.isShown(app.$('dashboard')), false, 'the Dashboard page is hidden');
  assert.equal(app.item('debts').classList.contains('active'), true);
  app.close();
});

test('test_csd004_ac_15_2_an_empty_tile_opens_its_page', async () => {
  const reads = exampleReads();
  reads.get_dashboard = emptyDashboard();
  const app = await openApp({ reads });
  const button = tile(app, 'income');
  assert.equal(button.tagName, 'BUTTON', 'every tile is a button for keyboard and screen readers');
  await app.click(button);
  assert.equal(app.isShown(app.$('income')), true);
  app.close();
});

test('test_csd004_ac_15_3_net_worth_opens_its_page', async () => {
  const reads = exampleReads();
  reads.get_dashboard = filled();
  const app = await openApp({ reads });
  await app.click(tile(app, 'net_worth'));
  assert.equal(app.isShown(app.$('net-worth')), true, 'the Net worth page is shown');
  app.close();
});
