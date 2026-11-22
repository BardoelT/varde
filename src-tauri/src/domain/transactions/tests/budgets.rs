use super::*;

/// A project with debits: Groceries 150.00 and Leisure 120.00 and Uncategorized
/// 30.00 in 2026-10, Shopping 45.00 and a refund (credit) of Groceries 20.00.
fn spending_fixture() -> Transactions {
    let rows = [
        simple(
            "05-10-2026",
            "Afschrijvingen",
            "150,00",
            "Example Grocer",
            "R1",
        ),
        simple(
            "06-10-2026",
            "Afschrijvingen",
            "120,00",
            "Example Fun",
            "R2",
        ),
        simple(
            "07-10-2026",
            "Afschrijvingen",
            "30,00",
            "Example Unknown",
            "R3",
        ),
        simple(
            "08-10-2026",
            "Afschrijvingen",
            "45,00",
            "Example Store",
            "R4",
        ),
        simple(
            "09-10-2026",
            "Bijschrijvingen",
            "20,00",
            "Example Grocer Refund",
            "R5",
        ),
        simple(
            "31-10-2026",
            "Afschrijvingen",
            "10,00",
            "Example Late",
            "R6",
        ),
        simple(
            "01-11-2026",
            "Afschrijvingen",
            "20,00",
            "Example Early",
            "R7",
        ),
    ];
    let mut data = Transactions::default();
    no_rules(&mut data);
    import_to_first(&mut data, &export(&rows), Currency::Eur).unwrap();
    let set = |data: &mut Transactions, name: &str, category_name: &str| {
        let id = find(data, name).id;
        let category = category(data, category_name);
        data.set_category(id, Some(category)).unwrap();
    };
    set(&mut data, "Example Grocer", "Groceries");
    set(&mut data, "Example Fun", "Leisure");
    set(&mut data, "Example Store", "Shopping");
    set(&mut data, "Example Grocer Refund", "Groceries");
    set(&mut data, "Example Late", "Groceries");
    set(&mut data, "Example Early", "Groceries");
    data
}

fn budget_of(data: &Transactions, name: &str, at: YearMonth) -> Option<Money> {
    let id = category(data, name);
    data.budgets()
        .iter()
        .find(|budget| budget.category_id == id)
        .and_then(|budget| budget.amount_in(at))
}

fn set(data: &mut Transactions, name: &str, at: YearMonth, amount: &str, scope: BudgetScope) {
    let id = category(data, name);
    data.set_budget(id, at, money(amount), scope).unwrap();
}

fn remove(data: &mut Transactions, name: &str, at: YearMonth, scope: BudgetScope) {
    let id = category(data, name);
    data.remove_budget(id, at, scope).unwrap();
}

fn row<'a>(view: &'a BudgetsView, name: &str) -> &'a BudgetRow {
    view.rows.iter().find(|row| row.name == name).unwrap()
}

#[test]
fn test_csd036_ac_3_1_a_month_with_budgets() {
    let mut data = spending_fixture();
    set(
        &mut data,
        "Groceries",
        month(2026, 10),
        "400.00",
        BudgetScope::All,
    );
    set(
        &mut data,
        "Leisure",
        month(2026, 10),
        "100.00",
        BudgetScope::All,
    );
    let view = data.budgets_view(month(2026, 10), month(2026, 10));
    let groceries = row(&view, "Groceries");
    // Groceries: 150.00 and the late debit of 10.00 on 31/10; the refund of 20.00 is ignored.
    assert_eq!(groceries.budget, Some(money("400.00")));
    assert_eq!(groceries.spent, money("160.00"));
    assert_eq!(groceries.remaining, Some(money("240.00")));
    assert!(!groceries.over);
    let leisure = row(&view, "Leisure");
    assert_eq!(
        (leisure.budget, leisure.spent, leisure.remaining),
        (
            Some(money("100.00")),
            money("120.00"),
            Some(money("-20.00"))
        )
    );
    assert!(leisure.over);
    assert_eq!(leisure.bar_percent.as_deref(), Some("100.00"));
    let shopping = row(&view, "Shopping");
    assert_eq!(shopping.budget, None);
    assert_eq!(shopping.spent, money("45.00"));
    assert_eq!(
        (shopping.remaining, shopping.bar_percent.clone()),
        (None, None)
    );
    assert_eq!(view.not_budgeted, money("30.00"));
    assert_eq!(view.total.budget, money("500.00"));
    assert_eq!(view.total.spent, money("280.00"));
    assert_eq!(view.total.remaining, money("220.00"));
    assert!(view.is_partial);
}

#[test]
fn test_csd036_ac_3_1_every_budgetable_category_is_a_row_in_list_order() {
    let data = spending_fixture();
    let view = data.budgets_view(month(2026, 10), month(2026, 10));
    let names: Vec<&str> = view.rows.iter().map(|row| row.name.as_str()).collect();
    let expected: Vec<&str> = data
        .categories()
        .iter()
        .map(|category| category.name.as_str())
        .collect();
    assert_eq!(names, expected);
    assert!(!names.contains(&"Uncategorized"));
}

#[test]
fn test_csd036_ac_3_2_the_bar_is_the_share_spent_and_stops_at_full() {
    let mut data = spending_fixture();
    let id = find(&data, "Example Grocer").id;
    data.delete_transaction(id).unwrap();
    // Spent in Groceries in 2026-10 is now 10.00 (the late debit).
    set(
        &mut data,
        "Groceries",
        month(2026, 10),
        "40.00",
        BudgetScope::All,
    );
    let bar = |data: &Transactions| {
        row(
            &data.budgets_view(month(2026, 10), month(2026, 10)),
            "Groceries",
        )
        .bar_percent
        .clone()
    };
    assert_eq!(bar(&data).as_deref(), Some("25.00"));
    set(
        &mut data,
        "Groceries",
        month(2026, 10),
        "10.00",
        BudgetScope::All,
    );
    let view = data.budgets_view(month(2026, 10), month(2026, 10));
    assert_eq!(
        row(&view, "Groceries").bar_percent.as_deref(),
        Some("100.00")
    );
    assert!(
        !row(&view, "Groceries").over,
        "exactly the budget is not over"
    );
    set(
        &mut data,
        "Groceries",
        month(2026, 10),
        "5.00",
        BudgetScope::All,
    );
    let view = data.budgets_view(month(2026, 10), month(2026, 10));
    assert_eq!(
        row(&view, "Groceries").bar_percent.as_deref(),
        Some("100.00")
    );
    assert!(row(&view, "Groceries").over);
}

#[test]
fn test_csd036_ac_3_3_a_budget_of_zero() {
    let mut data = spending_fixture();
    set(
        &mut data,
        "Leisure",
        month(2026, 11),
        "0.00",
        BudgetScope::All,
    );
    let view = data.budgets_view(month(2026, 11), month(2026, 11));
    let leisure = row(&view, "Leisure");
    assert_eq!(
        (leisure.budget, leisure.spent, leisure.remaining),
        (Some(money("0.00")), money("0.00"), Some(money("0.00")))
    );
    assert_eq!(leisure.bar_percent.as_deref(), Some("0.00"));
    assert!(!leisure.over);
    let view = data.budgets_view(month(2026, 10), month(2026, 10));
    let leisure = row(&view, "Leisure");
    assert_eq!(leisure.remaining, Some(money("-120.00")));
    assert!(leisure.over);
}

#[test]
fn test_csd036_ac_3_4_a_month_without_transactions() {
    let mut data = spending_fixture();
    set(
        &mut data,
        "Groceries",
        month(2026, 10),
        "400.00",
        BudgetScope::All,
    );
    let view = data.budgets_view(month(2026, 12), month(2026, 12));
    let groceries = row(&view, "Groceries");
    assert_eq!(
        (groceries.budget, groceries.spent, groceries.remaining),
        (Some(money("400.00")), money("0.00"), Some(money("400.00")))
    );
    assert_eq!(view.not_budgeted, money("0.00"));
}

#[test]
fn test_csd036_ac_4_1_credits_are_ignored() {
    let data = spending_fixture();
    let view = data.budgets_view(month(2026, 10), month(2026, 10));
    assert_eq!(row(&view, "Groceries").spent, money("160.00"));
}

#[test]
fn test_csd036_ac_4_2_the_month_comes_from_the_date() {
    let data = spending_fixture();
    let october = data.budgets_view(month(2026, 10), month(2026, 10));
    let november = data.budgets_view(month(2026, 11), month(2026, 10));
    assert_eq!(row(&october, "Groceries").spent, money("160.00"));
    assert_eq!(row(&november, "Groceries").spent, money("20.00"));
    assert!(!november.is_partial);
}

#[test]
fn test_csd036_ac_5_1_later_changes_win() {
    let mut data = spending_fixture();
    set(
        &mut data,
        "Groceries",
        month(2026, 10),
        "400.00",
        BudgetScope::All,
    );
    set(
        &mut data,
        "Groceries",
        month(2026, 10),
        "450.00",
        BudgetScope::From,
    );
    assert_eq!(
        budget_of(&data, "Groceries", month(2026, 9)),
        Some(money("400.00"))
    );
    assert_eq!(
        budget_of(&data, "Groceries", month(2026, 10)),
        Some(money("450.00"))
    );
    assert_eq!(
        budget_of(&data, "Groceries", month(2027, 3)),
        Some(money("450.00"))
    );
}

#[test]
fn test_csd036_ac_5_2_no_budget_until_one_is_set() {
    let data = spending_fixture();
    assert_eq!(budget_of(&data, "Groceries", month(2026, 10)), None);
    assert_eq!(data.budgets().len(), 0);
}

#[test]
fn test_csd036_ac_6_2_only_this_month() {
    let mut data = spending_fixture();
    set(
        &mut data,
        "Groceries",
        month(2026, 1),
        "400.00",
        BudgetScope::All,
    );
    set(
        &mut data,
        "Groceries",
        month(2026, 10),
        "500.00",
        BudgetScope::Month,
    );
    assert_eq!(
        budget_of(&data, "Groceries", month(2026, 9)),
        Some(money("400.00"))
    );
    assert_eq!(
        budget_of(&data, "Groceries", month(2026, 10)),
        Some(money("500.00"))
    );
    assert_eq!(
        budget_of(&data, "Groceries", month(2026, 11)),
        Some(money("400.00"))
    );
}

#[test]
fn test_csd036_ac_6_3_from_this_month_on_replaces_later_amounts() {
    let mut data = spending_fixture();
    set(
        &mut data,
        "Groceries",
        month(2026, 1),
        "400.00",
        BudgetScope::All,
    );
    set(
        &mut data,
        "Groceries",
        month(2026, 12),
        "700.00",
        BudgetScope::Month,
    );
    set(
        &mut data,
        "Groceries",
        month(2026, 10),
        "450.00",
        BudgetScope::From,
    );
    let at = |year, m| budget_of(&data, "Groceries", month(year, m));
    assert_eq!(at(2026, 9), Some(money("400.00")));
    for later in [(2026, 10), (2026, 11), (2026, 12), (2027, 1)] {
        assert_eq!(at(later.0, later.1), Some(money("450.00")), "{later:?}");
    }
}

#[test]
fn test_csd036_ac_6_4_all_months_replaces_everything() {
    let mut data = spending_fixture();
    set(
        &mut data,
        "Groceries",
        month(2026, 1),
        "400.00",
        BudgetScope::All,
    );
    set(
        &mut data,
        "Groceries",
        month(2026, 10),
        "450.00",
        BudgetScope::From,
    );
    set(
        &mut data,
        "Groceries",
        month(2026, 12),
        "700.00",
        BudgetScope::Month,
    );
    set(
        &mut data,
        "Groceries",
        month(2026, 10),
        "500.00",
        BudgetScope::All,
    );
    for at in [
        month(2025, 1),
        month(2026, 10),
        month(2026, 12),
        month(2030, 6),
    ] {
        assert_eq!(
            budget_of(&data, "Groceries", at),
            Some(money("500.00")),
            "{at}"
        );
    }
}

#[test]
fn test_csd036_ac_6_5_only_this_month_does_not_touch_what_follows() {
    let mut data = spending_fixture();
    set(
        &mut data,
        "Groceries",
        month(2026, 12),
        "450.00",
        BudgetScope::From,
    );
    set(
        &mut data,
        "Groceries",
        month(2026, 10),
        "300.00",
        BudgetScope::Month,
    );
    assert_eq!(budget_of(&data, "Groceries", month(2026, 9)), None);
    assert_eq!(
        budget_of(&data, "Groceries", month(2026, 10)),
        Some(money("300.00"))
    );
    assert_eq!(budget_of(&data, "Groceries", month(2026, 11)), None);
    assert_eq!(
        budget_of(&data, "Groceries", month(2026, 12)),
        Some(money("450.00"))
    );
}

#[test]
fn test_csd036_ac_6_6_and_9_2_the_amount_is_rounded() {
    assert_eq!(parse_budget_amount("400.505").unwrap(), money("400.51"));
    assert_eq!(parse_budget_amount("0.004").unwrap(), money("0.00"));
    assert_eq!(parse_budget_amount(" 12.5 ").unwrap(), money("12.50"));
}

#[test]
fn test_csd036_ac_7_1_remove_from_this_month_on() {
    let mut data = spending_fixture();
    set(
        &mut data,
        "Groceries",
        month(2026, 1),
        "400.00",
        BudgetScope::All,
    );
    remove(&mut data, "Groceries", month(2026, 10), BudgetScope::From);
    assert_eq!(
        budget_of(&data, "Groceries", month(2026, 9)),
        Some(money("400.00"))
    );
    assert_eq!(budget_of(&data, "Groceries", month(2026, 10)), None);
    assert_eq!(budget_of(&data, "Groceries", month(2027, 5)), None);
}

#[test]
fn test_csd036_ac_7_2_remove_for_this_month_only() {
    let mut data = spending_fixture();
    set(
        &mut data,
        "Groceries",
        month(2026, 1),
        "400.00",
        BudgetScope::All,
    );
    remove(&mut data, "Groceries", month(2026, 10), BudgetScope::Month);
    assert_eq!(
        budget_of(&data, "Groceries", month(2026, 9)),
        Some(money("400.00"))
    );
    assert_eq!(budget_of(&data, "Groceries", month(2026, 10)), None);
    assert_eq!(
        budget_of(&data, "Groceries", month(2026, 11)),
        Some(money("400.00"))
    );
    let view = data.budgets_view(month(2026, 10), month(2026, 10));
    assert!(!row(&view, "Groceries").single_month, "no budget, no label");
}

#[test]
fn test_csd036_ac_7_3_remove_for_all_months() {
    let mut data = spending_fixture();
    set(
        &mut data,
        "Groceries",
        month(2026, 1),
        "400.00",
        BudgetScope::All,
    );
    set(
        &mut data,
        "Groceries",
        month(2026, 10),
        "450.00",
        BudgetScope::From,
    );
    remove(&mut data, "Groceries", month(2026, 10), BudgetScope::All);
    assert_eq!(data.budgets().len(), 0);
    assert_eq!(budget_of(&data, "Groceries", month(2026, 9)), None);
}

#[test]
fn test_csd036_ac_7_4_zero_is_not_no_budget() {
    let mut data = spending_fixture();
    set(
        &mut data,
        "Leisure",
        month(2026, 10),
        "0.00",
        BudgetScope::From,
    );
    assert_eq!(
        budget_of(&data, "Leisure", month(2026, 10)),
        Some(money("0.00"))
    );
    remove(&mut data, "Leisure", month(2026, 10), BudgetScope::From);
    assert_eq!(budget_of(&data, "Leisure", month(2026, 10)), None);
}

#[test]
fn test_csd036_ac_8_1_a_single_month_amount_is_labelled() {
    let mut data = spending_fixture();
    set(
        &mut data,
        "Groceries",
        month(2026, 1),
        "400.00",
        BudgetScope::All,
    );
    set(
        &mut data,
        "Groceries",
        month(2026, 10),
        "500.00",
        BudgetScope::Month,
    );
    let single = |data: &Transactions, m| {
        row(
            &data.budgets_view(month(2026, m), month(2026, 10)),
            "Groceries",
        )
        .single_month
    };
    assert!(single(&data, 10));
    assert!(!single(&data, 11));
}

#[test]
fn test_csd036_ac_8_2_a_later_scope_replaces_the_label() {
    let mut data = spending_fixture();
    set(
        &mut data,
        "Groceries",
        month(2026, 10),
        "500.00",
        BudgetScope::Month,
    );
    set(
        &mut data,
        "Groceries",
        month(2026, 10),
        "450.00",
        BudgetScope::From,
    );
    let view = data.budgets_view(month(2026, 10), month(2026, 10));
    assert!(!row(&view, "Groceries").single_month);
}

#[test]
fn test_csd036_ac_8_a_single_month_amount_equal_to_its_neighbours_keeps_the_label() {
    let mut data = spending_fixture();
    set(
        &mut data,
        "Groceries",
        month(2026, 1),
        "400.00",
        BudgetScope::All,
    );
    set(
        &mut data,
        "Groceries",
        month(2026, 10),
        "400.00",
        BudgetScope::Month,
    );
    let view = data.budgets_view(month(2026, 10), month(2026, 10));
    assert!(row(&view, "Groceries").single_month);
}

#[test]
fn test_csd036_ac_9_1_invalid_changes_are_rejected() {
    let code = |error: DomainError| error.code();
    assert_eq!(
        code(parse_budget_amount("abc").unwrap_err()),
        "BUDGET_AMOUNT_INVALID"
    );
    assert_eq!(
        code(parse_budget_amount("-5.00").unwrap_err()),
        "BUDGET_AMOUNT_INVALID"
    );
    assert_eq!(
        code(parse_budget_amount("1000000000.01").unwrap_err()),
        "BUDGET_AMOUNT_INVALID"
    );
    assert!(parse_budget_amount("1000000000.00").is_ok());
    assert_eq!(
        code(parse_budget_month("2026-13").unwrap_err()),
        "BUDGET_MONTH_INVALID"
    );
    assert_eq!(
        code(BudgetScope::parse("sometimes").unwrap_err()),
        "BUDGET_SCOPE_INVALID"
    );
    let mut data = spending_fixture();
    let error = data
        .set_budget(
            CategoryId::new(),
            month(2026, 10),
            money("1.00"),
            BudgetScope::All,
        )
        .unwrap_err();
    assert_eq!(error.code(), "BUDGET_CATEGORY_UNKNOWN");
    assert_eq!(data.budgets().len(), 0, "nothing is stored");
}

#[test]
fn test_csd036_ac_10_1_a_rename_keeps_the_budget() {
    let mut data = spending_fixture();
    set(
        &mut data,
        "Groceries",
        month(2026, 1),
        "400.00",
        BudgetScope::All,
    );
    let id = category(&data, "Groceries");
    data.rename_category(id, "Food").unwrap();
    assert_eq!(
        budget_of(&data, "Food", month(2026, 10)),
        Some(money("400.00"))
    );
}

#[test]
fn test_csd036_ac_10_2_deleting_a_category_deletes_its_budgets() {
    let mut data = spending_fixture();
    set(
        &mut data,
        "Groceries",
        month(2026, 1),
        "400.00",
        BudgetScope::All,
    );
    set(
        &mut data,
        "Leisure",
        month(2026, 1),
        "100.00",
        BudgetScope::All,
    );
    let before = data.clone();
    let id = category(&data, "Groceries");
    data.delete_category(id).unwrap();
    assert_eq!(data.budgets().len(), 1, "the budget of Leisure stays");
    assert_ne!(data, before);
}

#[test]
fn test_csd036_ac_2_earliest_month_is_the_earliest_transaction_or_budget() {
    let mut data = spending_fixture();
    assert_eq!(
        data.budgets_view(month(2026, 10), month(2026, 10))
            .earliest_month,
        month(2026, 10)
    );
    set(
        &mut data,
        "Groceries",
        month(2026, 3),
        "400.00",
        BudgetScope::Month,
    );
    assert_eq!(
        data.budgets_view(month(2026, 10), month(2026, 10))
            .earliest_month,
        month(2026, 3)
    );
    let empty = Transactions::default();
    assert_eq!(
        empty
            .budgets_view(month(2026, 10), month(2026, 10))
            .earliest_month,
        month(2026, 10),
        "nothing to go back to"
    );
    // A future month alone never moves the earliest month past today.
    let mut future = Transactions::default();
    set(
        &mut future,
        "Groceries",
        month(2027, 3),
        "10.00",
        BudgetScope::Month,
    );
    assert_eq!(
        future
            .budgets_view(month(2026, 12), month(2026, 12))
            .earliest_month,
        month(2026, 12)
    );
}

#[test]
fn test_csd036_ac_12_4_invalid_budgets_in_the_file() {
    let data = spending_fixture();
    let groceries = category(&data, "Groceries");
    let entry = |amount: Option<&str>| FromEntry {
        month: Some(month(2026, 10)),
        amount: amount.map(money),
    };
    let budget = |from: Vec<FromEntry>| Budget {
        category_id: groceries,
        from,
        only: Vec::new(),
    };
    let rejected = |budgets: Vec<Budget>| {
        data.clone().with_budgets(budgets).unwrap_err().code() == "TRANSACTION_DATA_INVALID"
    };
    assert!(data
        .clone()
        .with_budgets(vec![budget(vec![entry(Some("400.00"))])])
        .is_ok());
    assert!(rejected(vec![Budget {
        category_id: CategoryId::new(),
        from: vec![entry(Some("1.00"))],
        only: Vec::new()
    }]));
    assert!(rejected(vec![budget(vec![entry(Some("-1.00"))])]));
    assert!(rejected(vec![budget(vec![entry(Some("1000000000.01"))])]));
    assert!(rejected(vec![budget(vec![
        entry(Some("1.00")),
        entry(Some("2.00"))
    ])]));
    assert!(rejected(vec![
        budget(vec![entry(Some("1.00"))]),
        budget(vec![entry(Some("2.00"))])
    ]));
    assert!(rejected(vec![Budget {
        category_id: groceries,
        from: Vec::new(),
        only: vec![
            OnlyEntry {
                month: month(2026, 10),
                amount: Some(money("1.00"))
            },
            OnlyEntry {
                month: month(2026, 10),
                amount: Some(money("2.00"))
            },
        ]
    }]));
}

/// The fixture of AC-20.1: Groceries 400.00 in every month, Leisure 100.00 from 2026-10 on, and
/// debits Groceries 20.00 in Jan and 150.00 in Oct, Leisure 120.00 in Oct, Shopping 45.00 (no budget).
fn year_fixture() -> Transactions {
    let rows = [
        simple(
            "05-01-2026",
            "Afschrijvingen",
            "20,00",
            "Example Grocer Jan",
            "R1",
        ),
        simple(
            "05-10-2026",
            "Afschrijvingen",
            "150,00",
            "Example Grocer",
            "R2",
        ),
        simple(
            "06-10-2026",
            "Afschrijvingen",
            "120,00",
            "Example Fun",
            "R3",
        ),
        simple(
            "08-10-2026",
            "Afschrijvingen",
            "45,00",
            "Example Store",
            "R4",
        ),
        simple(
            "09-10-2026",
            "Bijschrijvingen",
            "20,00",
            "Example Refund",
            "R5",
        ),
        simple("12-08-2024", "Afschrijvingen", "10,00", "Example Old", "R6"),
    ];
    let mut data = Transactions::default();
    no_rules(&mut data);
    import_to_first(&mut data, &export(&rows), Currency::Eur).unwrap();
    for (name, to) in [
        ("Example Grocer Jan", "Groceries"),
        ("Example Grocer", "Groceries"),
        ("Example Fun", "Leisure"),
        ("Example Store", "Shopping"),
    ] {
        let id = find(&data, name).id;
        let target = category(&data, to);
        data.set_category(id, Some(target)).unwrap();
    }
    set(
        &mut data,
        "Groceries",
        month(2026, 1),
        "400.00",
        BudgetScope::All,
    );
    set(
        &mut data,
        "Leisure",
        month(2026, 10),
        "100.00",
        BudgetScope::From,
    );
    data
}

fn year_row<'a>(view: &'a BudgetYear, name: &str) -> &'a BudgetYearRow {
    view.rows.iter().find(|row| row.name == name).unwrap()
}

fn cells_of(row: &BudgetYearRow) -> Vec<Option<Money>> {
    row.months.iter().map(|cell| cell.budget).collect()
}

#[test]
fn test_csd036_ac_20_1_a_year_with_budgets() {
    let data = year_fixture();
    let view = data.budget_year(2026, month(2026, 10));
    assert_eq!(view.year, 2026);
    let groceries = year_row(&view, "Groceries");
    assert_eq!(cells_of(groceries), vec![Some(money("400.00")); 12]);
    assert_eq!(groceries.total, Some(money("4800.00")));
    let leisure = year_row(&view, "Leisure");
    let mut expected = vec![None; 9];
    expected.extend(vec![Some(money("100.00")); 3]);
    assert_eq!(cells_of(leisure), expected);
    assert_eq!(leisure.total, Some(money("300.00")));
    let mut month_totals = vec![Some(money("400.00")); 9];
    month_totals.extend(vec![Some(money("500.00")); 3]);
    assert_eq!(view.months, month_totals);
    assert_eq!(view.total, Some(money("5100.00")));
    assert_eq!(view.figures.budget, money("5100.00"));
    assert_eq!(
        view.figures.spent,
        money("290.00"),
        "Shopping has no budget, the refund is a credit"
    );
    assert_eq!(view.figures.remaining, money("4810.00"));
    assert!(view.is_partial);
}

#[test]
fn test_csd036_ac_20_2_months_to_come_show_their_budget() {
    let data = year_fixture();
    let view = data.budget_year(2026, month(2026, 10));
    let groceries = year_row(&view, "Groceries");
    assert_eq!(
        groceries.months.get(10).unwrap().budget,
        Some(money("400.00"))
    );
    assert_eq!(
        groceries.months.get(11).unwrap().budget,
        Some(money("400.00"))
    );
    let next = data.budget_year(2027, month(2026, 10));
    assert!(!next.is_partial);
    assert_eq!(year_row(&next, "Groceries").total, Some(money("4800.00")));
}

#[test]
fn test_csd036_ac_20_3_a_single_month_cell_is_marked() {
    let mut data = year_fixture();
    set(
        &mut data,
        "Groceries",
        month(2026, 10),
        "500.00",
        BudgetScope::Month,
    );
    let view = data.budget_year(2026, month(2026, 10));
    let marks: Vec<bool> = year_row(&view, "Groceries")
        .months
        .iter()
        .map(|cell| cell.single_month)
        .collect();
    let mut expected = vec![false; 12];
    *expected.get_mut(9).unwrap() = true;
    assert_eq!(marks, expected);
}

#[test]
fn test_csd036_ac_20_4_uncategorized_is_not_a_row() {
    let data = year_fixture();
    let view = data.budget_year(2026, month(2026, 10));
    assert!(view.rows.iter().all(|row| row.name != "Uncategorized"));
    assert_eq!(view.rows.len(), data.categories().len());
}

#[test]
fn test_csd036_ac_20_5_a_budget_of_zero_is_not_a_dash() {
    let mut data = Transactions::default();
    set(
        &mut data,
        "Leisure",
        month(2026, 10),
        "0.00",
        BudgetScope::Month,
    );
    let view = data.budget_year(2026, month(2026, 10));
    let leisure = year_row(&view, "Leisure");
    assert_eq!(leisure.months.get(9).unwrap().budget, Some(money("0.00")));
    assert_eq!(leisure.months.get(8).unwrap().budget, None);
    assert_eq!(leisure.total, Some(money("0.00")));
}

#[test]
fn test_csd036_ac_20_6_a_year_without_a_budget() {
    // No budget at all, though the year has debits.
    let data = spending_fixture();
    let view = data.budget_year(2026, month(2026, 10));
    assert!(view
        .rows
        .iter()
        .all(|row| row.total.is_none() && row.months.iter().all(|cell| cell.budget.is_none())));
    assert_eq!(view.months, vec![None; 12]);
    assert_eq!(view.total, None);
    assert_eq!(
        (
            view.figures.budget,
            view.figures.spent,
            view.figures.remaining
        ),
        (money("0.00"), money("0.00"), money("0.00"))
    );
}

#[test]
fn test_csd036_ac_19_2_the_earliest_year_is_the_year_of_the_earliest_month() {
    let data = year_fixture();
    assert_eq!(data.budget_year(2026, month(2026, 10)).earliest_year, 2024);
    let empty = Transactions::default();
    assert_eq!(empty.budget_year(2026, month(2026, 10)).earliest_year, 2026);
}

#[test]
fn test_csd036_ac_22_3_an_invalid_year_is_rejected() {
    for year in ["26", "20266", "abcd", "0000", "", "-123"] {
        assert_eq!(
            parse_budget_year(year).unwrap_err().code(),
            "BUDGET_YEAR_INVALID",
            "{year}"
        );
    }
    assert_eq!(parse_budget_year("2026").unwrap(), 2026);
}
