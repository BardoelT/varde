// What the parts of the Transactions page share (CSD-032): the categories and rules as Rust listed them, the
// way a change reloads the page, and the way an action reports an error. Each part (overview, bulk select,
// categories, rules) has its own file and gets these from here, so none of them imports another.
import { errorText } from '../shared/ipc.js';
import { showToast } from '../shared/dom.js';
import { t } from '../shared/i18n.js';
import { formatDate } from '../shared/format.js';

/** The categories (with their transaction counts) and the rules (with theirs), as the last reload got them. */
export const state = { categories: [], rules: [] };

/** The name of a category by id; no id is the built-in Uncategorized. */
export const categoryName = (id) => state.categories.find((category) => category.id === id)?.name ?? t('Uncategorized');

/** A name for a row in labels and messages: the counterparty, else the description. */
export const transactionLabel = (transaction) => transaction.counterparty_name || transaction.description || formatDate(transaction.date);

/** The result as a yellow notification bottom right, like the one after an undo (CSD-032 REQ-16). */
export const showResult = (text) => showToast(text, 'notice');

let reloadPage = async () => {};
let reloadOthers = async () => {};

/** `reload()` redraws this page from Rust; `onChanged()` reloads whatever else shows transactions data. */
export function connect({ reload, onChanged }) {
  reloadPage = reload;
  reloadOthers = onChanged;
}

/** After a change: reloads this page, then the rest of the app. */
export async function changed() {
  await reloadPage();
  await reloadOthers();
}

/**
 * Runs an action of the user. An error becomes a notification; with `reloadAfterError` the page is also
 * reloaded, so it shows what Rust holds after a refused change.
 */
export async function run(action, { reloadAfterError = false } = {}) {
  try {
    return await action();
  } catch (error) {
    showToast(errorText(error));
    if (reloadAfterError) await reloadPage().catch((reloadError) => showToast(errorText(reloadError)));
    return undefined;
  }
}
