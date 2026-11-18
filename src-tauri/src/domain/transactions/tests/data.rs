use super::*;

#[test]
fn test_csd032_ac_17_4_a_transaction_with_an_unknown_category_is_invalid() {
    let data = imported(&[simple(
        "30-10-2026",
        "Afschrijvingen",
        "9,00",
        "Example Shop",
        "R1",
    )]);
    let mut list = data.transactions().to_vec();
    list.first_mut().unwrap().category_id = Some(CategoryId::new());
    let error = Transactions::from_parts(
        list,
        data.accounts().to_vec(),
        data.categories().to_vec(),
        data.rules().to_vec(),
    )
    .unwrap_err();
    assert_eq!(error.code(), "TRANSACTION_DATA_INVALID");
}

#[test]
fn test_csd032_ac_17_6_duplicate_ids_and_dangling_rules_are_invalid() {
    let data = imported(&[simple(
        "30-10-2026",
        "Afschrijvingen",
        "9,00",
        "Example Shop",
        "R1",
    )]);
    let mut doubled = data.transactions().to_vec();
    doubled.push(doubled.first().unwrap().clone());
    assert!(Transactions::from_parts(
        doubled,
        data.accounts().to_vec(),
        data.categories().to_vec(),
        data.rules().to_vec()
    )
    .is_err());
    let mut rules = data.rules().to_vec();
    rules.first_mut().unwrap().category_id = CategoryId::new();
    assert!(Transactions::from_parts(
        Vec::new(),
        data.accounts().to_vec(),
        data.categories().to_vec(),
        rules
    )
    .is_err());
}

#[test]
fn test_csd032_ac_19_1_a_clone_shares_the_list_until_it_changes() {
    let mut data = many(10);
    let before = data.clone();
    assert_eq!(data.shared_weight().0, before.shared_weight().0);
    let id = data.transactions().first().unwrap().id;
    data.set_category(id, None).unwrap();
    assert_ne!(data.shared_weight().0, before.shared_weight().0);
    assert!(!before.transactions().first().unwrap().manual);
}

#[test]
fn test_csd032_ac_24_1_yearly_totals_split_income_and_spending_newest_year_first() {
    let data = imported(&[
        simple(
            "30-10-2026",
            "Afschrijvingen",
            "23,97",
            "Example Shop",
            "R1",
        ),
        simple(
            "28-10-2026",
            "Bijschrijvingen",
            "2500,00",
            "Example Employer",
            "R2",
        ),
        simple(
            "30-12-2025",
            "Afschrijvingen",
            "100,00",
            "Example Shop",
            "R3",
        ),
        simple(
            "15-12-2025",
            "Afschrijvingen",
            "50,00",
            "Example Cafe",
            "R4",
        ),
    ]);
    let totals = data.yearly_totals();
    assert_eq!(
        totals.iter().map(|row| row.year).collect::<Vec<_>>(),
        [2026, 2025]
    );
    let newest = totals.first().unwrap();
    assert_eq!(newest.income, money("2500.00"));
    assert_eq!(newest.spent, money("23.97"));
    assert_eq!(newest.net, money("2476.03"));
    assert_eq!(newest.transaction_count, 2);
    let older = totals.last().unwrap();
    assert_eq!(older.income, money("0.00"));
    assert_eq!(older.spent, money("150.00"));
    assert_eq!(older.net, money("-150.00"));
}

#[test]
fn test_csd032_ac_24_2_no_transactions_no_totals_and_a_deleted_year_is_gone() {
    assert_eq!(Transactions::default().yearly_totals(), Vec::new());
    let mut data = imported(&[simple("30-12-2025", "Afschrijvingen", "10,00", "A", "R1")]);
    let id = data.transactions().first().unwrap().id;
    data.delete_transaction(id).unwrap();
    assert_eq!(data.yearly_totals(), Vec::new());
}
