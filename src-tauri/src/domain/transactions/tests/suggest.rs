use super::*;

#[test]
fn test_csd032_ac_29_2_store_numbers_and_legal_forms_are_dropped() {
    let data = many_of(&[
        ("Example Market 1157", 3),
        ("Example Market 2210", 2),
        ("Example Market B.V.", 1),
        ("Example Cafe", 4),
    ]);
    assert_eq!(
        texts(&data.suggest_rules(5).unwrap()),
        vec![("Example Market*", 6)]
    );
    assert_eq!(
        texts(&data.suggest_rules(4).unwrap()),
        vec![("Example Market*", 6), ("Example Cafe", 4)]
    );
    assert_eq!(data.suggest_rules(7).unwrap().suggestions.len(), 0);
    // Case does not split a group, and the most frequent spelling is the text.
    let cased = many_of(&[("EXAMPLE CAFE 12", 1), ("Example Cafe 3", 2)]);
    assert_eq!(
        texts(&cased.suggest_rules(1).unwrap()),
        vec![("Example Cafe*", 3)]
    );
}

#[test]
fn test_csd032_ac_29_3_the_count_is_what_the_rule_matches() {
    let mut lines = vec![
        simple("30-10-2026", "Afschrijvingen", "9,00", "Example Cafe", "R1"),
        simple(
            "29-10-2026",
            "Afschrijvingen",
            "9,00",
            "Example Cafe Amsterdam",
            "R2",
        ),
        simple(
            "28-10-2026",
            "Afschrijvingen",
            "9,00",
            "Example Cafe Amsterdam",
            "R3",
        ),
    ];
    // No counterparty, but the description holds the text.
    lines.push(row(
        "NL00EXAM0000000001",
        "27-10-2026",
        "EUR",
        "Afschrijvingen",
        "9,00",
        "",
        "Pin Example Cafe Utrecht",
        "R4",
    ));
    let mut data = imported(&lines);
    let shopping = category(&data, "Shopping");
    let manual = find(&data, "Example Cafe Amsterdam").id;
    data.set_category(manual, Some(shopping)).unwrap();
    // The group "Example Cafe Amsterdam" has two members and the rule matches exactly those two, manual included.
    let found = data.suggest_rules(1).unwrap();
    assert_eq!(texts(&found).first(), Some(&("Example Cafe Amsterdam", 2)));
    // A text without a star is the whole counterparty (or description): only "Example Cafe".
    assert_eq!(data.count_rule_matches("example cafe").unwrap(), 1);
    assert_eq!(data.count_rule_matches("Example Cafe*").unwrap(), 3);
    assert_eq!(
        data.count_rule_matches("*Example Cafe*").unwrap(),
        4,
        "the description counts too"
    );
    assert_eq!(data.count_rule_matches("Example Cafe A*").unwrap(), 2);
    assert_eq!(
        data.count_rule_matches("  ").unwrap_err().code(),
        "TRANSACTION_RULE_TEXT_REQUIRED"
    );
}

#[test]
fn test_csd032_ac_29_4_nothing_to_suggest_and_existing_rules_are_hidden() {
    let mut lines = vec![
        row(
            "NL00EXAM0000000001",
            "30-10-2026",
            "EUR",
            "Afschrijvingen",
            "9,00",
            "",
            "Example no counterparty",
            "R1",
        ),
        simple("29-10-2026", "Afschrijvingen", "9,00", "12345", "R2"),
        simple("28-10-2026", "Afschrijvingen", "9,00", "Example Cafe", "R3"),
        simple(
            "27-10-2026",
            "Afschrijvingen",
            "9,00",
            "Example Cafe 12",
            "R4",
        ),
    ];
    let mut data = imported(&lines);
    assert_eq!(
        texts(&data.suggest_rules(1).unwrap()),
        vec![("Example Cafe*", 2)]
    );
    // A rule with the same text hides the suggestion; its text with a star is another text.
    lines.clear();
    let leisure = category(&data, "Leisure");
    data.add_rule("example cafe*", leisure).unwrap();
    assert_eq!(data.suggest_rules(1).unwrap().suggestions.len(), 0);
}

#[test]
fn test_csd032_ac_29_5_sorted_and_cut_at_100() {
    let names: Vec<String> = (0..130_u8)
        .map(|n| {
            let first = char::from(b'a' + n / 26);
            let second = char::from(b'a' + n % 26);
            format!("Example Shop q{first}{second}")
        })
        .collect();
    let list: Vec<(&str, usize)> = names.iter().map(|name| (name.as_str(), 1)).collect();
    let mut data = many_of(&list);
    let found = data.suggest_rules(1).unwrap();
    // Every rule text "Example Shop xx" matches only its own transaction.
    assert_eq!(found.suggestions.len(), 100);
    assert_eq!(found.not_shown, 30);
    let listed: Vec<&str> = found.suggestions.iter().map(|s| s.text.as_str()).collect();
    let mut sorted = listed.clone();
    sorted.sort_unstable();
    assert_eq!(listed, sorted, "equal counts are ordered by text");
    // A higher count comes first.
    let more = many_of(&[("Example Big", 3), ("Example Small", 1)]);
    assert_eq!(
        texts(&more.suggest_rules(1).unwrap()),
        vec![("Example Big", 3), ("Example Small", 1)]
    );
    data.delete_all_transactions();
    assert_eq!(data.suggest_rules(1).unwrap().not_shown, 0);
}

#[test]
fn test_csd032_ac_29_6_the_most_used_category_is_suggested() {
    let mut data = many_of(&[("Example Cafe", 6), ("Example Bakery", 2)]);
    let leisure = category(&data, "Leisure");
    let shopping = category(&data, "Shopping");
    let ids: Vec<TransactionId> = data
        .transactions()
        .iter()
        .filter(|t| t.counterparty_name == "Example Cafe")
        .map(|t| t.id)
        .collect();
    for (position, id) in ids.iter().enumerate() {
        match position {
            0..=2 => data.set_category(*id, Some(leisure)).unwrap(),
            3 => data.set_category(*id, Some(shopping)).unwrap(),
            _ => {}
        }
    }
    let found = data.suggest_rules(1).unwrap();
    let cafe = found
        .suggestions
        .iter()
        .find(|s| s.text == "Example Cafe")
        .unwrap();
    assert_eq!(
        cafe.category_id,
        Some(leisure),
        "3 of the categorised 4, 2 uncategorized do not count"
    );
    let bakery = found
        .suggestions
        .iter()
        .find(|s| s.text == "Example Bakery")
        .unwrap();
    assert_eq!(bakery.category_id, None);
    // A tie goes to the category listed first.
    let tied = {
        let mut tied = many_of(&[("Example Tie", 3)]);
        let ids: Vec<TransactionId> = tied.transactions().iter().map(|t| t.id).collect();
        let (first, second) = (category(&tied, "Leisure"), category(&tied, "Shopping"));
        tied.set_category(*ids.first().unwrap(), Some(second))
            .unwrap();
        tied.set_category(*ids.last().unwrap(), Some(first))
            .unwrap();
        tied
    };
    let first_listed = tied
        .categories()
        .iter()
        .find(|c| c.id == category(&tied, "Leisure") || c.id == category(&tied, "Shopping"))
        .unwrap()
        .id;
    assert_eq!(
        tied.suggest_rules(1)
            .unwrap()
            .suggestions
            .first()
            .unwrap()
            .category_id,
        Some(first_listed)
    );
}

#[test]
fn test_csd032_ac_29_7_a_minimal_count_of_zero_is_rejected() {
    let data = Transactions::default();
    assert_eq!(
        data.suggest_rules(0).unwrap_err().code(),
        "TRANSACTION_SUGGEST_MIN_COUNT"
    );
}

#[test]
fn test_csd032_ac_29_10_adding_rules_is_all_or_nothing() {
    let mut data = with_rules(&[("Existing", "Groceries")]);
    let leisure = category(&data, "Leisure");
    let before = data.clone();
    for (rules, position, code) in [
        (
            vec![("Example Cafe", leisure), ("example cafe", leisure)],
            1,
            "TRANSACTION_RULE_TEXT_TAKEN",
        ),
        (
            vec![("Example Cafe", leisure), ("EXISTING", leisure)],
            1,
            "TRANSACTION_RULE_TEXT_TAKEN",
        ),
        (vec![("", leisure)], 0, "TRANSACTION_RULE_TEXT_REQUIRED"),
        (
            vec![("Example Cafe", CategoryId::new())],
            0,
            "TRANSACTION_RULE_CATEGORY_INVALID",
        ),
    ] {
        let rules: Vec<(String, CategoryId)> = rules
            .into_iter()
            .map(|(text, id)| (text.to_owned(), id))
            .collect();
        let (found_at, error) = data.add_rules(&rules).unwrap_err();
        assert_eq!((found_at, error.code()), (position, code));
        assert_eq!(data, before);
    }
    let rules = vec![
        ("Example Cafe".to_owned(), leisure),
        ("Example Bakery".to_owned(), leisure),
    ];
    assert_eq!(data.add_rules(&rules).unwrap().len(), 2);
    let added: Vec<&str> = data.rules().iter().map(|r| r.text.as_str()).collect();
    assert_eq!(added, vec!["Existing", "Example Cafe", "Example Bakery"]);
}

#[test]
fn test_csd032_ac_29_14_only_uncategorized_transactions_propose_a_rule_and_the_count_covers_all() {
    let mut data = many_of(&[("Example Done", 5), ("Example Part", 6), ("Example Few", 2)]);
    let leisure = category(&data, "Leisure");
    let ids_of = |data: &Transactions, name: &str| -> Vec<TransactionId> {
        data.transactions()
            .iter()
            .filter(|t| t.counterparty_name == name)
            .map(|t| t.id)
            .collect()
    };
    // "Example Done" is categorised completely, "Example Part" has one row left.
    for id in ids_of(&data, "Example Done") {
        data.set_category(id, Some(leisure)).unwrap();
    }
    for id in ids_of(&data, "Example Part").into_iter().skip(1) {
        data.set_category(id, Some(leisure)).unwrap();
    }
    let found = data.suggest_rules(5).unwrap();
    // Not "Example Done" (nothing uncategorized), and "Example Part" with the count of all 6.
    assert_eq!(texts(&found), vec![("Example Part", 6)]);
    assert_eq!(
        found.suggestions.first().unwrap().category_id,
        Some(leisure)
    );
    // The minimal count applies to that count over everything: 2 is not enough, 6 is.
    assert_eq!(texts(&data.suggest_rules(7).unwrap()), vec![]);
    assert_eq!(
        texts(&data.suggest_rules(1).unwrap()),
        vec![("Example Part", 6), ("Example Few", 2)]
    );
}
