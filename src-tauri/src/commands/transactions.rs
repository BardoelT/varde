#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]

use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Runtime, State};
use tauri_plugin_dialog::DialogExt;

use super::AppState;
use crate::domain::transactions::{
    Category, CategoryId, ImportSummary, ReapplyScope, Rule, RuleId, TransactionId,
    TransactionList, TransactionYearlyTotal, UNCATEGORIZED_ENTITY,
};
use crate::domain::{DomainError, StatementFilter};
use crate::error::AppError;
use crate::storage::{Record, StepLabel};

/// Rows the page shows before "Load more" (CSD-032 REQ-15).
const PAGE_ROWS: usize = 200;

/// Largest export the app reads, a bound on untrusted input.
const MAX_EXPORT_BYTES: u64 = 50 * 1024 * 1024;

const FILTER_NAME: &str = "Bank export (CSV)";
const FILTER_EXTENSIONS: &[&str] = &["csv"];

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
    filter: Option<StatementFilter>,
    limit: Option<usize>,
) -> Result<TransactionList, AppError> {
    let project = state.project()?;
    Ok(project
        .transactions()
        .list_filtered(filter.as_ref(), limit.unwrap_or(PAGE_ROWS)))
}

/// Yearly totals over all transactions, newest year first (CSD-032 REQ-24).
#[tauri::command]
pub fn yearly_transaction_totals(
    state: State<'_, AppState>,
) -> Result<Vec<TransactionYearlyTotal>, AppError> {
    let project = state.project()?;
    Ok(project.transactions().yearly_totals())
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
    let category = category_id
        .as_deref()
        .filter(|text| !text.is_empty())
        .map(CategoryId::parse)
        .transpose()?;
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

/// The categories, in the order they were added; "Uncategorized" is built in
/// and not listed (REQ-8).
#[tauri::command]
pub fn list_transaction_categories(state: State<'_, AppState>) -> Result<Vec<Category>, AppError> {
    let project = state.project()?;
    Ok(project.transactions().categories().to_vec())
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

/// The rules, in the order they are applied (REQ-9).
#[tauri::command]
pub fn list_transaction_rules(state: State<'_, AppState>) -> Result<Vec<Rule>, AppError> {
    let project = state.project()?;
    Ok(project.transactions().rules().to_vec())
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

/// Adds a rule. With `new_category` the rule names a category that is created
/// with it, as one undo step; otherwise it names `category_id` (REQ-9).
#[tauri::command]
pub fn add_transaction_rule(
    state: State<'_, AppState>,
    text: String,
    category_id: Option<String>,
    new_category: Option<String>,
) -> Result<Rule, AppError> {
    if let Some(name) = new_category {
        let mut project = state.edit(StepLabel::add(Record::TransactionRule))?;
        let (_, rule) = project
            .transactions_mut()
            .add_rule_with_new_category(&text, &name)?;
        return Ok(rule);
    }
    let category = rule_category(category_id.as_deref().unwrap_or_default())?;
    let mut project = state.edit(StepLabel::add(Record::TransactionRule))?;
    Ok(project.transactions_mut().add_rule(&text, category)?)
}

#[tauri::command]
pub fn edit_transaction_rule(
    state: State<'_, AppState>,
    rule_id: String,
    text: String,
    category_id: String,
) -> Result<Rule, AppError> {
    let id = RuleId::parse(&rule_id)?;
    let category = rule_category(&category_id)?;
    let mut project = state.edit(StepLabel::edit(Record::TransactionRule))?;
    Ok(project.transactions_mut().edit_rule(id, &text, category)?)
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
