// The page header (CSD-024 REQ-10 to REQ-16): the title and the tabs of a page stay at the top of
// the content area. The position is CSS sticky; the only script sets one custom property on the
// content area, the progress of the collapse from 0 (expanded) to 1 (compressed), as a plain
// function of the scroll position. The sizes in between are interpolated in the style sheet.

/** The custom property that holds the progress of the collapse. */
const PROGRESS = '--header-progress';

/** The page is scrolled this far, in pixels, before the header is fully compressed (REQ-11). */
export const COLLAPSE_RANGE = 80;

/** A page that overflows by less than the shrink plus this never collapses, so it cannot switch back and forth (REQ-12). */
const SHORT_PAGE_MARGIN = 32;

const content = () => document.querySelector('.content');

/** A length such as `77px` from a custom property of the content area, in pixels. */
function pixels(scroller, name) {
  return Number.parseFloat(getComputedStyle(scroller).getPropertyValue(name)) || 0;
}

/** The collapse as last set, from 0 to 1. */
export function headerProgress(scroller) {
  return Number.parseFloat(scroller.style.getPropertyValue(PROGRESS)) || 0;
}

/** How much taller the expanded header is than the compressed one, in pixels, when fully collapsed. */
function fullShrink(scroller) {
  return Math.max(0, pixels(scroller, '--header-expanded') - pixels(scroller, '--header-compressed'));
}

/** How much shorter the page header is right now than when expanded, in pixels (REQ-13). */
export function headerShrink(scroller) {
  return fullShrink(scroller) * headerProgress(scroller);
}

/**
 * The collapse that goes with the scroll position: none at the top, growing with the scroll distance,
 * full after `COLLAPSE_RANGE` pixels. It depends on nothing else, so there is no state to flicker.
 * A page that overflows by less than the shrink plus 32 px does not collapse at all.
 */
function progressFor(scroller) {
  if (scroller.scrollTop <= 0) return 0;
  const shrink = fullShrink(scroller);
  const expandedOverflow = scroller.scrollHeight - scroller.clientHeight + headerShrink(scroller);
  if (expandedOverflow < shrink + SHORT_PAGE_MARGIN) return 0;
  return Math.min(1, scroller.scrollTop / COLLAPSE_RANGE);
}

function sync(scroller) {
  const progress = Math.round(progressFor(scroller) * 1000) / 1000;
  if (progress !== headerProgress(scroller)) scroller.style.setProperty(PROGRESS, String(progress));
}

/** Shows the page at the top with the header expanded, as a page that was just chosen (REQ-12). */
export function resetPageHeader() {
  const scroller = content();
  if (!scroller) return;
  scroller.scrollTop = 0;
  scroller.style.setProperty(PROGRESS, '0');
}

/** Starts following the scroll position of the content area. */
export function initPageHeader() {
  const scroller = content();
  scroller.addEventListener('scroll', () => sync(scroller), { passive: true });
}
