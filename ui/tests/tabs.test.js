// CSD-025 REQ-15 and CSD-035 REQ-6: the tabs of a page, switching with the mouse and the arrow keys.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { TAB_PAGES } from './areas.js';

test('test_csd025_ac_15_1_switching_to_insights', async () => {
  for (const { page, first, insights } of TAB_PAGES) {
    const app = await openApp();
    await app.click(app.item(page));
    const [firstTab, insightsTab] = [app.$(`${page}-tab-${first}`), app.$(`${page}-tab-insights`)];
    assert.equal(firstTab.getAttribute('aria-selected'), 'true', `${page}: the first tab is shown first`);
    assert.equal(app.isShown(app.$(`${page}-${first}`)), true);
    assert.equal(app.isShown(app.$(`${page}-insights`)), false);

    const before = app.callsOf(insights).length;
    await app.click(insightsTab);
    assert.equal(app.isShown(app.$(`${page}-${first}`)), false, 'the first panel is hidden');
    assert.equal(app.isShown(app.$(`${page}-insights`)), true);
    assert.equal(app.isShown(app.$(`${page}-insights-scroll`)), true, 'the chart is shown');
    assert.ok(app.$(`${page}-insights-chart`).querySelector('svg .chart-year-group'), 'the chart is drawn');
    assert.equal(insightsTab.getAttribute('aria-selected'), 'true');
    assert.equal(firstTab.getAttribute('aria-selected'), 'false');
    assert.equal(app.callsOf(insights).length, before + 1, `${page}: the figures are requested when the tab opens`);
    app.close();
  }
});

test('test_csd025_ac_15_2_arrow_keys_move_between_tabs', async () => {
  for (const { page, first } of TAB_PAGES) {
    const app = await openApp();
    await app.click(app.item(page));
    const tabs = [...app.$(`${page}-tab-insights`).parentElement.querySelectorAll('[role="tab"]')];
    const [firstTab, insightsTab] = [app.$(`${page}-tab-${first}`), app.$(`${page}-tab-insights`)];
    const arrow = async (tab, key) => {
      tab.focus();
      const event = new app.window.KeyboardEvent('keydown', { key, bubbles: true, cancelable: true });
      tab.dispatchEvent(event);
      assert.equal(event.defaultPrevented, true, 'the arrow key does not scroll the page');
      await app.settle();
    };

    // Right through every tab up to Insights.
    for (let i = 0; i < tabs.length - 1; i++) await arrow(tabs[i], 'ArrowRight');
    assert.equal(insightsTab.getAttribute('aria-selected'), 'true');
    assert.equal(app.document.activeElement, insightsTab, 'the Insights tab has focus');
    assert.equal(app.isShown(app.$(`${page}-insights`)), true);
    assert.equal(firstTab.tabIndex, -1, 'only the selected tab is in the tab order');

    await arrow(insightsTab, 'ArrowLeft');
    assert.equal(tabs[tabs.length - 2].getAttribute('aria-selected'), 'true');

    await arrow(tabs[tabs.length - 2], 'ArrowRight');
    await arrow(insightsTab, 'ArrowRight');
    assert.equal(app.document.activeElement, firstTab, 'the arrow keys wrap around');
    assert.equal(firstTab.getAttribute('aria-selected'), 'true');
    assert.equal(app.isShown(app.$(`${page}-${first}`)), true);

    await arrow(firstTab, 'ArrowLeft');
    assert.equal(app.document.activeElement, insightsTab, 'and wrap around the other way');
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

  // The user changes the figures on another tab: Rust answers differently from now on.
  await app.click(app.$('income-tab-jobs'));
  app.bridge.table.get_income_insights = { ...app.bridge.table.get_income_insights, years: app.bridge.table.get_income_insights.years.slice(1) };
  assert.equal(groups(), first, 'nothing is requested while Insights is not shown');

  await app.click(app.$('income-tab-insights'));
  assert.equal(groups(), first - 1, 'the chart shows what Rust answers now');
  app.close();
});
