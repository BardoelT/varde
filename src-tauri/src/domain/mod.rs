//! Pure business logic: no Tauri, no IO.

pub mod bank_accounts;
pub mod charts;
pub mod common;
pub mod debts;
pub mod error;
pub mod expenses;
pub mod houses;
pub mod income;
pub mod investments;
pub mod money;
pub mod net_worth;
pub mod number_format;
pub mod preferences;
pub mod project_data;
pub mod ratios;
pub(crate) mod series;
pub mod settings;

pub use bank_accounts::{
    BankAccount, BankAccountId, BankAccounts, BankBalance, BankEntry, BankInsights, BankOverview,
    BankYearlyTotal, EntryPrefill,
};
pub use common::{apply_filter, Note, Period, StatementFilter, Year, YearMonth};
pub use debts::{
    Debt, DebtId, DebtInsights, DebtStatement, DebtStatementPrefill, DebtYearlyTotal, Debts,
};
pub use error::DomainError;
pub use expenses::{
    Expense, ExpenseId, ExpenseInsights, ExpenseList, ExpensePrefill, ExpenseYearlyTotal, Expenses,
};
pub use houses::{
    House, HouseId, HouseInsights, HouseValuation, HouseValuationPrefill, HouseYearlyValue, Houses,
};
pub use income::{
    Entry, Income, IncomeInsightsYear, IncomeOverview, IncomeYearlyTotal, Job, JobId, JobInput,
    PaycheckPrefill,
};
pub use investments::{
    Balance, CalculationInput, InvestmentAccount, InvestmentAccountId, InvestmentAccountInput,
    InvestmentInsightsYear, InvestmentStatement, InvestmentStatementPrefill, InvestmentYearlyTotal,
    Investments, InvestmentsOverview, NetDeposits, StatementCalculation,
};
pub use money::{Amounts, Currency};
pub use net_worth::NetWorthInsights;
pub use number_format::NumberFormat;
pub use preferences::{Language, Theme};
pub use project_data::ProjectData;
pub use ratios::Ratios;
pub use settings::Settings;
