//! Tests of CSD-034: the accounts the transactions belong to.

use super::*;

/// A project with the accounts "Example Bank" and "Example Savings", the first
/// holding the transactions R1 to R3 and the second R4.
fn two_accounts() -> (Transactions, AccountId, AccountId) {
    let mut data = Transactions::default();
    let bank = data.add_account("Example Bank").unwrap().id;
    let savings = data.add_account("Example Savings").unwrap().id;
    let rows = |names: &[(&str, &str)]| -> String {
        export(
            &names
                .iter()
                .map(|(name, reference)| {
                    simple("30-10-2026", "Afschrijvingen", "9,00", name, reference)
                })
                .collect::<Vec<_>>(),
        )
    };
    data.import(
        bank,
        &rows(&[
            ("Example One", "R1"),
            ("Example Two", "R2"),
            ("Example Three", "R3"),
        ]),
        Currency::Eur,
    )
    .unwrap();
    data.import(savings, &rows(&[("Example Four", "R4")]), Currency::Eur)
        .unwrap();
    (data, bank, savings)
}

fn count_of(data: &Transactions, account: AccountId) -> usize {
    data.account_rows()
        .into_iter()
        .find(|(each, _)| each.id == account)
        .unwrap()
        .1
}

#[test]
fn test_csd034_ac_2_1_accounts_are_added_in_order_and_renamed() {
    let mut data = Transactions::default();
    let bank = data.add_account("  Example Bank ").unwrap();
    assert_eq!(bank.name, "Example Bank", "trimmed");
    data.add_account("Example Savings").unwrap();
    let renamed = data.rename_account(bank.id, "Example Current").unwrap();
    assert_eq!(renamed.name, "Example Current");
    let names: Vec<&str> = data.accounts().iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, ["Example Current", "Example Savings"]);
    assert_eq!(
        data.rename_account(AccountId::new(), "Example")
            .unwrap_err()
            .code(),
        "TRANSACTION_ACCOUNT_NOT_FOUND"
    );
}

#[test]
fn test_csd034_ac_2_2_invalid_account_names_are_rejected() {
    let mut data = Transactions::default();
    data.add_account("Example Bank").unwrap();
    for (name, code) in [
        ("", "TRANSACTION_ACCOUNT_NAME_REQUIRED"),
        ("   ", "TRANSACTION_ACCOUNT_NAME_REQUIRED"),
        (&"x".repeat(51), "TRANSACTION_ACCOUNT_NAME_TOO_LONG"),
        ("example bank", "TRANSACTION_ACCOUNT_NAME_TAKEN"),
        (" EXAMPLE BANK ", "TRANSACTION_ACCOUNT_NAME_TAKEN"),
    ] {
        assert_eq!(data.add_account(name).unwrap_err().code(), code, "{name:?}");
    }
    assert!(
        data.add_account(&"x".repeat(50)).is_ok(),
        "50 characters are fine"
    );
    assert_eq!(data.accounts().len(), 2);
}

#[test]
fn test_csd034_ac_2_3_an_account_may_keep_its_own_name_in_another_case() {
    let mut data = Transactions::default();
    let bank = data.add_account("Example Bank").unwrap().id;
    assert_eq!(
        data.rename_account(bank, "EXAMPLE BANK").unwrap().name,
        "EXAMPLE BANK"
    );
    data.add_account("Example Savings").unwrap();
    assert_eq!(
        data.rename_account(bank, "example savings")
            .unwrap_err()
            .code(),
        "TRANSACTION_ACCOUNT_NAME_TAKEN"
    );
}

#[test]
fn test_csd034_ac_2_4_deleting_an_account_deletes_its_transactions_only() {
    let (mut data, bank, savings) = two_accounts();
    let categories = data.categories().to_vec();
    let rules = data.rules().to_vec();
    assert_eq!(data.delete_account(bank).unwrap(), 3);
    assert_eq!(data.accounts().len(), 1);
    assert_eq!(data.total_count(), 1);
    assert_eq!(count_of(&data, savings), 1);
    assert_eq!(data.categories(), categories.as_slice());
    assert_eq!(data.rules(), rules.as_slice());
    assert_eq!(
        data.delete_account(bank).unwrap_err().code(),
        "TRANSACTION_ACCOUNT_NOT_FOUND"
    );
}

#[test]
fn test_csd034_ac_4_1_deleting_the_transactions_of_an_account_keeps_the_account() {
    let (mut data, bank, savings) = two_accounts();
    assert_eq!(data.delete_account_transactions(bank).unwrap(), 3);
    assert_eq!(count_of(&data, bank), 0);
    assert_eq!(count_of(&data, savings), 1);
    assert_eq!(data.accounts().len(), 2);
    // An account without transactions has nothing to delete.
    assert_eq!(data.delete_account_transactions(bank).unwrap(), 0);
    assert_eq!(
        data.delete_account_transactions(AccountId::new())
            .unwrap_err()
            .code(),
        "TRANSACTION_ACCOUNT_NOT_FOUND"
    );
}

#[test]
fn test_csd034_ac_3_1_the_transactions_land_in_the_chosen_account() {
    let (data, bank, savings) = two_accounts();
    assert_eq!(count_of(&data, bank), 3);
    assert_eq!(count_of(&data, savings), 1);
    assert!(data
        .transactions()
        .iter()
        .filter(|t| t.counterparty_name == "Example Four")
        .all(|t| t.account_id == savings));
}

#[test]
fn test_csd034_ac_5_1_duplicates_are_checked_within_one_account() {
    let (mut data, bank, savings) = two_accounts();
    let again = export(&[simple(
        "30-10-2026",
        "Afschrijvingen",
        "9,00",
        "Example One",
        "R1",
    )]);
    let summary = data.import(bank, &again, Currency::Eur).unwrap();
    assert_eq!((summary.added, summary.skipped), (0, 1));
    // The same file into another account is not a duplicate there.
    let summary = data.import(savings, &again, Currency::Eur).unwrap();
    assert_eq!((summary.added, summary.skipped), (1, 0));
    assert_eq!(count_of(&data, savings), 2);
}

#[test]
fn test_csd034_ac_5_2_an_import_into_an_unknown_account_adds_nothing() {
    let (mut data, ..) = two_accounts();
    let before = data.total_count();
    let text = export(&[simple(
        "30-10-2026",
        "Afschrijvingen",
        "9,00",
        "Example New",
        "R9",
    )]);
    let error = data
        .import(AccountId::new(), &text, Currency::Eur)
        .unwrap_err();
    assert_eq!(error.code(), "TRANSACTION_ACCOUNT_NOT_FOUND");
    assert_eq!(data.total_count(), before);
    // The account is checked before the file: a bad file into a missing account says so first.
    let error = data
        .import(AccountId::new(), "nonsense", Currency::Eur)
        .unwrap_err();
    assert_eq!(error.code(), "TRANSACTION_ACCOUNT_NOT_FOUND");
    // A deleted account is unknown too.
    let (mut other, bank, _) = two_accounts();
    other.delete_account(bank).unwrap();
    assert_eq!(
        other.import(bank, &text, Currency::Eur).unwrap_err().code(),
        "TRANSACTION_ACCOUNT_NOT_FOUND"
    );
}

#[test]
fn test_csd034_ac_7_2_the_file_data_is_checked_for_accounts() {
    let (data, ..) = two_accounts();
    let parts = |list: Vec<Transaction>, accounts: Vec<Account>| {
        Transactions::from_parts(
            list,
            accounts,
            data.categories().to_vec(),
            data.rules().to_vec(),
        )
    };
    let list = data.transactions().to_vec();
    let accounts = data.accounts().to_vec();
    assert!(parts(list.clone(), accounts.clone()).is_ok());
    // A transaction whose account does not exist.
    let mut orphan = list.clone();
    orphan.first_mut().unwrap().account_id = AccountId::new();
    assert!(parts(orphan, accounts.clone()).is_err());
    // Transactions without any account at all.
    assert!(parts(list.clone(), Vec::new()).is_err());
    // Two accounts with the same name ignoring case, or with the same id, and a name that is not trimmed.
    let mut same_name = accounts.clone();
    same_name.last_mut().unwrap().name = "EXAMPLE BANK".to_owned();
    assert!(parts(list.clone(), same_name).is_err());
    let mut same_id = accounts.clone();
    same_id.last_mut().unwrap().id = accounts.first().unwrap().id;
    assert!(parts(list.clone(), same_id).is_err());
    let mut untrimmed = accounts.clone();
    untrimmed.first_mut().unwrap().name = " Example Bank".to_owned();
    assert!(parts(list.clone(), untrimmed).is_err());
    let mut empty = accounts;
    empty.first_mut().unwrap().name = String::new();
    assert!(parts(list, empty).is_err());
    // An account without transactions is fine.
    assert!(parts(
        Vec::new(),
        vec![Account {
            id: AccountId::new(),
            name: "Example".to_owned()
        }]
    )
    .is_ok());
}

#[test]
fn test_csd034_ac_6_2_the_list_filters_on_account() {
    let (data, bank, savings) = two_accounts();
    let count =
        |json: serde_json::Value| data.list_filtered(Some(&filter_of(json)), false, 10).count;
    assert_eq!(
        count(serde_json::json!({ "account_ids": [savings.to_string()] })),
        1
    );
    assert_eq!(
        count(serde_json::json!({ "account_ids": [bank.to_string()] })),
        3
    );
    assert_eq!(
        count(serde_json::json!({ "account_ids": [bank.to_string(), savings.to_string()] })),
        4
    );
    // No choice keeps every account; a choice that names nothing known keeps none.
    assert_eq!(count(serde_json::json!({ "account_ids": [] })), 4);
    assert_eq!(
        count(serde_json::json!({ "account_ids": ["not-an-id"] })),
        0
    );
    // It combines with the others, and the ids and total follow.
    let narrow = filter_of(serde_json::json!({
        "account_ids": [bank.to_string()],
        "counterparty": "Example T*",
    }));
    assert_eq!(data.matching_ids(Some(&narrow), false).len(), 2);
    assert_eq!(
        data.list_filtered(Some(&narrow), false, 10)
            .total
            .to_string(),
        "-18.00"
    );
}
