use super::*;

#[test]
fn test_csd032_ac_8_1_a_new_project_has_the_default_categories() {
    let data = Transactions::default();
    let names: Vec<&str> = data.categories().iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Salary",
            "Groceries",
            "Housing",
            "Utilities",
            "Insurance",
            "Transport",
            "Eating out",
            "Shopping",
            "Health",
            "Leisure",
            "Subscriptions",
            "Savings & transfers",
            "Taxes",
            "Other"
        ]
    );
    assert_eq!(data.total_count(), 0);
}

#[test]
fn test_csd032_ac_8_2_add_and_rename_a_category() {
    let mut data = Transactions::default();
    let added = data.add_category("Example Pets").unwrap();
    let renamed = data.rename_category(added.id, "Pets").unwrap();
    assert_eq!(renamed.name, "Pets");
    assert_eq!(category(&data, "Pets"), added.id);
}

#[test]
fn test_csd032_ac_8_3_invalid_category_names_are_rejected() {
    let mut data = Transactions::default();
    let cases = [
        (String::new(), "TRANSACTION_CATEGORY_NAME_REQUIRED"),
        ("   ".to_owned(), "TRANSACTION_CATEGORY_NAME_REQUIRED"),
        ("x".repeat(51), "TRANSACTION_CATEGORY_NAME_TOO_LONG"),
        ("groceries".to_owned(), "TRANSACTION_CATEGORY_NAME_TAKEN"),
        (
            "uncategorized".to_owned(),
            "TRANSACTION_CATEGORY_NAME_TAKEN",
        ),
    ];
    for (name, code) in cases {
        assert_eq!(
            data.add_category(&name).unwrap_err().code(),
            code,
            "{name:?}"
        );
    }
    assert!(data.add_category(&"x".repeat(50)).is_ok());
}

#[test]
fn test_csd032_ac_8_5_a_category_may_change_only_its_own_case() {
    let mut data = Transactions::default();
    let id = category(&data, "Groceries");
    assert_eq!(
        data.rename_category(id, "GROCERIES").unwrap().name,
        "GROCERIES"
    );
    let other = category(&data, "Taxes");
    let error = data.rename_category(other, "groceries").unwrap_err();
    assert_eq!(error.code(), "TRANSACTION_CATEGORY_NAME_TAKEN");
}

#[test]
fn test_csd032_ac_11_1_a_manual_choice_sticks() {
    let mut data = with_rules(&[]);
    let line = simple("30-10-2026", "Afschrijvingen", "9,00", "Example Shop", "R1");
    data.import(&export(&[line]), Currency::Eur).unwrap();
    let id = data.transactions().first().unwrap().id;
    let shopping = category(&data, "Shopping");
    data.set_category(id, Some(shopping)).unwrap();
    let leisure = category(&data, "Leisure");
    data.add_rule("Example Shop", leisure).unwrap();
    data.reapply_rules(ReapplyScope::AllNonManual);
    let only = data.transactions().first().unwrap();
    assert_eq!(only.category_id, Some(shopping));
    assert!(only.manual);
}

#[test]
fn test_csd032_ac_11_2_back_to_uncategorized_by_hand_stays_manual() {
    let mut data = with_rules(&[("Example", "Shopping")]);
    let line = simple("30-10-2026", "Afschrijvingen", "9,00", "Example Shop", "R1");
    data.import(&export(&[line]), Currency::Eur).unwrap();
    let id = data.transactions().first().unwrap().id;
    data.set_category(id, None).unwrap();
    let only = data.transactions().first().unwrap();
    assert_eq!(only.category_id, None);
    assert!(only.manual);
}

#[test]
fn test_csd032_ac_11_3_a_stale_screen_is_reported() {
    let mut data = Transactions::default();
    let error = data.set_category(TransactionId::new(), None).unwrap_err();
    assert_eq!(error.code(), "TRANSACTION_NOT_FOUND");
    let line = simple("30-10-2026", "Afschrijvingen", "9,00", "Example Shop", "R1");
    data.import(&export(&[line]), Currency::Eur).unwrap();
    let id = data.transactions().first().unwrap().id;
    let error = data.set_category(id, Some(CategoryId::new())).unwrap_err();
    assert_eq!(error.code(), "TRANSACTION_CATEGORY_NOT_FOUND");
}

#[test]
fn test_csd032_ac_13_1_deleting_a_category_clears_transactions_and_rules() {
    let mut data = Transactions::default();
    let leisure = category(&data, "Leisure");
    data.add_rule("Example Gym", leisure).unwrap();
    data.add_rule("Example Club", leisure).unwrap();
    let line = simple("30-10-2026", "Afschrijvingen", "9,00", "Example Gym", "R1");
    data.import(&export(&[line]), Currency::Eur).unwrap();
    let id = data.transactions().first().unwrap().id;
    data.set_category(id, Some(leisure)).unwrap();
    data.delete_category(leisure).unwrap();
    let only = data.transactions().first().unwrap();
    assert_eq!(only.category_id, None);
    assert!(only.manual);
    assert!(data.rules().iter().all(|rule| rule.category_id != leisure));
    assert!(data.categories().iter().all(|c| c.id != leisure));
}

#[test]
fn test_csd032_ac_14_1_deleting_a_transaction_removes_only_that_one() {
    let mut data = imported(&[
        simple("30-10-2026", "Afschrijvingen", "9,00", "Example Shop", "R1"),
        simple("29-10-2026", "Afschrijvingen", "9,00", "Example Cafe", "R2"),
    ]);
    let id = find(&data, "Example Shop").id;
    data.delete_transaction(id).unwrap();
    assert_eq!(data.total_count(), 1);
    assert_eq!(
        data.transactions().first().unwrap().counterparty_name,
        "Example Cafe"
    );
    assert_eq!(
        data.delete_transaction(id).unwrap_err().code(),
        "TRANSACTION_NOT_FOUND"
    );
}

#[test]
fn test_csd032_ac_25_1_deleting_all_transactions_keeps_categories_and_rules() {
    let mut data = imported(&[
        simple("30-10-2026", "Afschrijvingen", "9,00", "Example Shop", "R1"),
        simple("29-10-2026", "Afschrijvingen", "9,00", "Example Cafe", "R2"),
    ]);
    let categories = data.categories().to_vec();
    let rules = data.rules().to_vec();
    assert_eq!(data.delete_all_transactions(), 2);
    assert_eq!(data.total_count(), 0);
    assert_eq!(data.categories(), categories.as_slice());
    assert_eq!(data.rules(), rules.as_slice());
}

#[test]
fn test_csd032_ac_30_1_a_category_counts_its_transactions_by_whoever_set_them() {
    let mut data = imported(&[
        simple("30-10-2026", "Afschrijvingen", "9,00", "Example One", "R1"),
        simple("29-10-2026", "Afschrijvingen", "9,00", "Example Two", "R2"),
        simple(
            "28-10-2026",
            "Afschrijvingen",
            "9,00",
            "Example Three",
            "R3",
        ),
        simple("27-10-2026", "Afschrijvingen", "9,00", "Example Four", "R4"),
    ]);
    let groceries = category(&data, "Groceries");
    let leisure = category(&data, "Leisure");
    let counts = |data: &Transactions| -> (usize, usize) {
        let all = data.category_counts();
        let at = |id| {
            data.categories()
                .iter()
                .position(|each| each.id == id)
                .and_then(|index| all.get(index).copied())
                .unwrap()
        };
        (at(groceries), at(leisure))
    };
    assert_eq!(data.category_counts().len(), data.categories().len());
    for name in ["Example One", "Example Two"] {
        let id = find(&data, name).id;
        data.set_category(id, Some(groceries)).unwrap();
    }
    let three = find(&data, "Example Three").id;
    data.set_category(three, Some(leisure)).unwrap();
    // A hand-set transaction counts too; the uncategorized one does not.
    assert_eq!(counts(&data), (2, 1));
    let four = find(&data, "Example Four").id;
    data.set_category(four, Some(leisure)).unwrap();
    assert_eq!(counts(&data), (2, 2));
}

#[test]
fn test_csd032_ac_31_1_the_summary_gives_the_share_of_categorized_transactions() {
    let mut data = Transactions::default();
    assert_eq!(data.categorisation_summary().percent_label, None);
    no_rules(&mut data);
    let rows: Vec<String> = (1..=8)
        .map(|day| {
            simple(
                &format!("{day:02}-10-2026"),
                "Afschrijvingen",
                "9,00",
                &format!("Example {day}"),
                &format!("R{day}"),
            )
        })
        .collect();
    data.import(&export(&rows), Currency::Eur).unwrap();
    let groceries = category(&data, "Groceries");
    for day in 1..=7 {
        let id = find(&data, &format!("Example {day}")).id;
        data.set_category(id, Some(groceries)).unwrap();
    }
    let summary = data.categorisation_summary();
    assert_eq!((summary.categorized, summary.total), (7, 8));
    assert_eq!(summary.percent_label.as_deref(), Some("87.50%"));
    // Rounded half-up to two decimals: 7 of 11 is 63.6363...
    let more: Vec<String> = (9..=11)
        .map(|day| {
            simple(
                &format!("{day:02}-10-2026"),
                "Afschrijvingen",
                "9,00",
                &format!("Example {day}"),
                &format!("R{day}"),
            )
        })
        .collect();
    data.import(&export(&more), Currency::Eur).unwrap();
    let summary = data.categorisation_summary();
    assert_eq!((summary.categorized, summary.total), (7, 11));
    assert_eq!(summary.percent_label.as_deref(), Some("63.64%"));
}

#[test]
fn test_csd032_ac_11_5_unsetting_manual_keeps_the_category_and_lets_rules_in_again() {
    let mut data = with_rules(&[]);
    let line = simple("30-10-2026", "Afschrijvingen", "9,00", "Example Shop", "R1");
    data.import(&export(&[line]), Currency::Eur).unwrap();
    let id = data.transactions().first().unwrap().id;
    let shopping = category(&data, "Shopping");
    data.set_category(id, Some(shopping)).unwrap();
    data.unset_manual(id).unwrap();
    let only = data.transactions().first().unwrap();
    assert_eq!(only.category_id, Some(shopping));
    assert!(!only.manual);
    let leisure = category(&data, "Leisure");
    data.add_rule("Example Shop", leisure).unwrap();
    assert_eq!(data.reapply_rules(ReapplyScope::AllNonManual), 1);
    assert_eq!(
        data.transactions().first().unwrap().category_id,
        Some(leisure)
    );
}

#[test]
fn test_csd032_ac_11_6_unsetting_manual_on_a_missing_transaction_is_reported() {
    let error = Transactions::default()
        .unset_manual(TransactionId::new())
        .unwrap_err();
    assert_eq!(error.code(), "TRANSACTION_NOT_FOUND");
}
