// The frame shared by the yearly insights charts (CSD-006 income, CSD-007
// investments): plot size, axis and gridlines, one slot per year, the stacked
// bars, the tooltip, the legend highlight, and loading with its empty note.
//
// A page supplies a strategy, described at `createYearlyChart`. Rust returns the
// axis, pieces, markers and labels; this file draws them as received, so the only
// arithmetic here is turning received values into positions (CSD-025 REQ-5).
import { invoke } from './ipc.js';
import { t } from './i18n.js';
import { formatAmountLabel } from './number-format.js';
import { svg, toPosition, segmentShape, createTooltip, bindHighlight } from './chart.js';

const MARGIN = { top: 28, right: 16, left: 48 };
const MIN_PLOT_WIDTH = 560;
const MAX_BAR_WIDTH = 44;

/** Whether a received decimal string or label carries a minus sign (display only: it styles a negative figure). */
export const isNegative = (text) => text.startsWith('-');

/** "2025", or "2026 YTD" for the running year. */
export function yearLabel(row) {
  return row.is_partial ? t('{year} YTD', { year: row.year }) : String(row.year);
}

/** The size every yearly chart shares: a slot per year is at least this wide, and the plot is never lower than this. */
export const CHART_SIZE = { minPlotHeight: 260, minSlotWidth: 64 };

/** The space below the chart in the page: the padding of the chart box, the margin of its panel and the padding of the content area. */
function spaceBelow(inner) {
  const px = (element, property) => (element ? Number.parseFloat(getComputedStyle(element)[property]) || 0 : 0);
  return (
    px(inner.closest('.chart-scroll'), 'paddingBottom') +
    px(inner.closest('.panel'), 'marginBottom') +
    px(inner.closest('.content'), 'paddingBottom')
  );
}

/**
 * The plot height that lets the chart end at the bottom of the window with the page
 * scrolled to the top, and never less than `CHART_SIZE.minPlotHeight` (CSD-010 REQ-13).
 */
function plotHeightFor(inner, margin) {
  const scroller = inner.closest('.content');
  const top = inner.getBoundingClientRect().top + (scroller ? scroller.scrollTop : 0);
  // One pixel less, so rounding never pushes the page into a scrollbar.
  const room = window.innerHeight - top - margin.top - margin.bottom - spaceBelow(inner) - 1;
  return Math.max(CHART_SIZE.minPlotHeight, Math.floor(room));
}

/**
 * The marker of a year as received (`{ value, label }`): a short line across the bar, wider than
 * the bar, at its value, with its label above it (CSD-025 REQ-7).
 */
export function levelMarker(marker, { centre, barWidth, yOf }) {
  const y = yOf(marker.value);
  const half = barWidth * 0.7;
  return [
    svg('line', { class: 'chart-marker-hit', 'data-legend': 'marker', x1: centre - half, x2: centre + half, y1: y, y2: y }),
    svg('line', { class: 'chart-marker', 'data-legend': 'marker', x1: centre - half, x2: centre + half, y1: y, y2: y }),
    svg('text', { class: 'chart-marker-label', x: centre, y: y - 7 }, formatAmountLabel(marker.label)),
  ];
}

/**
 * Builds a yearly chart inside the page whose root element has the id `id`.
 * Its parts are found as `${id}-legend`, `${id}-empty`, `${id}-scroll` and
 * `${id}-chart`. The strategy:
 *
 * - `command`, `ariaLabel`: the Rust command that returns `{ axis, years }`, and the chart's name
 * - the size is the same for every chart: slots as wide as `CHART_SIZE` says, and a plot as high as the window allows
 * - `pieceStyle(row, piece)`: what the page adds to a received piece `{ series, low, high, hangs, rounding }`:
 *   `{ cssClass, label? }`, the colour class and the accessible name. The pieces, their order and their
 *   rounding are Rust's; the page decides only how a series looks
 * - `describe(row)`, `tooltipContent(row)`: the screen-reader text and the tooltip rows
 * - `decorate(row, geometry)`: extra nodes for a bar (growth label, marker); `geometry.barTop` is the
 *   y position of the top of the bar
 * - `labelLines(row)`: the lines under the axis, `[{ text, className }]`
 * - `onLoaded(data)` (optional): runs with the whole response before drawing, for a legend built from it
 */
export function createYearlyChart(strategy) {
  const part = (suffix) => document.getElementById(suffix ? `${strategy.id}-${suffix}` : strategy.id);
  let drawnData = null;

  function draw(data) {
    drawnData = data;
    const { axis, years } = data;
    const inner = part('chart');
    const lines = strategy.labelLines(years[0]).length;
    const margin = { ...MARGIN, bottom: 30 + 16 * (lines - 1) };
    const count = years.length;
    // Draw at the real pixel width, so bars, lines and text are never scaled up. The previous
    // drawing is removed first: its minimum width would otherwise keep the container wide and
    // the chart could never shrink with the window.
    inner.replaceChildren();
    inner.style.minWidth = '';
    const available = inner.clientWidth - margin.left - margin.right;
    const plotWidth = Math.max(count * CHART_SIZE.minSlotWidth, MIN_PLOT_WIDTH, available);
    const slot = plotWidth / count;
    const barWidth = Math.min(slot * 0.5, MAX_BAR_WIDTH);
    const width = margin.left + plotWidth + margin.right;
    const plotHeight = plotHeightFor(inner, margin);
    const height = margin.top + plotHeight + margin.bottom;
    const bottomEdge = margin.top + plotHeight;

    // The axis is Rust's: its first and last tick are the ends, so a value only becomes a position.
    const first = toPosition(axis.ticks.at(0)?.value ?? '0.00');
    const last = toPosition(axis.ticks.at(-1)?.value ?? '1.00');
    const span = last - first || 1;
    const yOf = (value) => margin.top + ((last - toPosition(value)) / span) * plotHeight;

    const chart = svg('svg', { viewBox: `0 0 ${width} ${height}`, role: 'group', 'aria-label': t(strategy.ariaLabel) });
    // The container is as wide as the chart, so the tooltip is placed against the whole chart.
    inner.style.minWidth = `${width}px`;
    chart.style.minWidth = `${width}px`;

    for (const tick of axis.ticks) {
      const y = yOf(tick.value);
      const lineClass = toPosition(tick.value) === 0 ? 'chart-zero' : 'chart-grid';
      chart.append(
        svg('line', { class: lineClass, x1: margin.left, x2: margin.left + plotWidth, y1: y, y2: y }),
        svg('text', { class: 'chart-axis-label', x: margin.left - 8, y: y + 4 }, formatAmountLabel(tick.label)),
      );
    }

    const tooltip = createTooltip(inner);
    years.forEach((row, index) => {
      const centre = margin.left + slot * index + slot / 2;
      const group = svg('g', {
        class: `chart-year-group${row.is_partial ? ' partial' : ''}`,
        tabindex: '0',
        role: 'img',
        'aria-label': strategy.describe(row),
      });
      const hit = svg('rect', { class: 'chart-hit', x: centre - slot / 2, y: margin.top, width: slot, height: plotHeight + margin.bottom });
      group.append(hit);

      // Pieces are drawn where Rust says; the box over the bar spans their pixel positions.
      const yearPieces = row.pieces;
      for (const piece of yearPieces) {
        const { cssClass, label } = strategy.pieceStyle(row, piece);
        group.append(
          segmentShape(
            { class: cssClass, 'data-legend': piece.series, ...(label ? { 'aria-label': label } : {}) },
            centre - barWidth / 2,
            barWidth,
            yOf(piece.high),
            yOf(piece.low),
            { rounded: piece.rounding === 'none' ? false : piece.rounding },
          ),
        );
      }
      const edges = yearPieces.flatMap((piece) => [yOf(piece.high), yOf(piece.low)]);
      const barTop = edges.length > 0 ? Math.min(...edges) : yOf('0.00');

      // An invisible box over the whole bar, so the tooltip can sit beside it.
      const bar = yearPieces.length > 0
        ? svg('rect', { class: 'chart-bar-box', x: centre - barWidth / 2, y: barTop, width: barWidth, height: Math.max(...edges) - barTop })
        : null;
      if (bar) group.append(bar);

      group.append(...strategy.decorate(row, { centre, barWidth, yOf, barTop }));
      strategy.labelLines(row).forEach((line, i) =>
        group.append(svg('text', { class: line.className, x: centre, y: bottomEdge + 18 + 16 * i }, line.text)),
      );

      group.addEventListener('mouseenter', () => tooltip.show(strategy.tooltipContent(row), bar, hit));
      group.addEventListener('mouseleave', () => tooltip.hide());
      group.addEventListener('focus', () => tooltip.show(strategy.tooltipContent(row), bar, hit));
      group.addEventListener('blur', () => tooltip.hide());
      // A highlighted series is highlighted in every year, not only the hovered one.
      bindHighlight(group, [part('legend'), inner]);
      chart.append(group);
    });

    inner.replaceChildren(chart, tooltip.element);
  }

  /** Loads the series from Rust and draws it, or shows the note when there is nothing yet. */
  async function reload() {
    const data = await invoke(strategy.command);
    const { years } = data;
    strategy.onLoaded?.(data);
    part('empty').hidden = years.length > 0;
    part('scroll').hidden = years.length === 0;
    if (years.length === 0) {
      part('chart').replaceChildren();
      return;
    }
    draw(data);
  }

  /** Reloads only while the chart's page is showing, so a hidden chart is not drawn at zero width. */
  async function reloadIfOpen() {
    if (!part().hidden) await reload();
  }

  /** The size the chart was last drawn for, so a redraw happens only when it changed. */
  let drawnFor = null;

  const redrawIfSizeChanged = () => {
    const scroll = part('scroll');
    // Hidden (another pane or tab, or no data): nothing can be measured; draw when it shows.
    if (!drawnData || scroll.hidden || scroll.offsetParent === null) {
      drawnFor = null;
      return;
    }
    const size = `${scroll.clientWidth}x${window.innerHeight}x${Math.round(part('chart').getBoundingClientRect().top)}`;
    if (size === drawnFor) return;
    draw(drawnData);
    drawnFor = `${scroll.clientWidth}x${window.innerHeight}x${Math.round(part('chart').getBoundingClientRect().top)}`;
  };

  /**
   * Redraws when the window is resized and when the chart becomes visible or changes width: a chart
   * loaded while its pane or tab was hidden was measured at zero, and switching back must fix it.
   */
  function init() {
    window.addEventListener('resize', redrawIfSizeChanged);
    if (typeof ResizeObserver === 'function') {
      let frame = 0;
      new ResizeObserver(() => {
        cancelAnimationFrame(frame);
        frame = requestAnimationFrame(redrawIfSizeChanged);
      }).observe(part('scroll'));
    }
  }

  return { reload, reloadIfOpen, init };
}
