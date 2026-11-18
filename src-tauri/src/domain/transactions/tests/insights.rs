use super::*;

#[test]
fn test_csd033_ac_1_1_yearly_bars_stack_the_spending_per_category() {
    let data = insights_fixture();
    let insights = data.insights(Granularity::Yearly, month(2026, 10));
    let years: Vec<i32> = insights.years.iter().map(|bar| bar.year).collect();
    assert_eq!(
        years,
        [2024, 2025, 2026],
        "a year without spending is a zero bar"
    );
    let groceries = category(&data, "Groceries").to_string();
    let last = insights.years.last().unwrap();
    assert!(last.is_partial);
    assert_eq!(last.month, None);
    assert_eq!(
        last.total,
        money("22.50"),
        "the credit of 2,500.00 is not spending"
    );
    assert_eq!(last.income, money("2500.00"));
    assert_eq!(last.marker.as_ref().unwrap().value, money("2500.00"));
    assert_eq!(last.transaction_count, 4, "three debits and a credit");
    let amounts: Vec<(&str, Money)> = last
        .segments
        .iter()
        .map(|segment| (segment.category.as_str(), segment.amount))
        .collect();
    assert_eq!(
        amounts,
        [
            (groceries.as_str(), money("15.00")),
            ("uncategorized", money("7.50"))
        ]
    );
    let middle = insights.years.get(1).unwrap();
    assert!(middle.segments.is_empty() && middle.pieces.is_empty());
    assert_eq!(middle.total, money("0.00"));
    assert!(!insights.years.first().unwrap().is_partial);
}

#[test]
fn test_csd033_ac_1_2_monthly_bars_run_from_the_first_month_with_spending_to_the_current_month() {
    let data = insights_fixture();
    let insights = data.insights(Granularity::Monthly, month(2026, 12));
    let first = insights.years.first().unwrap();
    assert_eq!((first.year, first.month), (2024, Some(12)));
    assert_eq!(insights.years.len(), 25);
    let last = insights.years.last().unwrap();
    assert_eq!(
        (last.year, last.month, last.is_partial),
        (2026, Some(12), true)
    );
    let october = insights
        .years
        .iter()
        .find(|bar| (bar.year, bar.month) == (2026, Some(10)))
        .unwrap();
    assert_eq!(october.total, money("22.50"));
    assert!(!october.is_partial);
    assert_eq!(first.total, money("20.00"));
    let january = insights
        .years
        .iter()
        .find(|bar| (bar.year, bar.month) == (2025, Some(1)))
        .unwrap();
    assert_eq!(january.total, money("0.00"));
}

#[test]
fn test_csd033_ac_2_1_the_monthly_series_holds_at_most_36_months_and_older_ones_have_no_slot() {
    let mut data = insights_fixture();
    let old = simple("15-01-2020", "Afschrijvingen", "99,00", "Example Old", "R9");
    import_to_first(&mut data, &export(&[old]), Currency::Eur).unwrap();
    let insights = data.insights(Granularity::Monthly, month(2026, 6));
    // The last month is 2026-10 (the latest debit), so the series starts 35 months before it.
    assert_eq!(insights.years.len(), 36);
    let first = insights.years.first().unwrap();
    assert_eq!((first.year, first.month), (2023, Some(11)));
    let last = insights.years.last().unwrap();
    assert_eq!((last.year, last.month), (2026, Some(10)));
}

#[test]
fn test_csd033_ac_3_1_a_month_after_the_current_one_gets_a_slot() {
    let mut data = with_rules(&[]);
    let line = simple("15-03-2027", "Afschrijvingen", "9,00", "Example", "R1");
    import_to_first(&mut data, &export(&[line]), Currency::Eur).unwrap();
    let insights = data.insights(Granularity::Monthly, month(2026, 12));
    let last = insights.years.last().unwrap();
    assert_eq!(
        (last.year, last.month, last.is_partial),
        (2027, Some(3), false)
    );
}

#[test]
fn test_csd033_ac_4_2_no_transactions_no_bars() {
    for granularity in [Granularity::Yearly, Granularity::Monthly] {
        assert_eq!(
            Transactions::default()
                .insights(granularity, month(2026, 6))
                .years,
            Vec::new()
        );
    }
}

#[test]
fn test_csd033_ac_4_4_a_credit_alone_gives_an_income_marker_and_no_segments() {
    let mut data = with_rules(&[]);
    let credit = simple("15-03-2026", "Bijschrijvingen", "9,00", "Example", "R1");
    import_to_first(&mut data, &export(&[credit]), Currency::Eur).unwrap();
    for granularity in [Granularity::Yearly, Granularity::Monthly] {
        let insights = data.insights(granularity, month(2026, 6));
        let bar = insights
            .years
            .iter()
            .find(|bar| bar.income == money("9.00"))
            .unwrap();
        assert!(bar.segments.is_empty() && bar.pieces.is_empty());
        assert_eq!(bar.total, money("0.00"));
        assert_eq!(bar.marker.as_ref().unwrap().value, money("9.00"));
        assert_eq!(bar.transaction_count, 1);
    }
}

#[test]
fn test_csd033_ac_4_5_a_bar_without_credits_has_no_marker() {
    let data = insights_fixture();
    let insights = data.insights(Granularity::Yearly, month(2026, 10));
    let older = insights.years.first().unwrap();
    assert_eq!(older.income, money("0.00"));
    assert_eq!(older.marker, None);
}

#[test]
fn test_csd033_ac_4_6_the_axis_reaches_the_income_marker() {
    let data = insights_fixture();
    let insights = data.insights(Granularity::Yearly, month(2026, 10));
    let top = insights.axis.ticks.last().unwrap().value;
    assert!(top >= money("2500.00"), "{top}");
}

#[test]
fn test_csd033_ac_6_1_the_legend_has_the_categories_in_order_and_uncategorized_last() {
    let data = Transactions::default();
    let legend = data
        .insights(Granularity::Yearly, month(2026, 6))
        .categories;
    assert_eq!(legend.len(), 15);
    assert_eq!(legend.first().unwrap().name, "Salary");
    let last = legend.last().unwrap();
    assert_eq!(
        (last.id.as_str(), last.name.as_str(), last.colour_slot.get()),
        ("uncategorized", "Uncategorized", 7)
    );
    let slots: Vec<u8> = legend
        .iter()
        .map(|category| category.colour_slot.get())
        .collect();
    assert_eq!(
        &slots[..9],
        &[0, 1, 2, 3, 4, 5, 6, 7, 0],
        "the colours repeat after eight categories"
    );
}

#[test]
fn test_csd033_ac_6_2_a_category_keeps_its_colour_while_others_are_added() {
    let mut data = Transactions::default();
    let before = data
        .insights(Granularity::Yearly, month(2026, 6))
        .categories;
    data.add_category("Example Pets").unwrap();
    let after = data
        .insights(Granularity::Yearly, month(2026, 6))
        .categories;
    assert_eq!(&after[..14], &before[..14]);
}
