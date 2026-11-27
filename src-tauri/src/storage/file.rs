use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::StorageError;
use crate::domain::transactions::{Account, Budget, Category, Rule, Transaction};
use crate::domain::{
    BankAccounts, Debts, Expenses, Houses, Income, Investments, ProjectData, Settings, Transactions,
};

/// Version of the project file format this app writes and understands.
pub const FORMAT_VERSION: u32 = 5;

/// The oldest version the app opens: it adds the sections the newer versions
/// hold, empty, while reading (CSD-002 REQ-30).
const OLDEST_OPENABLE_VERSION: u64 = 4;

/// Largest project file the app will read and the largest it will write, so a
/// file it saved can always be opened again (CSD-002 REQ-27).
pub const MAX_FILE_BYTES: u64 = 50 * 1024 * 1024;

#[derive(Serialize)]
struct FileRef<'a> {
    format_version: u32,
    income: &'a Income,
    investments: &'a Investments,
    bank_accounts: &'a BankAccounts,
    debts: &'a Debts,
    expenses: &'a Expenses,
    houses: &'a Houses,
    settings: &'a Settings,
    transactions: &'a [Transaction],
    transaction_accounts: &'a [Account],
    transaction_categories: &'a [Category],
    transaction_rules: &'a [Rule],
    budgets: &'a [Budget],
}

/// Every section is required and unknown fields are rejected, so a misspelled
/// section name can never load as an empty section and be saved over the data
/// (CSD-002 REQ-17).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileOwned {
    #[serde(rename = "format_version")]
    _format_version: u32,
    income: Income,
    investments: Investments,
    bank_accounts: BankAccounts,
    debts: Debts,
    expenses: Expenses,
    houses: Houses,
    settings: Settings,
    transactions: Vec<Transaction>,
    transaction_accounts: Vec<Account>,
    transaction_categories: Vec<Category>,
    transaction_rules: Vec<Rule>,
    budgets: Vec<Budget>,
}

/// Renders the project file: JSON with a `format_version` and decimal strings.
pub(super) fn to_json(data: &ProjectData) -> Result<String, StorageError> {
    serde_json::to_string_pretty(&FileRef {
        format_version: FORMAT_VERSION,
        income: &data.income,
        investments: &data.investments,
        bank_accounts: &data.bank_accounts,
        debts: &data.debts,
        expenses: &data.expenses,
        houses: &data.houses,
        settings: &data.settings,
        transactions: data.transactions.transactions(),
        transaction_accounts: data.transactions.accounts(),
        transaction_categories: data.transactions.categories(),
        transaction_rules: data.transactions.rules(),
        budgets: data.transactions.budgets(),
    })
    .map_err(|error| StorageError::SaveFailed(io::Error::other(error)))
}

/// Parses and validates project file contents.
///
/// Files with a missing, zero, older or newer `format_version` are rejected. An
/// older one has its own error, and is upgraded outside the app first (CSD-002
/// REQ-20, REQ-29), except a version 4 file, which opens with an empty
/// `budgets` section (CSD-002 REQ-30).
pub(super) fn from_json(text: &str) -> Result<ProjectData, StorageError> {
    let mut value: serde_json::Value =
        serde_json::from_str(text).map_err(|_| StorageError::InvalidFile)?;
    let version = value
        .get("format_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or(StorageError::InvalidFile)?;
    if version == 0 {
        return Err(StorageError::InvalidFile);
    }
    if version < OLDEST_OPENABLE_VERSION {
        return Err(StorageError::FileOlder);
    }
    if version > u64::from(FORMAT_VERSION) {
        return Err(StorageError::FileNewer);
    }
    if version < u64::from(FORMAT_VERSION) {
        // A version 4 file has no budgets; one that names the section is invalid.
        let object = value.as_object_mut().ok_or(StorageError::InvalidFile)?;
        if object.contains_key("budgets") {
            return Err(StorageError::InvalidFile);
        }
        object.insert("budgets".to_owned(), serde_json::Value::Array(Vec::new()));
    }
    let file: FileOwned = serde_json::from_value(value).map_err(|_| StorageError::InvalidFile)?;
    file.income
        .validate()
        .map_err(|_| StorageError::InvalidFile)?;
    file.investments
        .validate()
        .map_err(|_| StorageError::InvalidFile)?;
    file.bank_accounts
        .validate()
        .map_err(|_| StorageError::InvalidFile)?;
    file.debts
        .validate()
        .map_err(|_| StorageError::InvalidFile)?;
    file.expenses
        .validate()
        .map_err(|_| StorageError::InvalidFile)?;
    file.houses
        .validate()
        .map_err(|_| StorageError::InvalidFile)?;
    file.debts
        .validate_links(&file.houses)
        .map_err(|_| StorageError::InvalidFile)?;
    let transactions = Transactions::from_parts(
        file.transactions,
        file.transaction_accounts,
        file.transaction_categories,
        file.transaction_rules,
    )
    .and_then(|transactions| transactions.with_budgets(file.budgets))
    .map_err(|_| StorageError::InvalidFile)?;
    Ok(ProjectData {
        income: file.income,
        investments: file.investments,
        bank_accounts: file.bank_accounts,
        debts: file.debts,
        expenses: file.expenses,
        houses: file.houses,
        settings: file.settings,
        transactions,
    })
}

/// Reads a project file, refusing anything that is not a regular file or is
/// larger than [`MAX_FILE_BYTES`].
pub(super) fn read(path: &Path) -> Result<ProjectData, StorageError> {
    // Windows refuses to open a folder as a file (access denied), so a folder is told apart
    // first to give the same error on every platform.
    if fs::metadata(path)
        .map_err(StorageError::OpenFailed)?
        .is_dir()
    {
        return Err(StorageError::InvalidFile);
    }
    let file = File::open(path).map_err(StorageError::OpenFailed)?;
    let metadata = file.metadata().map_err(StorageError::OpenFailed)?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        return Err(StorageError::InvalidFile);
    }
    let mut text = String::new();
    file.take(MAX_FILE_BYTES)
        .read_to_string(&mut text)
        .map_err(|error| match error.kind() {
            io::ErrorKind::InvalidData => StorageError::InvalidFile,
            _ => StorageError::OpenFailed(error),
        })?;
    from_json(&text)
}

/// Writes the project file atomically: contents go to a temporary file in the
/// same directory which is renamed over the target, so a failed write never
/// damages an existing file (CSD-002 REQ-5).
pub(super) fn write(path: &Path, data: &ProjectData) -> Result<(), StorageError> {
    write_limited(path, data, MAX_FILE_BYTES)
}

/// [`write`] with the size limit given, so the limit can be tested with a small
/// project. A project whose file would be larger than `max_bytes` is refused
/// and nothing is written (CSD-002 REQ-27).
pub(super) fn write_limited(
    path: &Path,
    data: &ProjectData,
    max_bytes: u64,
) -> Result<(), StorageError> {
    let json = to_json(data)?;
    if u64::try_from(json.len()).map_or(true, |len| len > max_bytes) {
        return Err(StorageError::ProjectTooLarge);
    }
    write_atomic_with(path, |file| file.write_all(json.as_bytes()))
        .map_err(StorageError::SaveFailed)
}

/// The size of the project file as it would be written now, in bytes.
pub(super) fn size_of(data: &ProjectData) -> Result<u64, StorageError> {
    let json = to_json(data)?;
    Ok(u64::try_from(json.len()).unwrap_or(u64::MAX))
}

/// The backup copy next to a project file: `example-a.varde.bak`.
pub(super) fn backup_path(path: &Path) -> Option<std::path::PathBuf> {
    let mut name = path.file_name()?.to_os_string();
    name.push(".bak");
    Some(path.with_file_name(name))
}

/// Copies the project file that is about to be replaced to its backup, in the
/// same atomic way a project is written. Nothing happens when there is no file
/// yet. An existing backup is replaced (CSD-002 REQ-28).
pub(super) fn backup(path: &Path) -> Result<(), StorageError> {
    let Ok(metadata) = fs::metadata(path) else {
        return Ok(());
    };
    if !metadata.is_file() {
        return Ok(());
    }
    let target = backup_path(path)
        .ok_or_else(|| StorageError::SaveFailed(io::Error::from(io::ErrorKind::InvalidInput)))?;
    let bytes = fs::read(path).map_err(StorageError::SaveFailed)?;
    write_atomic_with(&target, |file| file.write_all(&bytes)).map_err(StorageError::SaveFailed)
}

/// Atomic write with an injectable writer, so failures can be tested.
pub(super) fn write_atomic_with(
    path: &Path,
    write: impl FnOnce(&mut File) -> io::Result<()>,
) -> io::Result<()> {
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let temporary = directory.join(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        Uuid::new_v4().simple()
    ));

    let result = (|| {
        let mut file = create_private(&temporary)?;
        write(&mut file)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        // Best effort: the original file is untouched either way.
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// Creates a new file readable only by the owner: it holds financial data.
#[cfg(unix)]
fn create_private(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

/// Creates a new file. Windows has no file modes; a file in the user's own folders is
/// private to the user by the folder's access rules (CSD-038 REQ-11).
#[cfg(not(unix))]
fn create_private(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}
