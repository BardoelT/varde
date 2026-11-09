//! The errors of the domain: stable codes and safe messages.

use thiserror::Error;

/// Validation and lookup failures of the income (CSD-001), investments
/// (CSD-005), bank accounts (CSD-009), debts (CSD-011) and expenses
/// (CSD-013) domains.
///
/// Messages are safe to show to the user: they never contain user data.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    /// Reported as: "Enter an employer name."
    #[error("Enter an employer name.")]
    JobEmployerRequired,
    /// Reported as: "Enter a start date."
    #[error("Enter a start date.")]
    JobStartRequired,
    /// Reported as: "The end date cannot be before the start date."
    #[error("The end date cannot be before the start date.")]
    JobEndBeforeStart,
    /// Reported as: "Enter dates as dd/mm/yyyy."
    #[error("Enter dates as dd/mm/yyyy.")]
    JobDateInvalid,
    /// Reported as: "Employer and title can be at most 200 characters."
    #[error("Employer and title can be at most 200 characters.")]
    JobFieldTooLong,
    /// Reported as: "Choose EUR, USD or GBP."
    #[error("Choose EUR, USD or GBP.")]
    ProjectCurrencyUnsupported,
    /// Reported as: "Choose System, Light or Dark."
    #[error("Choose System, Light or Dark.")]
    PreferencesThemeUnsupported,
    /// Reported as: "Choose English or Dutch."
    #[error("Choose English or Dutch.")]
    PreferencesLanguageUnsupported,
    /// Reported as: "Choose 1,234.50, 1.234,50 or 1'234,50."
    #[error("Choose 1,234.50, 1.234,50 or 1'234,50.")]
    PreferencesNumberFormatUnsupported,
    /// Reported as: "This job does not exist."
    #[error("This job does not exist.")]
    JobNotFound,
    /// Reported as: "Choose Monthly or Yearly."
    #[error("Choose Monthly or Yearly.")]
    JobPeriodUnsupported,
    /// Reported as: "The pay period cannot change once the job has paychecks or annual statements."
    #[error("The pay period cannot change once the job has paychecks or annual statements.")]
    JobPeriodLocked,
    /// Reported as: "A monthly job has paychecks and a yearly job has annual statements."
    #[error("A monthly job has paychecks and a yearly job has annual statements.")]
    JobPeriodMismatch,
    /// Reported as: "Enter the year as YYYY."
    #[error("Enter the year as YYYY.")]
    StatementYearInvalid,
    /// Reported as: "This job already has an annual statement for that year."
    #[error("This job already has an annual statement for that year.")]
    StatementDuplicateYear,
    /// Reported as: "There is no annual statement for that year."
    #[error("There is no annual statement for that year.")]
    StatementNotFound,
    /// Reported as: "Enter the month as YYYY-MM."
    #[error("Enter the month as YYYY-MM.")]
    PaycheckMonthInvalid,
    /// Reported as: "This job already has a paycheck for that month."
    #[error("This job already has a paycheck for that month.")]
    PaycheckDuplicateMonth,
    /// Reported as: "There is no paycheck for that month."
    #[error("There is no paycheck for that month.")]
    PaycheckNotFound,
    /// Reported as: "Gross salary must be greater than 0.00."
    #[error("Gross salary must be greater than 0.00.")]
    PaycheckGrossInvalid,
    /// Reported as: "Enter amounts as numbers such as 3000.00."
    #[error("Enter amounts as numbers such as 3000.00.")]
    PaycheckAmountInvalid,
    /// Reported as: "Amounts cannot be negative."
    #[error("Amounts cannot be negative.")]
    PaycheckAmountNegative,
    /// Reported as: "Amounts can be at most 1000000.00."
    #[error("Amounts can be at most 1000000.00.")]
    PaycheckAmountTooLarge,
    /// Reported as: "The income data is not consistent."
    #[error("The income data is not consistent.")]
    DataInvalid,
    /// Reported as: "Enter an account name."
    #[error("Enter an account name.")]
    InvestmentAccountNameRequired,
    /// Reported as: "The account name can be at most 100 characters."
    #[error("The account name can be at most 100 characters.")]
    InvestmentAccountNameTooLong,
    /// Reported as: "An account with this name already exists."
    #[error("An account with this name already exists.")]
    InvestmentAccountNameDuplicate,
    /// Reported as: "This investment account does not exist."
    #[error("This investment account does not exist.")]
    InvestmentAccountNotFound,
    /// Reported as: "Choose Monthly or Yearly."
    #[error("Choose Monthly or Yearly.")]
    InvestmentPeriodUnsupported,
    /// Reported as: "The statement period cannot change once the account has statements."
    #[error("The statement period cannot change once the account has statements.")]
    InvestmentPeriodLocked,
    #[error(
        "A monthly account has monthly statements and a yearly account has yearly statements."
    )]
    /// Reported as: "A monthly account has monthly statements and a yearly account has yearly statements."
    InvestmentPeriodMismatch,
    /// Reported as: "Enter the month as YYYY-MM or the year as YYYY."
    #[error("Enter the month as YYYY-MM or the year as YYYY.")]
    InvestmentStatementPeriodInvalid,
    /// Reported as: "This account already has a statement for that period."
    #[error("This account already has a statement for that period.")]
    InvestmentStatementDuplicate,
    /// Reported as: "There is no statement for that period."
    #[error("There is no statement for that period.")]
    InvestmentStatementNotFound,
    /// Reported as: "Enter the balance as a number such as 12500.00."
    #[error("Enter the balance as a number such as 12500.00.")]
    InvestmentBalanceInvalid,
    /// Reported as: "The balance cannot be negative."
    #[error("The balance cannot be negative.")]
    InvestmentBalanceNegative,
    /// Reported as: "The balance can be at most 1000000000.00."
    #[error("The balance can be at most 1000000000.00.")]
    InvestmentBalanceTooLarge,
    /// Reported as: "A note can be at most 2000 characters."
    #[error("A note can be at most 2000 characters.")]
    InvestmentNoteTooLong,
    /// Reported as: "Enter the net deposits as a number such as 300.00 or -250.00."
    #[error("Enter the net deposits as a number such as 300.00 or -250.00.")]
    InvestmentNetDepositsInvalid,
    /// Reported as: "Net deposits can be at most 1000000000.00, or -1000000000.00."
    #[error("Net deposits can be at most 1000000000.00, or -1000000000.00.")]
    InvestmentNetDepositsTooLarge,
    /// Reported as: "Enter the capital growth as a number such as 200.00 or -150.00."
    #[error("Enter the capital growth as a number such as 200.00 or -150.00.")]
    InvestmentGrowthInvalid,
    /// Reported as: "Capital growth can be at most 1000000000.00, or -1000000000.00."
    #[error("Capital growth can be at most 1000000000.00, or -1000000000.00.")]
    InvestmentGrowthTooLarge,
    /// Reported as: "The investment data is not consistent."
    #[error("The investment data is not consistent.")]
    InvestmentDataInvalid,
    /// Reported as: "Enter an account name."
    #[error("Enter an account name.")]
    BankAccountNameRequired,
    /// Reported as: "The account name can be at most 100 characters."
    #[error("The account name can be at most 100 characters.")]
    BankAccountNameTooLong,
    /// Reported as: "A bank account with this name already exists."
    #[error("A bank account with this name already exists.")]
    BankAccountNameDuplicate,
    /// Reported as: "This bank account does not exist."
    #[error("This bank account does not exist.")]
    BankAccountNotFound,
    /// Reported as: "Choose Monthly or Yearly."
    #[error("Choose Monthly or Yearly.")]
    BankAccountPeriodUnsupported,
    /// Reported as: "The entry period cannot change once the account has entries."
    #[error("The entry period cannot change once the account has entries.")]
    BankAccountPeriodLocked,
    /// Reported as: "A monthly account has monthly entries and a yearly account has yearly entries."
    #[error("A monthly account has monthly entries and a yearly account has yearly entries.")]
    BankEntryPeriodMismatch,
    /// Reported as: "Enter the month as YYYY-MM or the year as YYYY."
    #[error("Enter the month as YYYY-MM or the year as YYYY.")]
    BankEntryPeriodInvalid,
    /// Reported as: "This account already has an entry for that period."
    #[error("This account already has an entry for that period.")]
    BankEntryDuplicate,
    /// Reported as: "There is no entry for that period."
    #[error("There is no entry for that period.")]
    BankEntryNotFound,
    /// Reported as: "Enter the balance as a number such as 2500.00 or -150.25."
    #[error("Enter the balance as a number such as 2500.00 or -150.25.")]
    BankEntryBalanceInvalid,
    /// Reported as: "The balance can be at most 1000000000.00, or -1000000000.00."
    #[error("The balance can be at most 1000000000.00, or -1000000000.00.")]
    BankEntryBalanceTooLarge,
    /// Reported as: "A note can be at most 2000 characters."
    #[error("A note can be at most 2000 characters.")]
    BankEntryNoteTooLong,
    /// Reported as: "The bank account data is not consistent."
    #[error("The bank account data is not consistent.")]
    BankDataInvalid,
    /// Reported as: "Enter a debt name."
    #[error("Enter a debt name.")]
    DebtNameRequired,
    /// Reported as: "The debt name can be at most 100 characters."
    #[error("The debt name can be at most 100 characters.")]
    DebtNameTooLong,
    /// Reported as: "A debt with this name already exists."
    #[error("A debt with this name already exists.")]
    DebtNameDuplicate,
    /// Reported as: "This debt does not exist."
    #[error("This debt does not exist.")]
    DebtNotFound,
    /// Reported as: "Choose Monthly or Yearly."
    #[error("Choose Monthly or Yearly.")]
    DebtPeriodUnsupported,
    /// Reported as: "The statement period cannot change once the debt has statements."
    #[error("The statement period cannot change once the debt has statements.")]
    DebtPeriodLocked,
    /// Reported as: "A monthly debt has monthly statements and a yearly debt has yearly statements."
    #[error("A monthly debt has monthly statements and a yearly debt has yearly statements.")]
    DebtPeriodMismatch,
    /// Reported as: "Enter the month as YYYY-MM or the year as YYYY."
    #[error("Enter the month as YYYY-MM or the year as YYYY.")]
    DebtStatementPeriodInvalid,
    /// Reported as: "This debt already has a statement for that period."
    #[error("This debt already has a statement for that period.")]
    DebtStatementDuplicate,
    /// Reported as: "There is no statement for that period."
    #[error("There is no statement for that period.")]
    DebtStatementNotFound,
    /// Reported as: "Enter the balance as a number such as 150000.00."
    #[error("Enter the balance as a number such as 150000.00.")]
    DebtStatementBalanceInvalid,
    /// Reported as: "The balance cannot be negative."
    #[error("The balance cannot be negative.")]
    DebtStatementBalanceNegative,
    /// Reported as: "The balance can be at most 1000000000.00."
    #[error("The balance can be at most 1000000000.00.")]
    DebtStatementBalanceTooLarge,
    /// Reported as: "A note can be at most 2000 characters."
    #[error("A note can be at most 2000 characters.")]
    DebtStatementNoteTooLong,
    /// Reported as: "The debt data is not consistent."
    #[error("The debt data is not consistent.")]
    DebtDataInvalid,
    /// Reported as: "A linked debt has a statement after the month or year of the sale."
    #[error("A linked debt has a statement after the month or year of the sale.")]
    DebtStatementAfterSale,
    /// Reported as: "This debt is closed, so it can no longer change from that period on."
    #[error("This debt is closed, so it can no longer change from that period on.")]
    DebtSettledLocked,
    /// Reported as: "Choose Mortgage, Loan or Other as the category."
    #[error("Choose Mortgage, Loan or Other as the category.")]
    DebtCategoryInvalid,
    /// Reported as: "Only a mortgage or a loan can be linked to a house."
    #[error("Only a mortgage or a loan can be linked to a house.")]
    DebtLinkCategoryInvalid,
    /// Reported as: "Enter the date the debt started, as dd/mm/yyyy."
    #[error("Enter the date the debt started, as dd/mm/yyyy.")]
    DebtOpeningDateRequired,
    /// Reported as: "Enter the opening date as dd/mm/yyyy, for example 15/03/2020."
    #[error("Enter the opening date as dd/mm/yyyy, for example 15/03/2020.")]
    DebtOpeningDateInvalid,
    /// Reported as: "Enter the amount the debt started with."
    #[error("Enter the amount the debt started with.")]
    DebtOpeningAmountRequired,
    /// Reported as: "Enter the opening amount as a number such as 150000.00."
    #[error("Enter the opening amount as a number such as 150000.00.")]
    DebtOpeningAmountInvalid,
    /// Reported as: "The opening amount cannot be negative."
    #[error("The opening amount cannot be negative.")]
    DebtOpeningAmountNegative,
    /// Reported as: "The opening amount can be at most 1000000000.00."
    #[error("The opening amount can be at most 1000000000.00.")]
    DebtOpeningAmountTooLarge,
    /// Reported as: "Enter both the closing date and the closing amount, or neither."
    #[error("Enter both the closing date and the closing amount, or neither.")]
    DebtClosingIncomplete,
    /// Reported as: "The closing date cannot be before the opening date."
    #[error("The closing date cannot be before the opening date.")]
    DebtClosingBeforeOpening,
    /// Reported as: "Enter the closing date as dd/mm/yyyy, for example 10/06/2026."
    #[error("Enter the closing date as dd/mm/yyyy, for example 10/06/2026.")]
    DebtClosingDateInvalid,
    /// Reported as: "Choose Repaid or Cancelled for the closing."
    #[error("Choose Repaid or Cancelled for the closing.")]
    DebtClosingOutcomeRequired,
    /// Reported as: "Enter the closing amount as a number such as 0.00."
    #[error("Enter the closing amount as a number such as 0.00.")]
    DebtClosingAmountInvalid,
    /// Reported as: "The closing amount cannot be negative."
    #[error("The closing amount cannot be negative.")]
    DebtClosingAmountNegative,
    /// Reported as: "The closing amount can be at most 1000000000.00."
    #[error("The closing amount can be at most 1000000000.00.")]
    DebtClosingAmountTooLarge,
    /// Reported as: "A statement must be between the opening and the closing of the debt."
    #[error("A statement must be between the opening and the closing of the debt.")]
    DebtStatementOutsideWindow,
    /// Reported as: "These dates leave existing statements outside the opening and closing of the debt."
    #[error("These dates leave existing statements outside the opening and closing of the debt.")]
    DebtDatesExcludeStatements,
    /// Reported as: "Confirm that clearing the closing also removes its settlement statement."
    #[error("Confirm that clearing the closing also removes its settlement statement.")]
    DebtClosingUndoUnconfirmed,
    /// Reported as: "Choose Repaid or Cancelled for every linked debt with an open balance."
    #[error("Choose Repaid or Cancelled for every linked debt with an open balance.")]
    HouseSettlementRequired,
    /// Reported as: "That choice does not belong to a linked debt with an open balance."
    #[error("That choice does not belong to a linked debt with an open balance.")]
    HouseSettlementInvalid,
    /// Reported as: "Confirm that undoing the sale also undoes the settled debts."
    #[error("Confirm that undoing the sale also undoes the settled debts.")]
    HouseSaleUndoUnconfirmed,
    /// Reported as: "The sell date cannot change while debts are settled on it. Undo the sale first."
    #[error("The sell date cannot change while debts are settled on it. Undo the sale first.")]
    HouseSellDateLocked,
    /// Reported as: "Remove the linked debts from this house before deleting it."
    #[error("Remove the linked debts from this house before deleting it.")]
    HouseHasLinkedDebts,
    /// Reported as: "Enter a house name."
    #[error("Enter a house name.")]
    HouseNameRequired,
    /// Reported as: "The house name can be at most 100 characters."
    #[error("The house name can be at most 100 characters.")]
    HouseNameTooLong,
    /// Reported as: "A house with this name already exists."
    #[error("A house with this name already exists.")]
    HouseNameDuplicate,
    /// Reported as: "This house does not exist."
    #[error("This house does not exist.")]
    HouseNotFound,
    /// Reported as: "Choose Monthly or Yearly."
    #[error("Choose Monthly or Yearly.")]
    HousePeriodUnsupported,
    /// Reported as: "The valuation period cannot change once the house has valuations."
    #[error("The valuation period cannot change once the house has valuations.")]
    HousePeriodLocked,
    /// Reported as: "A monthly house has monthly valuations and a yearly house has yearly valuations."
    #[error("A monthly house has monthly valuations and a yearly house has yearly valuations.")]
    HousePeriodMismatch,
    /// Reported as: "Enter the month as YYYY-MM or the year as YYYY."
    #[error("Enter the month as YYYY-MM or the year as YYYY.")]
    HouseValuationPeriodInvalid,
    /// Reported as: "This house already has a valuation for that period."
    #[error("This house already has a valuation for that period.")]
    HouseValuationDuplicate,
    /// Reported as: "There is no valuation for that period."
    #[error("There is no valuation for that period.")]
    HouseValuationNotFound,
    /// Reported as: "Enter the value as a number such as 300000.00."
    #[error("Enter the value as a number such as 300000.00.")]
    HouseValuationValueInvalid,
    /// Reported as: "The value cannot be negative."
    #[error("The value cannot be negative.")]
    HouseValuationValueNegative,
    /// Reported as: "The value can be at most 1000000000.00."
    #[error("The value can be at most 1000000000.00.")]
    HouseValuationValueTooLarge,
    /// Reported as: "A note can be at most 2000 characters."
    #[error("A note can be at most 2000 characters.")]
    HouseValuationNoteTooLong,
    /// Reported as: "The house data is not consistent."
    #[error("The house data is not consistent.")]
    HouseDataInvalid,
    /// Reported as: "Enter the purchase date."
    #[error("Enter the purchase date.")]
    HousePurchaseDateRequired,
    /// Reported as: "Enter the purchase amount."
    #[error("Enter the purchase amount.")]
    HousePurchaseAmountRequired,
    /// Reported as: "Enter the date as dd/mm/yyyy, from 01/01/0001."
    #[error("Enter the date as dd/mm/yyyy, from 01/01/0001.")]
    HouseDateInvalid,
    /// Reported as: "The sell date cannot be before the purchase date."
    #[error("The sell date cannot be before the purchase date.")]
    HouseSellBeforePurchase,
    /// Reported as: "Enter both the sell date and the sell amount, or neither."
    #[error("Enter both the sell date and the sell amount, or neither.")]
    HouseSaleIncomplete,
    /// Reported as: "Enter the amount as a number such as 250000.00."
    #[error("Enter the amount as a number such as 250000.00.")]
    HouseAmountInvalid,
    /// Reported as: "The amount cannot be negative."
    #[error("The amount cannot be negative.")]
    HouseAmountNegative,
    /// Reported as: "The amount can be at most 1000000000.00."
    #[error("The amount can be at most 1000000000.00.")]
    HouseAmountTooLarge,
    /// Reported as: "A valuation must be between the purchase and the sale of the house."
    #[error("A valuation must be between the purchase and the sale of the house.")]
    HouseValuationOutsideHolding,
    /// Reported as: "These dates leave existing valuations outside the purchase and sale of the house."
    #[error("These dates leave existing valuations outside the purchase and sale of the house.")]
    HouseDatesExcludeValuations,
    /// Reported as: "Enter a name for the expense."
    #[error("Enter a name for the expense.")]
    ExpenseNameRequired,
    /// Reported as: "The expense name can be at most 100 characters."
    #[error("The expense name can be at most 100 characters.")]
    ExpenseNameTooLong,
    /// Reported as: "Choose Home, Vehicle, Holiday, Sport, Study, Appliances, Hobby or Other."
    #[error("Choose Home, Vehicle, Holiday, Sport, Study, Appliances, Hobby or Other.")]
    ExpenseCategoryUnsupported,
    /// Reported as: "Enter the date as dd/mm/yyyy, from 01/01/0001."
    #[error("Enter the date as dd/mm/yyyy, from 01/01/0001.")]
    ExpenseDateInvalid,
    /// Reported as: "Enter the amount as a number such as 4500.00."
    #[error("Enter the amount as a number such as 4500.00.")]
    ExpenseAmountInvalid,
    /// Reported as: "The amount must be greater than 0.00."
    #[error("The amount must be greater than 0.00.")]
    ExpenseAmountNotPositive,
    /// Reported as: "The amount can be at most 1000000000.00."
    #[error("The amount can be at most 1000000000.00.")]
    ExpenseAmountTooLarge,
    /// Reported as: "A note can be at most 2000 characters."
    #[error("A note can be at most 2000 characters.")]
    ExpenseNoteTooLong,
    /// Reported as: "This expense does not exist."
    #[error("This expense does not exist.")]
    ExpenseNotFound,
    /// Reported as: "The expense data is not consistent."
    #[error("The expense data is not consistent.")]
    ExpenseDataInvalid,
}

impl DomainError {
    /// Stable machine-readable code for the frontend.
    #[must_use]
    #[allow(
        clippy::too_many_lines,
        reason = "one flat match from error to code, one arm per error"
    )]
    pub fn code(&self) -> &'static str {
        match self {
            Self::JobEmployerRequired => "JOB_EMPLOYER_REQUIRED",
            Self::JobStartRequired => "JOB_START_REQUIRED",
            Self::JobEndBeforeStart => "JOB_END_BEFORE_START",
            Self::JobDateInvalid => "JOB_DATE_INVALID",
            Self::JobFieldTooLong => "JOB_FIELD_TOO_LONG",
            Self::ProjectCurrencyUnsupported => "PROJECT_CURRENCY_UNSUPPORTED",
            Self::PreferencesThemeUnsupported => "PREFERENCES_THEME_UNSUPPORTED",
            Self::PreferencesLanguageUnsupported => "PREFERENCES_LANGUAGE_UNSUPPORTED",
            Self::PreferencesNumberFormatUnsupported => "PREFERENCES_NUMBER_FORMAT_UNSUPPORTED",
            Self::JobNotFound => "JOB_NOT_FOUND",
            Self::JobPeriodUnsupported => "JOB_PERIOD_UNSUPPORTED",
            Self::JobPeriodLocked => "JOB_PERIOD_LOCKED",
            Self::JobPeriodMismatch => "JOB_PERIOD_MISMATCH",
            Self::StatementYearInvalid => "STATEMENT_YEAR_INVALID",
            Self::StatementDuplicateYear => "STATEMENT_DUPLICATE_YEAR",
            Self::StatementNotFound => "STATEMENT_NOT_FOUND",
            Self::PaycheckMonthInvalid => "PAYCHECK_MONTH_INVALID",
            Self::PaycheckDuplicateMonth => "PAYCHECK_DUPLICATE_MONTH",
            Self::PaycheckNotFound => "PAYCHECK_NOT_FOUND",
            Self::PaycheckGrossInvalid => "PAYCHECK_GROSS_INVALID",
            Self::PaycheckAmountInvalid => "PAYCHECK_AMOUNT_INVALID",
            Self::PaycheckAmountNegative => "PAYCHECK_AMOUNT_NEGATIVE",
            Self::PaycheckAmountTooLarge => "PAYCHECK_AMOUNT_TOO_LARGE",
            Self::DataInvalid => "INCOME_DATA_INVALID",
            Self::InvestmentAccountNameRequired => "INVESTMENT_ACCOUNT_NAME_REQUIRED",
            Self::InvestmentAccountNameTooLong => "INVESTMENT_ACCOUNT_NAME_TOO_LONG",
            Self::InvestmentAccountNameDuplicate => "INVESTMENT_ACCOUNT_NAME_DUPLICATE",
            Self::InvestmentAccountNotFound => "INVESTMENT_ACCOUNT_NOT_FOUND",
            Self::InvestmentPeriodUnsupported => "INVESTMENT_PERIOD_UNSUPPORTED",
            Self::InvestmentPeriodLocked => "INVESTMENT_PERIOD_LOCKED",
            Self::InvestmentPeriodMismatch => "INVESTMENT_PERIOD_MISMATCH",
            Self::InvestmentStatementPeriodInvalid => "INVESTMENT_STATEMENT_PERIOD_INVALID",
            Self::InvestmentStatementDuplicate => "INVESTMENT_STATEMENT_DUPLICATE",
            Self::InvestmentStatementNotFound => "INVESTMENT_STATEMENT_NOT_FOUND",
            Self::InvestmentBalanceInvalid => "INVESTMENT_BALANCE_INVALID",
            Self::InvestmentBalanceNegative => "INVESTMENT_BALANCE_NEGATIVE",
            Self::InvestmentBalanceTooLarge => "INVESTMENT_BALANCE_TOO_LARGE",
            Self::InvestmentNoteTooLong => "INVESTMENT_NOTE_TOO_LONG",
            Self::InvestmentNetDepositsInvalid => "INVESTMENT_NET_DEPOSITS_INVALID",
            Self::InvestmentNetDepositsTooLarge => "INVESTMENT_NET_DEPOSITS_TOO_LARGE",
            Self::InvestmentGrowthInvalid => "INVESTMENT_GROWTH_INVALID",
            Self::InvestmentGrowthTooLarge => "INVESTMENT_GROWTH_TOO_LARGE",
            Self::InvestmentDataInvalid => "INVESTMENT_DATA_INVALID",
            Self::BankAccountNameRequired => "BANK_ACCOUNT_NAME_REQUIRED",
            Self::BankAccountNameTooLong => "BANK_ACCOUNT_NAME_TOO_LONG",
            Self::BankAccountNameDuplicate => "BANK_ACCOUNT_NAME_DUPLICATE",
            Self::BankAccountNotFound => "BANK_ACCOUNT_NOT_FOUND",
            Self::BankAccountPeriodUnsupported => "BANK_ACCOUNT_PERIOD_UNSUPPORTED",
            Self::BankAccountPeriodLocked => "BANK_ACCOUNT_PERIOD_LOCKED",
            Self::BankEntryPeriodMismatch => "BANK_ENTRY_PERIOD_MISMATCH",
            Self::BankEntryPeriodInvalid => "BANK_ENTRY_PERIOD_INVALID",
            Self::BankEntryDuplicate => "BANK_ENTRY_DUPLICATE",
            Self::BankEntryNotFound => "BANK_ENTRY_NOT_FOUND",
            Self::BankEntryBalanceInvalid => "BANK_ENTRY_BALANCE_INVALID",
            Self::BankEntryBalanceTooLarge => "BANK_ENTRY_BALANCE_TOO_LARGE",
            Self::BankEntryNoteTooLong => "BANK_ENTRY_NOTE_TOO_LONG",
            Self::BankDataInvalid => "BANK_DATA_INVALID",
            Self::DebtNameRequired => "DEBT_NAME_REQUIRED",
            Self::DebtNameTooLong => "DEBT_NAME_TOO_LONG",
            Self::DebtNameDuplicate => "DEBT_NAME_DUPLICATE",
            Self::DebtNotFound => "DEBT_NOT_FOUND",
            Self::DebtPeriodUnsupported => "DEBT_PERIOD_UNSUPPORTED",
            Self::DebtPeriodLocked => "DEBT_PERIOD_LOCKED",
            Self::DebtPeriodMismatch => "DEBT_PERIOD_MISMATCH",
            Self::DebtStatementPeriodInvalid => "DEBT_STATEMENT_PERIOD_INVALID",
            Self::DebtStatementDuplicate => "DEBT_STATEMENT_DUPLICATE",
            Self::DebtStatementNotFound => "DEBT_STATEMENT_NOT_FOUND",
            Self::DebtStatementBalanceInvalid => "DEBT_STATEMENT_BALANCE_INVALID",
            Self::DebtStatementBalanceNegative => "DEBT_STATEMENT_BALANCE_NEGATIVE",
            Self::DebtStatementBalanceTooLarge => "DEBT_STATEMENT_BALANCE_TOO_LARGE",
            Self::DebtStatementNoteTooLong => "DEBT_STATEMENT_NOTE_TOO_LONG",
            Self::DebtDataInvalid => "DEBT_DATA_INVALID",
            Self::DebtStatementAfterSale => "DEBT_STATEMENT_AFTER_SALE",
            Self::DebtSettledLocked => "DEBT_SETTLED_LOCKED",
            Self::DebtCategoryInvalid => "DEBT_CATEGORY_INVALID",
            Self::DebtLinkCategoryInvalid => "DEBT_LINK_CATEGORY_INVALID",
            Self::DebtOpeningDateRequired => "DEBT_OPENING_DATE_REQUIRED",
            Self::DebtOpeningDateInvalid => "DEBT_OPENING_DATE_INVALID",
            Self::DebtOpeningAmountRequired => "DEBT_OPENING_AMOUNT_REQUIRED",
            Self::DebtOpeningAmountInvalid => "DEBT_OPENING_AMOUNT_INVALID",
            Self::DebtOpeningAmountNegative => "DEBT_OPENING_AMOUNT_NEGATIVE",
            Self::DebtOpeningAmountTooLarge => "DEBT_OPENING_AMOUNT_TOO_LARGE",
            Self::DebtClosingIncomplete => "DEBT_CLOSING_INCOMPLETE",
            Self::DebtClosingBeforeOpening => "DEBT_CLOSING_BEFORE_OPENING",
            Self::DebtClosingDateInvalid => "DEBT_CLOSING_DATE_INVALID",
            Self::DebtClosingOutcomeRequired => "DEBT_CLOSING_OUTCOME_REQUIRED",
            Self::DebtClosingAmountInvalid => "DEBT_CLOSING_AMOUNT_INVALID",
            Self::DebtClosingAmountNegative => "DEBT_CLOSING_AMOUNT_NEGATIVE",
            Self::DebtClosingAmountTooLarge => "DEBT_CLOSING_AMOUNT_TOO_LARGE",
            Self::DebtStatementOutsideWindow => "DEBT_STATEMENT_OUTSIDE_WINDOW",
            Self::DebtDatesExcludeStatements => "DEBT_DATES_EXCLUDE_STATEMENTS",
            Self::DebtClosingUndoUnconfirmed => "DEBT_CLOSING_UNDO_UNCONFIRMED",
            Self::HouseSettlementRequired => "HOUSE_SETTLEMENT_REQUIRED",
            Self::HouseSettlementInvalid => "HOUSE_SETTLEMENT_INVALID",
            Self::HouseSaleUndoUnconfirmed => "HOUSE_SALE_UNDO_UNCONFIRMED",
            Self::HouseSellDateLocked => "HOUSE_SELL_DATE_LOCKED",
            Self::HouseHasLinkedDebts => "HOUSE_HAS_LINKED_DEBTS",
            Self::HouseNameRequired => "HOUSE_NAME_REQUIRED",
            Self::HouseNameTooLong => "HOUSE_NAME_TOO_LONG",
            Self::HouseNameDuplicate => "HOUSE_NAME_DUPLICATE",
            Self::HouseNotFound => "HOUSE_NOT_FOUND",
            Self::HousePeriodUnsupported => "HOUSE_PERIOD_UNSUPPORTED",
            Self::HousePeriodLocked => "HOUSE_PERIOD_LOCKED",
            Self::HousePeriodMismatch => "HOUSE_PERIOD_MISMATCH",
            Self::HouseValuationPeriodInvalid => "HOUSE_VALUATION_PERIOD_INVALID",
            Self::HouseValuationDuplicate => "HOUSE_VALUATION_DUPLICATE",
            Self::HouseValuationNotFound => "HOUSE_VALUATION_NOT_FOUND",
            Self::HouseValuationValueInvalid => "HOUSE_VALUATION_VALUE_INVALID",
            Self::HouseValuationValueNegative => "HOUSE_VALUATION_VALUE_NEGATIVE",
            Self::HouseValuationValueTooLarge => "HOUSE_VALUATION_VALUE_TOO_LARGE",
            Self::HouseValuationNoteTooLong => "HOUSE_VALUATION_NOTE_TOO_LONG",
            Self::HouseDataInvalid => "HOUSE_DATA_INVALID",
            Self::HousePurchaseDateRequired => "HOUSE_PURCHASE_DATE_REQUIRED",
            Self::HousePurchaseAmountRequired => "HOUSE_PURCHASE_AMOUNT_REQUIRED",
            Self::HouseDateInvalid => "HOUSE_DATE_INVALID",
            Self::HouseSellBeforePurchase => "HOUSE_SELL_BEFORE_PURCHASE",
            Self::HouseSaleIncomplete => "HOUSE_SALE_INCOMPLETE",
            Self::HouseAmountInvalid => "HOUSE_AMOUNT_INVALID",
            Self::HouseAmountNegative => "HOUSE_AMOUNT_NEGATIVE",
            Self::HouseAmountTooLarge => "HOUSE_AMOUNT_TOO_LARGE",
            Self::HouseValuationOutsideHolding => "HOUSE_VALUATION_OUTSIDE_HOLDING",
            Self::HouseDatesExcludeValuations => "HOUSE_DATES_EXCLUDE_VALUATIONS",
            Self::ExpenseNameRequired => "EXPENSE_NAME_REQUIRED",
            Self::ExpenseNameTooLong => "EXPENSE_NAME_TOO_LONG",
            Self::ExpenseCategoryUnsupported => "EXPENSE_CATEGORY_UNSUPPORTED",
            Self::ExpenseDateInvalid => "EXPENSE_DATE_INVALID",
            Self::ExpenseAmountInvalid => "EXPENSE_AMOUNT_INVALID",
            Self::ExpenseAmountNotPositive => "EXPENSE_AMOUNT_NOT_POSITIVE",
            Self::ExpenseAmountTooLarge => "EXPENSE_AMOUNT_TOO_LARGE",
            Self::ExpenseNoteTooLong => "EXPENSE_NOTE_TOO_LONG",
            Self::ExpenseNotFound => "EXPENSE_NOT_FOUND",
            Self::ExpenseDataInvalid => "EXPENSE_DATA_INVALID",
        }
    }
}
