use super::*;

#[test]
fn test_csd032_ac_15_1_newest_first_and_cut_to_the_limit() {
    let data = many(450);
    let page = data.list_filtered(None, false, 200);
    assert_eq!(page.rows.len(), 200);
    assert_eq!(page.count, 450);
    let dates: Vec<_> = page.rows.iter().map(|row| row.date).collect();
    assert!(dates.windows(2).all(|pair| pair[0] >= pair[1]));
    assert_eq!(data.list_filtered(None, false, 400).rows.len(), 400);
    assert_eq!(data.list_filtered(None, false, 600).rows.len(), 450);
}

#[test]
fn test_csd032_ac_15_2_the_total_covers_all_matches() {
    let mut data = many(450);
    let groceries = category(&data, "Groceries");
    let ids: Vec<TransactionId> = data.transactions().iter().map(|t| t.id).collect();
    for id in ids {
        data.set_category(id, Some(groceries)).unwrap();
    }
    let filter = filter_of(serde_json::json!({ "ids": [groceries.to_string()] }));
    let page = data.list_filtered(Some(&filter), false, 10);
    assert_eq!(page.rows.len(), 10);
    assert_eq!(page.count, 450);
    assert_eq!(page.total, money("-450.00"));
}

#[test]
fn test_csd032_ac_15_5_equal_dates_list_the_one_added_last_first() {
    let data = imported(&[
        simple("30-10-2026", "Afschrijvingen", "9,00", "First added", "R1"),
        simple("30-10-2026", "Afschrijvingen", "9,00", "Second added", "R2"),
    ]);
    let page = data.list_filtered(None, false, 10);
    assert_eq!(page.rows.first().unwrap().counterparty_name, "Second added");
}

#[test]
fn test_csd032_ac_26_1_manual_only_lists_the_hand_set_transactions() {
    let mut data = imported(&[
        simple("30-10-2026", "Afschrijvingen", "9,00", "Example Shop", "R1"),
        simple("29-10-2026", "Afschrijvingen", "5,00", "Example Cafe", "R2"),
        simple(
            "28-10-2026",
            "Afschrijvingen",
            "1,00",
            "Example Bakery",
            "R3",
        ),
    ]);
    let shopping = category(&data, "Shopping");
    let shop = find(&data, "Example Shop").id;
    data.set_category(shop, Some(shopping)).unwrap();
    let page = data.list_filtered(None, true, 10);
    assert_eq!(page.count, 1);
    assert_eq!(page.rows.first().unwrap().counterparty_name, "Example Shop");
    assert_eq!(page.total, money("-9.00"));
    // Without the filter every transaction is listed.
    assert_eq!(data.list_filtered(None, false, 10).count, 3);
    // It combines with the filter of CSD-028: manual and not in "Shopping" is nothing.
    let leisure = category(&data, "Leisure");
    let other = filter_of(serde_json::json!({ "ids": [leisure.to_string()] }));
    assert_eq!(data.list_filtered(Some(&other), true, 10).count, 0);
}

#[test]
fn test_csd032_ac_15_6_uncategorized_can_be_filtered() {
    let mut data = with_rules(&[("Example Shop", "Shopping")]);
    let rows = [
        simple("30-10-2026", "Afschrijvingen", "9,00", "Example Shop", "R1"),
        simple("30-10-2026", "Afschrijvingen", "9,00", "Example Cafe", "R2"),
    ];
    data.import(&export(&rows), Currency::Eur).unwrap();
    let filter = filter_of(serde_json::json!({ "ids": ["uncategorized"] }));
    let page = data.list_filtered(Some(&filter), false, 10);
    assert_eq!(page.count, 1);
    assert_eq!(page.rows.first().unwrap().counterparty_name, "Example Cafe");
}

#[test]
fn test_csd032_ac_15_7_the_period_filter_goes_down_to_the_day() {
    let data = imported(&[
        simple("31-10-2026", "Afschrijvingen", "9,00", "Example Late", "R1"),
        simple(
            "15-10-2026",
            "Afschrijvingen",
            "5,00",
            "Example Middle",
            "R2",
        ),
        simple(
            "01-10-2026",
            "Afschrijvingen",
            "1,00",
            "Example Early",
            "R3",
        ),
        simple(
            "30-09-2026",
            "Afschrijvingen",
            "2,00",
            "Example Before",
            "R4",
        ),
    ]);
    let filter = |json: serde_json::Value| -> TransactionFilter { filter_of(json) };
    let count = |filter: &TransactionFilter| data.list_filtered(Some(filter), false, 10).count;
    // Both ends are included, to the day.
    assert_eq!(
        count(&filter(
            serde_json::json!({ "from": "2026-10-01", "to": "2026-10-15" })
        )),
        2
    );
    assert_eq!(
        count(&filter(
            serde_json::json!({ "from": "2026-10-02", "to": "2026-10-30" })
        )),
        1
    );
    assert_eq!(
        count(&filter(serde_json::json!({ "from": "2026-10-15" }))),
        2
    );
    assert_eq!(count(&filter(serde_json::json!({ "to": "2026-09-30" }))), 1);
    // An inverted range shows nothing, and the ids of the same filter agree.
    let inverted = filter(serde_json::json!({ "from": "2026-10-20", "to": "2026-10-10" }));
    assert_eq!(count(&inverted), 0);
    let range = filter(serde_json::json!({ "from": "2026-10-01", "to": "2026-10-15" }));
    assert_eq!(data.matching_ids(Some(&range), false).len(), 2);
    // A month is no longer a valid bound for this list.
    assert!(serde_json::from_value::<TransactionFilterInput>(
        serde_json::json!({ "from": "2026-10" })
    )
    .is_err());
}

#[test]
fn test_csd032_ac_15_8_the_amount_filter_is_a_range_of_signed_amounts() {
    let data = imported(&[
        simple(
            "30-10-2026",
            "Afschrijvingen",
            "100,00",
            "Example Big",
            "R1",
        ),
        simple(
            "29-10-2026",
            "Afschrijvingen",
            "10,00",
            "Example Small",
            "R2",
        ),
        simple(
            "28-10-2026",
            "Afschrijvingen",
            "10,00",
            "Example Small Too",
            "R3",
        ),
        simple(
            "27-10-2026",
            "Bijschrijvingen",
            "2500,00",
            "Example Salary",
            "R4",
        ),
    ]);
    let count =
        |json: serde_json::Value| data.list_filtered(Some(&filter_of(json)), false, 10).count;
    // A debit is negative; both ends are included.
    assert_eq!(
        count(serde_json::json!({ "min_amount": "-100", "max_amount": "-10" })),
        3
    );
    assert_eq!(
        count(serde_json::json!({ "min_amount": "-99.99", "max_amount": "-10.00" })),
        2
    );
    assert_eq!(count(serde_json::json!({ "min_amount": "0" })), 1);
    assert_eq!(count(serde_json::json!({ "max_amount": "-100" })), 1);
    // Typed amounts are rounded like every typed amount, text that is not an amount leaves the end open.
    assert_eq!(
        count(serde_json::json!({ "min_amount": "-10.004", "max_amount": "-9.996" })),
        2
    );
    assert_eq!(
        count(serde_json::json!({ "min_amount": "abc", "max_amount": "" })),
        4
    );
    // An inverted range shows nothing; the ids of the same filter agree; it combines with the days.
    assert_eq!(
        count(serde_json::json!({ "min_amount": "-10", "max_amount": "-100" })),
        0
    );
    let range = filter_of(
        serde_json::json!({ "min_amount": "-100", "max_amount": "-10", "from": "2026-10-29" }),
    );
    assert_eq!(data.matching_ids(Some(&range), false).len(), 2);
    // The number format decides how the text is read: "1.000,50" in the comma-point format.
    let input: TransactionFilterInput =
        serde_json::from_value(serde_json::json!({ "min_amount": "1.000,50" })).unwrap();
    let thousands = input.resolve(NumberFormat::CommaPoint);
    assert_eq!(data.list_filtered(Some(&thousands), false, 10).count, 1);
}

#[test]
fn test_csd032_ac_15_9_the_text_filters_look_in_counterparty_and_description() {
    let own = "NL00EXAM0000000001";
    let data = imported(&[
        row(
            own,
            "30-10-2026",
            "EUR",
            "Afschrijvingen",
            "9,00",
            "Example Shop",
            "Weekly groceries",
            "R1",
        ),
        row(
            own,
            "29-10-2026",
            "EUR",
            "Afschrijvingen",
            "5,00",
            "Example Cafe",
            "Coffee at the shop",
            "R2",
        ),
        row(
            own,
            "28-10-2026",
            "EUR",
            "Afschrijvingen",
            "7,00",
            "CA",
            "",
            "R3",
        ),
        row(
            own,
            "27-10-2026",
            "EUR",
            "Afschrijvingen",
            "7,00",
            "CAFE",
            "",
            "R4",
        ),
    ]);
    let count =
        |json: serde_json::Value| data.list_filtered(Some(&filter_of(json)), false, 10).count;
    // Without a star the whole text is meant, ignoring case and surrounding spaces.
    assert_eq!(count(serde_json::json!({ "counterparty": "CA" })), 1);
    assert_eq!(count(serde_json::json!({ "counterparty": " ca " })), 1);
    assert_eq!(count(serde_json::json!({ "counterparty": "example" })), 0);
    assert_eq!(
        count(serde_json::json!({ "counterparty": "example shop" })),
        1
    );
    assert_eq!(
        count(serde_json::json!({ "description": "weekly groceries" })),
        1
    );
    assert_eq!(count(serde_json::json!({ "description": "groceries" })), 0);
    // Stars reach other texts: starts with, ends with, contains.
    assert_eq!(count(serde_json::json!({ "counterparty": "CA*" })), 2);
    assert_eq!(count(serde_json::json!({ "counterparty": "Example*" })), 2);
    assert_eq!(count(serde_json::json!({ "counterparty": "*Cafe" })), 2);
    assert_eq!(count(serde_json::json!({ "counterparty": "*ca*" })), 3);
    assert_eq!(count(serde_json::json!({ "description": "*SHOP" })), 1);
    assert_eq!(count(serde_json::json!({ "description": "*shop*" })), 1);
    assert_eq!(count(serde_json::json!({ "description": "*o*" })), 2);
    // Each field looks only in its own column; together they both have to match.
    assert_eq!(count(serde_json::json!({ "counterparty": "*coffee*" })), 0);
    assert_eq!(
        count(serde_json::json!({ "counterparty": "example*", "description": "*groceries" })),
        1
    );
    // An empty field leaves the filter open.
    assert_eq!(
        count(serde_json::json!({ "counterparty": "  ", "description": "" })),
        4
    );
    // The ids, count and total of the same filter agree, and it combines with the others.
    let narrow = filter_of(serde_json::json!({ "counterparty": "example*", "from": "2026-10-30" }));
    assert_eq!(data.matching_ids(Some(&narrow), false).len(), 1);
    assert_eq!(
        data.list_filtered(Some(&narrow), false, 10)
            .total
            .to_string(),
        "-9.00"
    );
}

#[test]
fn test_csd032_ac_28_6_bulk_changes_are_all_or_nothing() {
    let mut data = imported(&[
        simple("30-10-2026", "Afschrijvingen", "9,00", "Example Shop", "R1"),
        simple("29-10-2026", "Afschrijvingen", "5,00", "Example Cafe", "R2"),
        simple(
            "28-10-2026",
            "Afschrijvingen",
            "1,00",
            "Example Bakery",
            "R3",
        ),
    ]);
    let leisure = category(&data, "Leisure");
    let ids: Vec<TransactionId> = ["Example Shop", "Example Cafe"]
        .iter()
        .map(|name| find(&data, name).id)
        .collect();
    // Every id of a filter: all of them, or only the manual ones.
    assert_eq!(data.matching_ids(None, false).len(), 3);
    assert_eq!(data.matching_ids(None, true).len(), 0);
    // Set the category: both are in Leisure and manual, the third is untouched.
    assert_eq!(data.set_category_of_many(&ids, Some(leisure)).unwrap(), 2);
    assert_eq!(data.matching_ids(None, true).len(), 2);
    assert_eq!(find(&data, "Example Cafe").category_id, Some(leisure));
    assert_eq!(find(&data, "Example Bakery").category_id, None);
    assert!(!find(&data, "Example Bakery").manual);
    // An unknown id or category changes nothing at all.
    let before = data.clone();
    let mut with_unknown = ids.clone();
    with_unknown.push(TransactionId::new());
    assert_eq!(
        data.set_category_of_many(&with_unknown, None)
            .unwrap_err()
            .code(),
        "TRANSACTION_NOT_FOUND"
    );
    assert_eq!(
        data.set_category_of_many(&ids, Some(CategoryId::new()))
            .unwrap_err()
            .code(),
        "TRANSACTION_CATEGORY_NOT_FOUND"
    );
    assert_eq!(
        data.delete_many(&with_unknown).unwrap_err().code(),
        "TRANSACTION_NOT_FOUND"
    );
    assert_eq!(data, before);
    // Delete removes exactly those two; no ids is no change.
    assert_eq!(data.delete_many(&[]).unwrap(), 0);
    assert_eq!(data.delete_many(&ids).unwrap(), 2);
    assert_eq!(data.total_count(), 1);
    assert_eq!(
        data.transactions().first().unwrap().counterparty_name,
        "Example Bakery"
    );
}

#[test]
fn test_csd032_con_9_a_long_filter_text_is_cut_and_the_ids_are_bounded() {
    let data = imported(&[simple(
        "30-10-2026",
        "Afschrijvingen",
        "9,00",
        "Example Shop",
        "R1",
    )]);
    let count =
        |json: serde_json::Value| data.list_filtered(Some(&filter_of(json)), false, 10).count;
    // A text longer than 100 characters is cut to 100, so a long exact text can never match a short name.
    let long = "x".repeat(5000);
    assert_eq!(count(serde_json::json!({ "counterparty": long })), 0);
    // The cut is on characters, not bytes: 100 accented characters are kept whole.
    let accented = "é".repeat(150);
    assert_eq!(count(serde_json::json!({ "counterparty": accented })), 0);
    // A very long list of ids is cut, and does not stop the filter from working.
    let ids: Vec<String> = (0..5000).map(|n| format!("id-{n}")).collect();
    assert_eq!(count(serde_json::json!({ "ids": ids })), 0);
    let mut with_uncategorized: Vec<String> = (0..999).map(|n| format!("id-{n}")).collect();
    with_uncategorized.push("uncategorized".to_owned());
    assert_eq!(count(serde_json::json!({ "ids": with_uncategorized })), 1);
}
