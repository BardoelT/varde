#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]

use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime, State};
use tauri_plugin_dialog::DialogExt;

use super::AppState;
use crate::domain::transactions::{
    CategorisationSummary, Category, CategoryId, Granularity, ImportSummary, ReapplyScope, Rule,
    RuleDraft, RuleId, RulePeriod, RuleSuggestions, TransactionDate, TransactionFilter,
    TransactionFilterInput, TransactionId, TransactionInsights, TransactionList,
    TransactionYearlyTotal, UNCATEGORIZED_ENTITY,
};
use crate::domain::DomainError;
use crate::error::AppError;
use crate::storage::{Preferences, Record, StepLabel};

/// Rows the page shows before "Load more" (CSD-032 REQ-15).
const PAGE_ROWS: usize = 200;

/// Largest export the app reads, a bound on untrusted input.
const MAX_EXPORT_BYTES: u64 = 50 * 1024 * 1024;

const FILTER_NAME: &str = "Bank export (CSV)";
const FILTER_EXTENSIONS: &[&str] = &["csv"];

/// The filter of a list command as the frontend sent it, with the typed
/// amounts read in the number format of the app.
fn resolved_filter(
    filter: Option<TransactionFilterInput>,
    preferences: &Preferences,
) -> Option<TransactionFilter> {
    filter.map(|input| input.resolve(preferences.load_number_format()))
}

/// A category id as the frontend sends it for a transaction: empty or missing
/// means Uncategorized (CSD-032 REQ-11).
fn optional_category(text: Option<&str>) -> Result<Option<CategoryId>, DomainError> {
    text.filter(|text| !text.is_empty())
        .map(CategoryId::parse)
        .transpose()
}

/// Shows the native "open" dialog for a CSV file. Returns `None` when cancelled.
fn pick_export<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.dialog()
        .file()
        .add_filter(FILTER_NAME, FILTER_EXTENSIONS)
        .blocking_pick_file()
        .and_then(|path| path.into_path().ok())
}

/// Reads an export as UTF-8 text, refusing anything that is not a regular file
/// or is larger than [`MAX_EXPORT_BYTES`]. The error never names the path.
fn read_export(path: &Path) -> Result<String, DomainError> {
    let unreadable = |_: io::Error| DomainError::TransactionImportUnreadable;
    let file = File::open(path).map_err(unreadable)?;
    let metadata = file.metadata().map_err(unreadable)?;
    if !metadata.is_file() || metadata.len() > MAX_EXPORT_BYTES {
        return Err(DomainError::TransactionImportUnreadable);
    }
    let mut text = String::new();
    file.take(MAX_EXPORT_BYTES)
        .read_to_string(&mut text)
        .map_err(unreadable)?;
    Ok(text)
}

/// Picks a file with `ask_path`, reads it and imports it as one undo step.
/// `None` means the user cancelled (CSD-032 REQ-1, REQ-6). The project is not
/// locked while the dialog is open.
pub(crate) fn import_flow(
    state: &AppState,
    ask_path: impl FnOnce() -> Option<PathBuf>,
) -> Result<Option<ImportSummary>, AppError> {
    let Some(path) = ask_path() else {
        return Ok(None);
    };
    let text = read_export(&path)?;
    let mut project = state.edit(StepLabel::import(Record::Transactions))?;
    let (settings, transactions) = project.settings_and_transactions_mut();
    let currency = settings.currency;
    Ok(Some(transactions.import(&text, currency)?))
}

/// Imports a Knab CSV chosen in a native dialog (CSD-032 REQ-1 to REQ-6).
#[tauri::command(async)]
pub fn import_transactions<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<Option<ImportSummary>, AppError> {
    import_flow(&state, || pick_export(&app))
}

/// The transactions that pass the filter, newest first, `limit` rows of them
/// (200 when not given), with the count and total of all matches (REQ-15).
#[tauri::command]
pub fn list_transactions(
    state: State<'_, AppState>,
    filter: Option<TransactionFilterInput>,
    manual_only: Option<bool>,
    limit: Option<usize>,
    preferences: State<'_, Preferences>,
) -> Result<TransactionList, AppError> {
    let filter = resolved_filter(filter, &preferences);
    let project = state.project()?;
    Ok(project.transactions().list_filtered(
        filter.as_ref(),
        manual_only.unwrap_or(false),
        limit.unwrap_or(PAGE_ROWS),
    ))
}

/// Yearly totals over all transactions, newest year first (CSD-032 REQ-24).
#[tauri::command]
pub fn yearly_transaction_totals(
    state: State<'_, AppState>,
) -> Result<Vec<TransactionYearlyTotal>, AppError> {
    let project = state.project()?;
    Ok(project.transactions().yearly_totals())
}

/// What was spent per category, per year or per month, computed from the
/// project as it is now, saved or not (CSD-033 REQ-13).
#[tauri::command]
pub fn get_transaction_insights(
    state: State<'_, AppState>,
    granularity: Option<Granularity>,
) -> Result<TransactionInsights, AppError> {
    let today = state.today();
    let project = state.project()?;
    Ok(project
        .transactions()
        .insights(granularity.unwrap_or_default(), today))
}

/// Sets the category of a transaction by hand; no category means
/// Uncategorized (REQ-11).
#[tauri::command]
pub fn set_transaction_category(
    state: State<'_, AppState>,
    transaction_id: String,
    category_id: Option<String>,
) -> Result<(), AppError> {
    let id = TransactionId::parse(&transaction_id)?;
    let category = optional_category(category_id.as_deref())?;
    let mut project = state.edit(StepLabel::edit(Record::Transaction))?;
    project.transactions_mut().set_category(id, category)?;
    Ok(())
}

/// Takes the manual mark off a transaction; its category stays (REQ-11).
#[tauri::command]
pub fn unset_transaction_manual(
    state: State<'_, AppState>,
    transaction_id: String,
) -> Result<(), AppError> {
    let id = TransactionId::parse(&transaction_id)?;
    let mut project = state.edit(StepLabel::edit(Record::Transaction))?;
    project.transactions_mut().unset_manual(id)?;
    Ok(())
}

#[tauri::command]
pub fn delete_transaction(
    state: State<'_, AppState>,
    transaction_id: String,
) -> Result<(), AppError> {
    let id = TransactionId::parse(&transaction_id)?;
    let mut project = state.edit(StepLabel::delete(Record::Transaction))?;
    project.transactions_mut().delete_transaction(id)?;
    Ok(())
}

/// The most ids one bulk command takes, a bound on what the frontend can send.
const MAX_BULK_IDS: usize = 200_000;

fn parse_ids(texts: &[String]) -> Result<Vec<TransactionId>, AppError> {
    if texts.len() > MAX_BULK_IDS {
        return Err(DomainError::TransactionInputTooLarge.into());
    }
    texts
        .iter()
        .map(|text| TransactionId::parse(text).map_err(AppError::from))
        .collect()
}

/// The ids of every transaction that passes the filters of the list, for
/// "select all in the filter" (REQ-28).
#[tauri::command]
pub fn list_transaction_ids(
    state: State<'_, AppState>,
    filter: Option<TransactionFilterInput>,
    manual_only: Option<bool>,
    preferences: State<'_, Preferences>,
) -> Result<Vec<TransactionId>, AppError> {
    let filter = resolved_filter(filter, &preferences);
    let project = state.project()?;
    Ok(project
        .transactions()
        .matching_ids(filter.as_ref(), manual_only.unwrap_or(false)))
}

/// Sets the category of the chosen transactions by hand, as one change; no
/// category means Uncategorized. Returns how many were set (REQ-28).
#[tauri::command]
pub fn set_transactions_category(
    state: State<'_, AppState>,
    transaction_ids: Vec<String>,
    category_id: Option<String>,
) -> Result<usize, AppError> {
    let ids = parse_ids(&transaction_ids)?;
    let category = optional_category(category_id.as_deref())?;
    let mut project = state.edit(StepLabel::edit(Record::Transactions))?;
    Ok(project
        .transactions_mut()
        .set_category_of_many(&ids, category)?)
}

/// Deletes the chosen transactions and nothing else, as one change. Returns
/// how many were deleted (REQ-28).
#[tauri::command]
pub fn delete_transactions(
    state: State<'_, AppState>,
    transaction_ids: Vec<String>,
) -> Result<usize, AppError> {
    let ids = parse_ids(&transaction_ids)?;
    let mut project = state.edit(StepLabel::delete(Record::Transactions))?;
    Ok(project.transactions_mut().delete_many(&ids)?)
}

/// Removes every transaction; the categories and the rules stay (CSD-032
/// REQ-25). One undo step.
#[tauri::command]
pub fn delete_all_transactions(state: State<'_, AppState>) -> Result<(), AppError> {
    let mut project = state.edit(StepLabel::delete(Record::Transactions))?;
    project.transactions_mut().delete_all_transactions();
    Ok(())
}

/// A category with the number of transactions in it (REQ-30).
#[derive(Serialize)]
pub struct CategoryRow {
    #[serde(flatten)]
    category: Category,
    transaction_count: usize,
}

/// The categories, in the order they were added, each with its number of
/// transactions; "Uncategorized" is built in and not listed (REQ-8, REQ-30).
#[tauri::command]
pub fn list_transaction_categories(
    state: State<'_, AppState>,
) -> Result<Vec<CategoryRow>, AppError> {
    let project = state.project()?;
    Ok(project
        .transactions()
        .category_rows()
        .into_iter()
        .map(|(category, transaction_count)| CategoryRow {
            category: category.clone(),
            transaction_count,
        })
        .collect())
}

#[tauri::command]
pub fn add_transaction_category(
    state: State<'_, AppState>,
    name: String,
) -> Result<Category, AppError> {
    let mut project = state.edit(StepLabel::add(Record::TransactionCategory))?;
    Ok(project.transactions_mut().add_category(&name)?)
}

#[tauri::command]
pub fn rename_transaction_category(
    state: State<'_, AppState>,
    category_id: String,
    name: String,
) -> Result<Category, AppError> {
    let id = editable_category(&category_id)?;
    let mut project = state.edit(StepLabel::edit(Record::TransactionCategory))?;
    Ok(project.transactions_mut().rename_category(id, &name)?)
}

/// Deletes a category; its transactions become Uncategorized and its rules go
/// with it (REQ-13). The frontend has asked for the confirmation.
#[tauri::command]
pub fn delete_transaction_category(
    state: State<'_, AppState>,
    category_id: String,
) -> Result<(), AppError> {
    let id = editable_category(&category_id)?;
    let mut project = state.edit(StepLabel::delete(Record::TransactionCategory))?;
    project.transactions_mut().delete_category(id)?;
    Ok(())
}

/// A rule with the number of transactions it would categorise (REQ-27).
#[derive(Serialize)]
pub struct RuleRow {
    #[serde(flatten)]
    rule: Rule,
    transaction_count: usize,
}

/// The rules, in the order they are applied, each with the number of
/// non-manual transactions it is the first match of (REQ-9, REQ-27).
#[tauri::command]
pub fn list_transaction_rules(state: State<'_, AppState>) -> Result<Vec<RuleRow>, AppError> {
    let project = state.project()?;
    Ok(project
        .transactions()
        .rule_rows()
        .into_iter()
        .map(|(rule, transaction_count)| RuleRow {
            rule: rule.clone(),
            transaction_count,
        })
        .collect())
}

/// How many transactions are categorized and what share that is (REQ-31).
#[tauri::command]
pub fn get_categorisation_summary(
    state: State<'_, AppState>,
) -> Result<CategorisationSummary, AppError> {
    let project = state.project()?;
    Ok(project.transactions().categorisation_summary())
}

/// The rules worth adding, from the counterparties of the transactions (REQ-29).
#[tauri::command]
pub fn suggest_transaction_rules(
    state: State<'_, AppState>,
    min_count: usize,
) -> Result<RuleSuggestions, AppError> {
    let project = state.project()?;
    Ok(project.transactions().suggest_rules(min_count)?)
}

/// How many transactions a rule with this text would match (REQ-29).
#[tauri::command]
pub fn count_transaction_rule_matches(
    state: State<'_, AppState>,
    text: String,
) -> Result<usize, AppError> {
    let project = state.project()?;
    Ok(project.transactions().count_rule_matches(&text)?)
}

/// One rule of "Add rules": a text and the id of its category.
#[derive(Deserialize)]
pub struct NewRule {
    text: String,
    category_id: String,
}

/// The first invalid rule of "Add rules", by its position in the list sent.
#[derive(Serialize)]
pub struct RejectedRule {
    index: usize,
    code: &'static str,
    message: String,
}

/// The result of "Add rules": how many rules were added and how many
/// transactions the rules then changed, or the first invalid rule when
/// nothing was added.
#[derive(Serialize)]
pub struct AddRulesResult {
    added: usize,
    changed: usize,
    rejected: Option<RejectedRule>,
}

/// The most rules one "Add rules" takes, a bound on what the frontend can send
/// (the dialog lists at most 100 suggestions).
const MAX_NEW_RULES: usize = 1000;

/// Adds the rules at the end of the list and applies the rules to the
/// transactions that are not manual, as one change; when one rule is invalid,
/// nothing changes and the answer names it (REQ-29).
#[tauri::command]
pub fn add_transaction_rules(
    state: State<'_, AppState>,
    rules: Vec<NewRule>,
) -> Result<AddRulesResult, AppError> {
    if rules.len() > MAX_NEW_RULES {
        return Err(DomainError::TransactionInputTooLarge.into());
    }
    let mut parsed = Vec::with_capacity(rules.len());
    for (index, rule) in rules.into_iter().enumerate() {
        match CategoryId::parse(&rule.category_id) {
            Ok(category) => parsed.push((rule.text, category)),
            Err(error) => return Ok(rejected(index, &error)),
        }
    }
    let mut project = state.edit(StepLabel::add(Record::TransactionRule))?;
    match project.transactions_mut().add_rules(&parsed) {
        Ok(added) => {
            let changed = project
                .transactions_mut()
                .reapply_rules(ReapplyScope::AllNonManual);
            Ok(AddRulesResult {
                added: added.len(),
                changed,
                rejected: None,
            })
        }
        Err((index, error)) => Ok(rejected(index, &error)),
    }
}

fn rejected(index: usize, error: &DomainError) -> AddRulesResult {
    AddRulesResult {
        added: 0,
        changed: 0,
        rejected: Some(RejectedRule {
            index,
            code: error.code(),
            message: error.to_string(),
        }),
    }
}

/// The id of a category to rename or delete. The built-in Uncategorized has
/// no id, and the frontend names it `uncategorized`: it is protected (REQ-8).
fn editable_category(text: &str) -> Result<CategoryId, DomainError> {
    if text.trim().eq_ignore_ascii_case(UNCATEGORIZED_ENTITY) {
        return Err(DomainError::TransactionCategoryProtected);
    }
    CategoryId::parse(text)
}

/// A rule's category as typed: text that is no category id is an invalid
/// category of the rule, not a missing category.
fn rule_category(text: &str) -> Result<CategoryId, DomainError> {
    CategoryId::parse(text).map_err(|_| DomainError::TransactionRuleCategoryInvalid)
}

/// One end of a rule's period as typed, `dd/mm/yyyy`; empty leaves the end
/// open (REQ-32).
fn typed_day(typed: Option<&str>) -> Result<Option<TransactionDate>, DomainError> {
    match typed.map(str::trim).filter(|text| !text.is_empty()) {
        None => Ok(None),
        Some(text) => TransactionDate::parse_typed(text)
            .map(Some)
            .ok_or(DomainError::TransactionRuleDateInvalid),
    }
}

/// The draft of a rule from what the form sent: the text and the two typed
/// days (REQ-9, REQ-32).
fn typed_draft(
    text: String,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<RuleDraft, DomainError> {
    Ok(RuleDraft::new(
        text,
        RulePeriod {
            from: typed_day(from)?,
            to: typed_day(to)?,
        },
    ))
}

/// Adds a rule, optionally limited to the days `from` to `to` (REQ-32). With
/// `new_category` the rule names a category that is created with it, as one
/// undo step; otherwise it names `category_id` (REQ-9).
#[tauri::command]
pub fn add_transaction_rule(
    state: State<'_, AppState>,
    text: String,
    category_id: Option<String>,
    new_category: Option<String>,
    from: Option<String>,
    to: Option<String>,
) -> Result<Rule, AppError> {
    let draft = typed_draft(text, from.as_deref(), to.as_deref())?;
    if let Some(name) = new_category {
        let mut project = state.edit(StepLabel::add(Record::TransactionRule))?;
        let (_, rule) = project
            .transactions_mut()
            .add_rule_with_new_category(&draft, &name)?;
        return Ok(rule);
    }
    let category = rule_category(category_id.as_deref().unwrap_or_default())?;
    let mut project = state.edit(StepLabel::add(Record::TransactionRule))?;
    Ok(project.transactions_mut().add_rule_with(&draft, category)?)
}

#[tauri::command]
pub fn edit_transaction_rule(
    state: State<'_, AppState>,
    rule_id: String,
    text: String,
    category_id: String,
    from: Option<String>,
    to: Option<String>,
) -> Result<Rule, AppError> {
    let id = RuleId::parse(&rule_id)?;
    let category = rule_category(&category_id)?;
    let draft = typed_draft(text, from.as_deref(), to.as_deref())?;
    let mut project = state.edit(StepLabel::edit(Record::TransactionRule))?;
    Ok(project.transactions_mut().edit_rule(id, &draft, category)?)
}

/// Moves a rule to a position, counted from 0 at the top (REQ-9).
#[tauri::command]
pub fn move_transaction_rule(
    state: State<'_, AppState>,
    rule_id: String,
    position: usize,
) -> Result<(), AppError> {
    let id = RuleId::parse(&rule_id)?;
    let mut project = state.edit(StepLabel::edit(Record::TransactionRule))?;
    project.transactions_mut().move_rule(id, position)?;
    Ok(())
}

#[tauri::command]
pub fn delete_transaction_rule(
    state: State<'_, AppState>,
    rule_id: String,
) -> Result<(), AppError> {
    let id = RuleId::parse(&rule_id)?;
    let mut project = state.edit(StepLabel::delete(Record::TransactionRule))?;
    project.transactions_mut().delete_rule(id)?;
    Ok(())
}

/// How many transactions a re-apply changed (REQ-12).
#[derive(Serialize)]
pub struct ReapplyResult {
    changed: usize,
}

/// Runs the rules again on the uncategorized or all non-manual transactions.
#[tauri::command]
pub fn reapply_transaction_rules(
    state: State<'_, AppState>,
    scope: ReapplyScope,
) -> Result<ReapplyResult, AppError> {
    let mut project = state.edit(StepLabel::reapply(Record::TransactionRule))?;
    let changed = project.transactions_mut().reapply_rules(scope);
    Ok(ReapplyResult { changed })
}
