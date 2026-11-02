// The Dutch texts (CSD-022): the English text of the app is the key, the Dutch text the value.
// A code of an error from Rust is written as `code:<CODE>`. One file per area, merged here.
import errors from './nl-errors.js';
import shared from './nl-shared.js';
import staticTexts from './nl-static.js';
import incomeExpenses from './nl-income-expenses.js';
import investmentsBank from './nl-investments-bank.js';
import debtsHouses from './nl-debts-houses.js';

export default {
  ...staticTexts,
  ...shared,
  ...incomeExpenses,
  ...investmentsBank,
  ...debtsHouses,
  ...errors,
};
