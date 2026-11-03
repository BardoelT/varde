// The three steps every overview dialog repeats: open a form, save it, and delete a row after
// a confirmation. A page keeps what is its own (the fields, the commands, the texts) and passes
// it in as small option objects.
import { invoke, errorText } from './ipc.js';
import { $, confirmAction, showToast, showError, reloadAfterSave } from './dom.js';

/**
 * Opens a dialog with a fresh form: resets the form, hides its error box, sets the title, lets
 * `fill(form)` lock and fill the fields, shows the dialog and focuses the field `focus`.
 * All options but `fill` are element ids (`focus` is a field name of the form).
 */
export function openDialog({ dialog, form, error, title, titleText, fill = () => {}, focus }) {
  const formElement = $(form);
  formElement.reset();
  $(error).hidden = true;
  $(title).textContent = titleText;
  fill(formElement);
  $(dialog).showModal();
  formElement.elements[focus].focus();
}

/**
 * Saves a form: runs `save()` (the invoke of the add or the update command), then closes the
 * dialog and reloads through `changed`. `save` may return `false` to stop quietly and keep the
 * dialog open (the user went Back in a follow-up question). A failing `save` shows its safe
 * message in the dialog's error box; a failing reload goes to the toast (`reloadAfterSave`).
 */
export async function saveDialog({ dialog, error, save, changed }) {
  try {
    if ((await save()) === false) return;
    $(dialog).close();
    await reloadAfterSave(changed);
  } catch (caught) {
    showError(error, caught);
  }
}

/**
 * Asks for a confirmation, then runs the delete `command` with `args` and reloads through
 * `changed`. Declining does nothing; a failure goes to the toast.
 */
export async function deleteWithConfirm({ title, message, confirmLabel, command, args, changed }) {
  const confirmed = await confirmAction({ title, message, confirmLabel, danger: true });
  if (!confirmed) return;
  try {
    await invoke(command, args);
    await changed();
  } catch (error) {
    showToast(errorText(error));
  }
}
