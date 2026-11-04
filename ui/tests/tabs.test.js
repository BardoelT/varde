// CSD-025 REQ-15: the Overview and Insights tabs of a page.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { TAB_PAGES } from './areas.js';

test('test_csd025_ac_15_1_switching_to_insights', async () => {
  for (const { page, insights } of TAB_PAGES) {
    const app = await openApp();
    await app.click(app.item(page));
    const [overviewTab, insightsTab] = [app.$(`${page}-tab-overview`), app.$(`${page}-tab-insights`)];
    assert.equal(overviewTab.getAttribute('aria-selected'), 'true', `${page}: the Overview tab is shown first`);
    assert.equal(app.isShown(app.$(`${page}-overview`)), true);
    assert.equal(app.isShown(app.$(`${page}-insights`)), false);

    const before = app.callsOf(insights).length;
    await app.click(insightsTab);
    assert.equal(app.isShown(app.$(`${page}-overview`)), false, 'the Overview sections are hidden');
    assert.equal(app.isShown(app.$(`${page}-insights`)), true);
    assert.equal(app.isShown(app.$(`${page}-insights-scroll`)), true, 'the chart is shown');
    assert.ok(app.$(`${page}-insights-chart`).querySelector('svg .chart-year-group'), 'the chart is drawn');
    assert.equal(insightsTab.getAttribute('aria-selected'), 'true');
    assert.equal(overviewTab.getAttribute('aria-selected'), 'false');
    assert.equal(app.callsOf(insights).length, before + 1, `${page}: the figures are requested when the tab opens`);
    app.close();
  }
});

test('test_csd025_ac_15_2_arrow_keys_move_between_tabs', async () => {
  for (const { page } of TAB_PAGES) {
    const app = await openApp();
    await app.click(app.item(page));
    const [overviewTab, insightsTab] = [app.$(`${page}-tab-overview`), app.$(`${page}-tab-insights`)];
    const arrow = async (tab, key) => {
      tab.focus();
      const event = new app.window.KeyboardEvent('keydown', { key, bubbles: true, cancelable: true });
      tab.dispatchEvent(event);
      assert.equal(event.defaultPrevented, true, 'the arrow key does not scroll the page');
      await app.settle();
    };

    await arrow(overviewTab, 'ArrowRight');
    assert.equal(insightsTab.getAttribute('aria-selected'), 'true');
    assert.equal(app.document.activeElement, insightsTab, 'the Insights tab has focus');
    assert.equal(app.isShown(app.$(`${page}-insights`)), true);
    assert.equal(overviewTab.tabIndex, -1, 'only the selected tab is in the tab order');

    await arrow(insightsTab, 'ArrowLeft');
    assert.equal(overviewTab.getAttribute('aria-selected'), 'true');
    assert.equal(app.document.activeElement, overviewTab);
    assert.equal(app.isShown(app.$(`${page}-overview`)), true);

    await arrow(overviewTab, 'ArrowLeft');
    assert.equal(app.document.activeElement, insightsTab, 'the arrow keys wrap around');
    app.close();
  }
});

test('test_csd025_ac_15_3_figures_follow_changes_when_the_insights_tab_opens_again', async () => {
  const app = await openApp();
  await app.click(app.item('income'));
  await app.click(app.$('income-tab-insights'));
  const groups = () => app.$('income-insights-chart').querySelectorAll('.chart-year-group').length;
  const first = app.bridge.table.get_income_insights.years.length;
  assert.equal(groups(), first);

  // The user changes the figures on the Overview tab: Rust answers differently from now on.
  await app.click(app.$('income-tab-overview'));
  app.bridge.table.get_income_insights = { ...app.bridge.table.get_income_insights, years: app.bridge.table.get_income_insights.years.slice(1) };
  assert.equal(groups(), first, 'nothing is requested while the Overview tab is shown');

  await app.click(app.$('income-tab-insights'));
  assert.equal(groups(), first - 1, 'the chart shows what Rust answers now');
  app.close();
});
