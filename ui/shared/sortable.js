// Drag-and-drop ordering of the rows of a list. The user grabs a row by its handle (pointer events, so it
// needs no HTML5 drag-and-drop and works the same in the Tauri webview) and the row follows the pointer
// through the list. The handle also takes the arrow keys, so ordering never needs a pointer. The list
// itself is not the order: the caller sends the move to Rust and draws the list again.

/**
 * Makes the rows (`li`) of `list` sortable by the elements that match `handle`.
 * `onMove(from, to)` is called with the old and the new position, counted from 0, when a row has been
 * dropped somewhere else or moved by a key. `onCancel()` is called when a drag ends where it started or is
 * cancelled, so the caller can draw the list as it was.
 */
export function makeSortable(list, { handle, onMove, onCancel = () => {} }) {
  const rows = () => [...list.children];

  /** Where the dragged row belongs for a pointer at height `y`: after every other row that lies above it. */
  function positionAt(dragged, y) {
    let position = 0;
    for (const row of rows()) {
      if (row === dragged) continue;
      const box = row.getBoundingClientRect();
      if (y > box.top + box.height / 2) position += 1;
    }
    return position;
  }

  /** Puts the dragged row at `position` among the others, so the list shows where it would land. */
  function place(dragged, position) {
    const others = rows().filter((row) => row !== dragged);
    const before = others[position] ?? null;
    if (dragged.nextElementSibling !== before) list.insertBefore(dragged, before);
  }

  list.addEventListener('pointerdown', (event) => {
    const grip = event.target.closest?.(handle);
    if (!grip || !list.contains(grip) || (event.button !== undefined && event.button !== 0)) return;
    const dragged = grip.closest('li');
    if (!dragged) return;
    event.preventDefault();
    const from = rows().indexOf(dragged);
    let to = from;
    dragged.classList.add('dragging');
    list.classList.add('sorting');
    grip.setPointerCapture?.(event.pointerId);

    const finish = (apply) => {
      document.removeEventListener('pointermove', move);
      document.removeEventListener('pointerup', up);
      document.removeEventListener('pointercancel', cancel);
      document.removeEventListener('keydown', key, true);
      dragged.classList.remove('dragging');
      list.classList.remove('sorting');
      if (apply && to !== from) onMove(from, to);
      else {
        place(dragged, from);
        onCancel();
      }
    };
    const move = (moved) => {
      to = positionAt(dragged, moved.clientY);
      place(dragged, to);
    };
    const up = () => finish(true);
    const cancel = () => finish(false);
    const key = (pressed) => {
      if (pressed.key !== 'Escape') return;
      pressed.stopPropagation();
      finish(false);
    };
    document.addEventListener('pointermove', move);
    document.addEventListener('pointerup', up);
    document.addEventListener('pointercancel', cancel);
    document.addEventListener('keydown', key, true);
  });

  list.addEventListener('keydown', (event) => {
    const grip = event.target.closest?.(handle);
    if (!grip || (event.key !== 'ArrowUp' && event.key !== 'ArrowDown')) return;
    const row = grip.closest('li');
    const all = rows();
    const from = all.indexOf(row);
    const to = from + (event.key === 'ArrowUp' ? -1 : 1);
    event.preventDefault();
    if (to >= 0 && to < all.length) onMove(from, to);
  });
}
