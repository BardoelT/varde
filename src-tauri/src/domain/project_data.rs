//! All the data of one project: the areas and the settings, as stored in the project file.

use super::bank_accounts::BankAccounts;
use super::debts::Debts;
use super::expenses::Expenses;
use super::houses::Houses;
use super::income::Income;
use super::investments::Investments;
use super::settings::Settings;

/// Everything a project file holds: the income data, the investments, the bank
/// accounts, the debts, the expenses, the houses and the project settings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectData {
    /// The income data.
    pub income: Income,
    /// The investment data.
    pub investments: Investments,
    /// The bank account data.
    pub bank_accounts: BankAccounts,
    /// The debt data.
    pub debts: Debts,
    /// The house data.
    pub houses: Houses,
    /// The expense data.
    pub expenses: Expenses,
    /// The project settings.
    pub settings: Settings,
}
