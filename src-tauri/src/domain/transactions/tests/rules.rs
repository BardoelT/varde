use super::*;

#[test]
fn test_csd032_ac_9_1_add_and_reorder_rules() {
    let mut data = Transactions::default();
    let groceries = category(&data, "Groceries");
    let before = data.rules().len();
    let added = data.add_rule("Example Market", groceries).unwrap();
    assert_eq!(data.rules().last().unwrap().text, "Example Market");
    assert_eq!(data.rules().len(), before + 1);
    data.move_rule(added.id, 0).unwrap();
    assert_eq!(data.rules().first().unwrap().id, added.id);
    data.move_rule(added.id, 9999).unwrap();
    assert_eq!(data.rules().last().unwrap().id, added.id);
}

#[test]
fn test_csd032_ac_9_3_invalid_rules_are_rejected() {
    let mut data = Transactions::default();
    let groceries = category(&data, "Groceries");
    assert_eq!(
        data.add_rule("  ", groceries).unwrap_err().code(),
        "TRANSACTION_RULE_TEXT_REQUIRED"
    );
    assert_eq!(
        data.add_rule(&"x".repeat(101), groceries)
            .unwrap_err()
            .code(),
        "TRANSACTION_RULE_TEXT_TOO_LONG"
    );
    let unknown = CategoryId::new();
    assert_eq!(
        data.add_rule("Example", unknown).unwrap_err().code(),
        "TRANSACTION_RULE_CATEGORY_INVALID"
    );
}

#[test]
fn test_csd032_ac_9_4_the_seed_rules_name_existing_categories() {
    let data = Transactions::default();
    assert!(data.rules().len() >= 10);
    for rule in data.rules() {
        assert!(data.categories().iter().any(|c| c.id == rule.category_id));
    }
}

#[test]
fn test_csd032_ac_9_5_edit_and_delete_a_rule() {
    let mut data = Transactions::default();
    let leisure = category(&data, "Leisure");
    let rule = data.add_rule("Example Gym", leisure).unwrap();
    let edited = data
        .edit_rule(
            rule.id,
            &RuleDraft::new("Example Club", RulePeriod::default()),
            leisure,
        )
        .unwrap();
    assert_eq!(edited.text, "Example Club");
    data.delete_rule(rule.id).unwrap();
    assert_eq!(
        data.delete_rule(rule.id).unwrap_err().code(),
        "TRANSACTION_RULE_NOT_FOUND"
    );
}

#[test]
fn test_csd032_ac_10_1_the_first_matching_rule_wins() {
    let mut data = with_rules(&[("Example Market*", "Groceries"), ("Example*", "Shopping")]);
    let line = simple(
        "30-10-2026",
        "Afschrijvingen",
        "9,00",
        "EXAMPLE MARKET Town",
        "R1",
    );
    import_to_first(&mut data, &export(&[line]), Currency::Eur).unwrap();
    let groceries = category(&data, "Groceries");
    assert_eq!(
        data.transactions().first().unwrap().category_id,
        Some(groceries)
    );
}

#[test]
fn test_csd032_ac_10_2_the_description_is_matched_too() {
    let mut data = with_rules(&[("Rent October*", "Housing")]);
    let line = row(
        "NL00EXAM0000000001",
        "30-10-2026",
        "EUR",
        "Afschrijvingen",
        "1000,00",
        "",
        "Rent October 2026",
        "R1",
    );
    import_to_first(&mut data, &export(&[line]), Currency::Eur).unwrap();
    let housing = category(&data, "Housing");
    assert_eq!(
        data.transactions().first().unwrap().category_id,
        Some(housing)
    );
}

#[test]
fn test_csd032_ac_10_3_no_match_is_uncategorized() {
    let mut data = with_rules(&[]);
    let line = simple("30-10-2026", "Afschrijvingen", "9,00", "Example Shop", "R1");
    import_to_first(&mut data, &export(&[line]), Currency::Eur).unwrap();
    let only = data.transactions().first().unwrap();
    assert_eq!(only.category_id, None);
    assert!(!only.manual);
}

#[test]
fn test_csd032_ac_10_6_matching_ignores_case_but_not_the_text() {
    let mut data = with_rules(&[("example market", "Groceries")]);
    let line = simple("30-10-2026", "Afschrijvingen", "9,00", "Example Mkt", "R1");
    import_to_first(&mut data, &export(&[line]), Currency::Eur).unwrap();
    assert_eq!(data.transactions().first().unwrap().category_id, None);
}

#[test]
fn test_csd032_ac_12_1_including_manual_also_changes_hand_set_categories() {
    let mut data = reapply_fixture();
    let leisure = category(&data, "Leisure");
    let shopping = category(&data, "Shopping");
    let id = find(&data, "Example One").id;
    data.set_category(id, Some(shopping)).unwrap();
    // Apply rules leaves the hand-set category alone: only Two and Three are covered.
    assert_eq!(data.reapply_rules(ReapplyScope::AllNonManual), 2);
    assert_eq!(find(&data, "Example One").category_id, Some(shopping));
    assert!(find(&data, "Example One").manual);
    // Including manual changes it too, and the transaction is no longer manual.
    assert_eq!(data.reapply_rules(ReapplyScope::All), 1);
    assert_eq!(find(&data, "Example One").category_id, Some(leisure));
    assert!(!find(&data, "Example One").manual);
    assert_eq!(data.reapply_rules(ReapplyScope::All), 0);
}

#[test]
fn test_csd032_ac_12_2_reapply_to_all_non_manual() {
    let mut data = reapply_fixture();
    let leisure = category(&data, "Leisure");
    assert_eq!(data.reapply_rules(ReapplyScope::AllNonManual), 3);
    assert_eq!(find(&data, "Example One").category_id, Some(leisure));
    assert_eq!(find(&data, "Example Three").category_id, Some(leisure));
}

#[test]
fn test_csd032_ac_12_3_nothing_to_change_reports_zero() {
    let mut data = reapply_fixture();
    data.reapply_rules(ReapplyScope::AllNonManual);
    let before = data.clone();
    assert_eq!(data.reapply_rules(ReapplyScope::AllNonManual), 0);
    assert_eq!(data, before);
}

#[test]
fn test_csd032_ac_9_6_a_rule_can_create_its_category_with_it() {
    let mut data = Transactions::default();
    let (categories, rules) = (data.categories().len(), data.rules().len());
    let (category, rule) = data
        .add_rule_with_new_category(
            &RuleDraft::new("Example Gym", RulePeriod::default()),
            "Example Sport",
        )
        .unwrap();
    assert_eq!(data.categories().len(), categories + 1);
    assert_eq!(data.categories().last().unwrap().name, "Example Sport");
    assert_eq!(data.rules().len(), rules + 1);
    assert_eq!(data.rules().last().unwrap().category_id, category.id);
    assert_eq!(rule.text, "Example Gym");
}

#[test]
fn test_csd032_ac_10_4_a_star_in_a_rule_matches_any_text() {
    let rules = |text: &str| {
        // The seed rules would match too, so start from none.
        let mut data = Transactions::default();
        no_rules(&mut data);
        let groceries = category(&data, "Groceries");
        data.add_rule(text, groceries).unwrap();
        data
    };
    let hit = |data: &Transactions, counterparty: &str, description: &str| {
        categorise(data.rules(), day("15-10-2026"), counterparty, description).is_some()
    };
    let trailing = rules("Albert Heijn*");
    assert!(hit(&trailing, "Albert Heijn B.V.", ""));
    assert!(hit(&trailing, "ALBERT HEIJN 1157", "Pinbetaling"));
    assert!(hit(&trailing, "Albert Heijn", ""));
    assert!(!hit(&trailing, "Jumbo 1157", "Albert Hein"));
    assert!(
        !hit(&trailing, "Der Albert Heijn", ""),
        "starts with, not contains"
    );
    // A star inside: the parts must come in that order, within one field.
    let inner = rules("Albert*1157");
    assert!(hit(&inner, "Albert Heijn 1157", ""));
    assert!(
        !hit(&inner, "Albert", "Heijn 1157"),
        "the fields are not joined"
    );
    assert!(!hit(&inner, "1157 Albert Heijn", ""));
    assert!(
        !hit(&inner, "Albert Heijn 1157 Utrecht", ""),
        "has to end with 1157"
    );
    // A star at the start, at both ends, and in the middle only.
    assert!(hit(&rules("*heijn 1157"), "Albert Heijn 1157", ""));
    assert!(!hit(&rules("*heijn 1157"), "Albert Heijn 1157 Utrecht", ""));
    assert!(hit(&rules("*heijn*1157*"), "Albert Heijn 1157 Utrecht", ""));
    assert!(!hit(&rules("Albert*Jumbo"), "Albert Heijn 1157", ""));
    // A star alone matches everything.
    assert!(hit(&rules("*"), "Anything", ""));
}

#[test]
fn test_csd032_ac_10_5_a_rule_without_a_star_is_the_exact_text() {
    let rules = |text: &str| {
        let mut data = Transactions::default();
        no_rules(&mut data);
        let groceries = category(&data, "Groceries");
        data.add_rule(text, groceries).unwrap();
        data
    };
    let hit = |data: &Transactions, counterparty: &str, description: &str| {
        categorise(data.rules(), day("15-10-2026"), counterparty, description).is_some()
    };
    let exact = rules("CA");
    assert!(hit(&exact, "CA", ""));
    assert!(hit(&exact, "ca", ""), "ignoring case");
    assert!(hit(&exact, "  Ca ", ""), "ignoring surrounding spaces");
    assert!(hit(&exact, "Other", "CA"), "or the description as a whole");
    for other in [
        "CAFE",
        "ACA",
        "CA 1157",
        "Example CA",
        "Example CA Town",
        "C A",
    ] {
        assert!(!hit(&exact, other, ""), "{other} is not exactly CA");
        assert!(!hit(&exact, "", other), "{other} is not exactly CA");
    }
    // Only with a star can it reach other texts.
    assert!(hit(&rules("CA*"), "CAFE", ""));
    assert!(!hit(&rules("CA*"), "ACA", ""));
    assert!(hit(&rules("*CA"), "ACA", ""));
    assert!(!hit(&rules("*CA"), "CAFE", ""));
    assert!(hit(&rules("*CA*"), "Example CA Town", ""));
    assert!(!hit(&rules("*CA*"), "C A", ""));
}

#[test]
fn test_csd032_ac_27_1_a_rule_counts_the_transactions_it_categorises() {
    let mut data = Transactions::default();
    no_rules(&mut data);
    let groceries = category(&data, "Groceries");
    let leisure = category(&data, "Leisure");
    data.add_rule("Example Market*", groceries).unwrap();
    data.add_rule("Example C*", leisure).unwrap();
    data.add_rule("Nothing Matches This", leisure).unwrap();
    let rows = [
        simple(
            "30-10-2026",
            "Afschrijvingen",
            "9,00",
            "Example Market One",
            "R1",
        ),
        simple(
            "29-10-2026",
            "Afschrijvingen",
            "9,00",
            "Example Market Two",
            "R2",
        ),
        simple("28-10-2026", "Afschrijvingen", "9,00", "Example Cafe", "R3"),
        simple(
            "27-10-2026",
            "Afschrijvingen",
            "9,00",
            "Example Bakery",
            "R4",
        ),
        simple("26-10-2026", "Afschrijvingen", "9,00", "Other Place", "R5"),
    ];
    import_to_first(&mut data, &export(&rows), Currency::Eur).unwrap();
    // The first match counts: "Example Market One" and "Two" belong to the first rule.
    assert_eq!(data.rule_counts(), vec![2, 1, 0]);
    // A hand-set transaction is not counted.
    let cafe = find(&data, "Example Cafe").id;
    data.set_category(cafe, Some(groceries)).unwrap();
    assert_eq!(data.rule_counts(), vec![2, 0, 0]);
}

#[test]
fn test_csd032_ac_9_10_a_rule_with_the_same_text_is_rejected() {
    let mut data = Transactions::default();
    let groceries = category(&data, "Groceries");
    let leisure = category(&data, "Leisure");
    let first = data.add_rule("Example Market", groceries).unwrap();
    let before = data.clone();
    // The same text in another case or with spaces around it, whatever the category.
    for text in ["Example Market", "  example MARKET ", "EXAMPLE MARKET"] {
        for target in [groceries, leisure] {
            assert_eq!(
                data.add_rule(text, target).unwrap_err().code(),
                "TRANSACTION_RULE_TEXT_TAKEN"
            );
        }
    }
    assert_eq!(
        data.add_rule_with_new_category(
            &RuleDraft::new("example market", RulePeriod::default()),
            "Example Sport"
        )
        .unwrap_err()
        .code(),
        "TRANSACTION_RULE_TEXT_TAKEN"
    );
    assert_eq!(
        data, before,
        "a rejected rule adds nothing, not even a category"
    );
    // Another text is fine, and editing a rule to keep its own text is too.
    let second = data.add_rule("Example Cafe", groceries).unwrap();
    assert!(data
        .edit_rule(
            first.id,
            &RuleDraft::new("EXAMPLE MARKET", RulePeriod::default()),
            leisure
        )
        .is_ok());
    assert_eq!(
        data.edit_rule(
            second.id,
            &RuleDraft::new("example market", RulePeriod::default()),
            groceries
        )
        .unwrap_err()
        .code(),
        "TRANSACTION_RULE_TEXT_TAKEN"
    );
}

#[test]
fn test_csd032_ac_9_7_a_rejected_rule_or_name_adds_neither() {
    let mut data = Transactions::default();
    let before = data.clone();
    for (text, name, code) in [
        ("", "Example Sport", "TRANSACTION_RULE_TEXT_REQUIRED"),
        ("Example Gym", "", "TRANSACTION_CATEGORY_NAME_REQUIRED"),
        (
            "Example Gym",
            "groceries",
            "TRANSACTION_CATEGORY_NAME_TAKEN",
        ),
    ] {
        let error = data
            .add_rule_with_new_category(&RuleDraft::new(text, RulePeriod::default()), name)
            .unwrap_err();
        assert_eq!(error.code(), code);
        assert_eq!(data, before, "{code}");
    }
}

#[test]
fn test_csd032_ac_32_1_a_rule_without_text_categorises_every_transaction_in_its_period() {
    let mut data = holiday_fixture();
    let leisure = category(&data, "Leisure");
    data.add_rule_with(
        &RuleDraft::new("", period("01-07-2026", "15-07-2026")),
        leisure,
    )
    .unwrap();
    assert_eq!(data.reapply_rules(ReapplyScope::AllNonManual), 2);
    assert_eq!(category_on(&data, "R1"), None, "the day before");
    assert_eq!(
        category_on(&data, "R2"),
        Some(leisure),
        "first day included"
    );
    assert_eq!(category_on(&data, "R3"), Some(leisure), "last day included");
    assert_eq!(category_on(&data, "R4"), None, "the day after");
    // Rust counts what the rule categorises.
    assert_eq!(data.rule_counts(), vec![2]);
    // Imports follow the rule too.
    import_to_first(
        &mut data,
        &export(&[simple(
            "10-07-2026",
            "Afschrijvingen",
            "5,00",
            "Other",
            "R5",
        )]),
        Currency::Eur,
    )
    .unwrap();
    assert_eq!(category_on(&data, "R5"), Some(leisure));
}

#[test]
fn test_csd032_ac_32_2_text_and_period_both_have_to_fit() {
    let mut data = holiday_fixture();
    let leisure = category(&data, "Leisure");
    data.add_rule_with(
        &RuleDraft::new("Example Hotel", period("01-07-2026", "15-07-2026")),
        leisure,
    )
    .unwrap();
    data.reapply_rules(ReapplyScope::AllNonManual);
    assert_eq!(category_on(&data, "R3"), Some(leisure));
    assert_eq!(category_on(&data, "R2"), None, "in the period, wrong text");
    // The same text outside the period does not match.
    let mut outside = holiday_fixture();
    outside
        .add_rule_with(
            &RuleDraft::new("Example Hotel", period("01-08-2026", "15-08-2026")),
            leisure,
        )
        .unwrap();
    assert_eq!(outside.reapply_rules(ReapplyScope::AllNonManual), 0);
}

#[test]
fn test_csd032_ac_32_3_one_open_end_is_allowed() {
    let leisure_rule = |from: &str, to: &str| {
        let mut data = holiday_fixture();
        let leisure = category(&data, "Leisure");
        data.add_rule_with(&RuleDraft::new("", period(from, to)), leisure)
            .unwrap();
        data.reapply_rules(ReapplyScope::AllNonManual);
        ["R1", "R2", "R3", "R4"].map(|reference| category_on(&data, reference).is_some())
    };
    assert_eq!(leisure_rule("15-07-2026", ""), [false, false, true, true]);
    assert_eq!(leisure_rule("", "01-07-2026"), [true, true, false, false]);
}

#[test]
fn test_csd032_ac_32_4_the_order_of_the_rules_decides() {
    let mut data = holiday_fixture();
    let leisure = category(&data, "Leisure");
    let shopping = category(&data, "Shopping");
    data.add_rule("Example Shop", shopping).unwrap();
    let holiday = data
        .add_rule_with(
            &RuleDraft::new("", period("01-07-2026", "15-07-2026")),
            leisure,
        )
        .unwrap();
    data.reapply_rules(ReapplyScope::AllNonManual);
    assert_eq!(
        category_on(&data, "R2"),
        Some(shopping),
        "the text rule is first"
    );
    data.move_rule(holiday.id, 0).unwrap();
    data.reapply_rules(ReapplyScope::AllNonManual);
    assert_eq!(
        category_on(&data, "R2"),
        Some(leisure),
        "the holiday rule moved to the top"
    );
    assert_eq!(
        category_on(&data, "R1"),
        Some(shopping),
        "outside the period the text rule rules"
    );
    // Hand-set categories stay.
    let id = find(&data, "Example Hotel").id;
    data.set_category(id, Some(shopping)).unwrap();
    data.reapply_rules(ReapplyScope::AllNonManual);
    assert_eq!(category_on(&data, "R3"), Some(shopping));
}

#[test]
fn test_csd032_ac_32_5_invalid_periods_are_rejected() {
    let mut data = holiday_fixture();
    let leisure = category(&data, "Leisure");
    let code = |data: &mut Transactions, text: &str, period: RulePeriod| {
        data.add_rule_with(&RuleDraft::new(text, period), leisure)
            .unwrap_err()
            .code()
    };
    assert_eq!(
        code(&mut data, "Example", period("15-07-2026", "01-07-2026")),
        "TRANSACTION_RULE_PERIOD_INVALID"
    );
    assert_eq!(
        code(&mut data, "", RulePeriod::default()),
        "TRANSACTION_RULE_TEXT_REQUIRED",
        "no text needs a period"
    );
    assert_eq!(
        code(&mut data, "  ", RulePeriod::default()),
        "TRANSACTION_RULE_TEXT_REQUIRED"
    );
    // The same day on both ends is one day.
    assert!(data
        .add_rule_with(
            &RuleDraft::new("", period("01-07-2026", "01-07-2026")),
            leisure
        )
        .is_ok());
    // Editing validates the same way.
    let rule = data.rules().first().unwrap().clone();
    assert_eq!(
        data.edit_rule(rule.id, &RuleDraft::new("", RulePeriod::default()), leisure)
            .unwrap_err()
            .code(),
        "TRANSACTION_RULE_TEXT_REQUIRED"
    );
    assert!(data
        .edit_rule(
            rule.id,
            &RuleDraft::new("", period("", "05-07-2026")),
            leisure
        )
        .is_ok());
}

#[test]
fn test_csd032_ac_32_6_the_same_text_may_have_another_period() {
    let mut data = holiday_fixture();
    let leisure = category(&data, "Leisure");
    data.add_rule_with(
        &RuleDraft::new("Example", period("01-07-2026", "15-07-2026")),
        leisure,
    )
    .unwrap();
    assert_eq!(
        data.add_rule_with(
            &RuleDraft::new("EXAMPLE", period("01-07-2026", "15-07-2026")),
            leisure
        )
        .unwrap_err()
        .code(),
        "TRANSACTION_RULE_TEXT_TAKEN"
    );
    assert!(data
        .add_rule_with(
            &RuleDraft::new("Example", period("01-08-2026", "15-08-2026")),
            leisure
        )
        .is_ok());
    assert!(
        data.add_rule("Example", leisure).is_ok(),
        "no period is a period of its own"
    );
    // A period rule does not hide the same text from the suggestions.
    assert!(
        data.rules()
            .iter()
            .filter(|rule| rule.text == "Example")
            .count()
            >= 3
    );
}

#[test]
fn test_csd032_ac_32_7_a_period_survives_the_project_file_and_is_checked_on_load() {
    let mut data = holiday_fixture();
    let leisure = category(&data, "Leisure");
    data.add_rule_with(
        &RuleDraft::new("", period("01-07-2026", "15-07-2026")),
        leisure,
    )
    .unwrap();
    data.add_rule("Example Shop", leisure).unwrap();
    let json = serde_json::to_value(data.rules()).unwrap();
    assert_eq!(json[0]["from"], "2026-07-01");
    assert_eq!(json[0]["to"], "2026-07-15");
    assert!(
        json[1].get("from").is_none(),
        "a rule without a period writes no dates"
    );
    let rules: Vec<Rule> = serde_json::from_value(json).unwrap();
    let list = data.transactions().to_vec();
    let categories = data.categories().to_vec();
    assert!(Transactions::from_parts(
        list.clone(),
        data.accounts().to_vec(),
        categories.clone(),
        rules.clone()
    )
    .is_ok());
    // An empty text needs a period, and the first day cannot be after the last.
    let mut no_text = rules.clone();
    no_text[1].text = String::new();
    assert!(Transactions::from_parts(
        list.clone(),
        data.accounts().to_vec(),
        categories.clone(),
        no_text
    )
    .is_err());
    let mut inverted = rules;
    inverted[0].from = Some(day("20-07-2026"));
    assert!(
        Transactions::from_parts(list, data.accounts().to_vec(), categories, inverted).is_err()
    );
}
