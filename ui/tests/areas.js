// The six pages whose first tab opens before Insights (CSD-035), with what the tests need to know of each.
export const TAB_PAGES = [
  { page: 'income', first: 'jobs', insights: 'get_income_insights' },
  { page: 'investments', first: 'accounts', insights: 'get_investment_insights' },
  { page: 'bank', first: 'accounts', insights: 'get_bank_insights' },
  { page: 'houses', first: 'houses', insights: 'get_house_insights' },
  { page: 'debts', first: 'debts', insights: 'get_debt_insights' },
  { page: 'expenses', first: 'totals', insights: 'get_expense_insights' },
];
