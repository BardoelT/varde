// Dutch texts of undo and redo (CSD-027): the message after an undo or redo, the names of the
// steps (the English names that Rust sends, such as "Delete Bank Account") and two error codes.
const RECORDS = {
  Job: 'Baan',
  Paycheck: 'Salarisstrook',
  Statement: 'Overzicht',
  'Investment Account': 'Beleggingsrekening',
  'Investment Statement': 'Beleggingsoverzicht',
  'Bank Account': 'Bankrekening',
  'Bank Entry': 'Banksaldo',
  Debt: 'Schuld',
  'Debt Statement': 'Schuldoverzicht',
  House: 'Woning',
  'House Valuation': 'Taxatie',
  Expense: 'Uitgave',
};
const ACTIONS = { Add: 'toevoegen', Edit: 'bewerken', Delete: 'verwijderen' };

const steps = {};
for (const [action, verb] of Object.entries(ACTIONS)) {
  for (const [record, name] of Object.entries(RECORDS)) {
    steps[`${action} ${record}`] = `${name} ${verb}`;
  }
}

export default {
  'Undid: {step}': 'Ongedaan gemaakt: {step}',
  'Redid: {step}': 'Opnieuw uitgevoerd: {step}',
  'code:NOTHING_TO_UNDO': 'Er is niets om ongedaan te maken.',
  'code:NOTHING_TO_REDO': 'Er is niets om opnieuw uit te voeren.',
  ...steps,
};
