// The page header (CSD-024 REQ-10 to REQ-16): the title and the tabs of a page stay at the top of
// the content area. The position is CSS sticky; the only script is one class on the content area,
// set from its scroll position, that makes the title smaller.

/** The class on the content area while the page is scrolled and the header is compressed. */
export const COMPRESSED_CLASS = 'header-compressed';

/** The page is scrolled down further than this before the header compresses (REQ-11). */
const COMPRESS_AFTER = 32;

const content = () => document.querySelector('.content');

/** A length such as `77px` from a custom property of the content area, in pixels. */
function pixels(scroller, name) {
  return Number.parseFloat(getComputedStyle(scroller).getPropertyValue(name)) || 0;
}

/** How much taller the expanded header is than the compressed one, in pixels (REQ-13). */
export function headerShrink(scroller) {
  return Math.max(0, pixels(scroller, '--header-expanded') - pixels(scroller, '--header-compressed'));
}

/**
 * Compresses at more than 32 px of scrolling, expands only at the very top, and keeps the state in between.
 * A page that overflows by less than the shrink plus 32 px never compresses, so it cannot switch back and forth (REQ-12).
 */
function sync(scroller) {
  const compressed = scroller.classList.contains(COMPRESSED_CLASS);
  if (scroller.scrollTop <= 0) {
    if (compressed) scroller.classList.remove(COMPRESSED_CLASS);
    return;
  }
  if (compressed || scroller.scrollTop <= COMPRESS_AFTER) return;
  const overflow = scroller.scrollHeight - scroller.clientHeight;
  if (overflow >= headerShrink(scroller) + COMPRESS_AFTER) scroller.classList.add(COMPRESSED_CLASS);
}

/** Shows the page at the top with the header expanded, as a page that was just chosen (REQ-12). */
export function resetPageHeader() {
  const scroller = content();
  if (!scroller) return;
  scroller.scrollTop = 0;
  scroller.classList.remove(COMPRESSED_CLASS);
}

/** Starts following the scroll position of the content area. */
export function initPageHeader() {
  const scroller = content();
  scroller.addEventListener('scroll', () => sync(scroller), { passive: true });
}
