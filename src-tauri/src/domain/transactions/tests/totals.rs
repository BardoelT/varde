use super::*;

/// 2026: a salary credit, groceries, a refund and a debit that cancel out in March for Leisure, an
/// uncategorized debit; 2025: one uncategorized debit.
fn grid_fixture() -> Transactions {
    let mut data = with_rules(&[
        ("Example Market", "Groceries"),
        ("Example Employer", "Salary"),
        ("Example Gym", "Leisure"),
    ]);
    let rows = [
        simple(
            "28-10-2026",
            "Bijschrijvingen",
            "2500,00",
            "Example Employer",
            "R1",
        ),
        simple(
            "30-10-2026",
            "Afschrijvingen",
            "10,00",
            "Example Market",
            "R2",
        ),
        simple(
            "15-10-2026",
            "Afschrijvingen",
            "13,97",
            "Example Market",
            "R3",
        ),
        simple(
            "03-03-2026",
            "Bijschrijvingen",
            "10,00",
            "Example Gym",
            "R4",
        ),
        simple("20-03-2026", "Afschrijvingen", "10,00", "Example Gym", "R5"),
        simple(
            "02-10-2026",
            "Afschrijvingen",
            "7,50",
            "Example Other",
            "R6",
        ),
        simple(
            "30-12-2025",
            "Afschrijvingen",
            "100,00",
            "Example Other",
            "R7",
        ),
    ];
    import_to_first(&mut data, &export(&rows), Currency::Eur).unwrap();
    data
}

fn nets(months: &[Option<Money>]) -> Vec<Option<String>> {
    months
        .iter()
        .map(|net| net.map(|net| net.to_string()))
        .collect()
}

#[test]
fn test_csd032_ac_24_7_the_grid_holds_the_net_per_category_and_month() {
    let data = grid_fixture();
    let totals = data.yearly_totals();
    let year = totals.first().unwrap();
    assert_eq!(year.year, 2026);
    let groceries = year
        .categories
        .iter()
        .find(|row| row.name == "Groceries")
        .unwrap();
    assert_eq!(groceries.id, category(&data, "Groceries").to_string());
    assert_eq!(groceries.months.len(), 12);
    assert_eq!(
        groceries.months.get(9).copied().flatten(),
        Some(money("-23.97"))
    );
    assert_eq!(groceries.total, money("-23.97"));
    let salary = year
        .categories
        .iter()
        .find(|row| row.name == "Salary")
        .unwrap();
    assert_eq!(
        salary.months.get(9).copied().flatten(),
        Some(money("2500.00"))
    );
    assert_eq!(salary.total, money("2500.00"));
    assert_eq!(year.months.len(), 12);
    // October: 2,500.00 - 23.97 - 7.50.
    assert_eq!(
        year.months.get(9).copied().flatten(),
        Some(money("2468.53"))
    );
    assert_eq!(
        year.total, year.net,
        "the Total cell equals the net of the header"
    );
    assert_eq!(year.net, money("2468.53"));
    let categories_total = Money::saturating_sum(year.categories.iter().map(|row| row.total));
    assert_eq!(categories_total, year.net, "the rows add up to the net");
}

#[test]
fn test_csd032_ac_24_8_a_category_has_a_row_when_it_has_any_transaction_in_the_year() {
    let mut data = with_rules(&[("Example Refund Shop", "Leisure")]);
    let rows = [simple(
        "05-02-2026",
        "Bijschrijvingen",
        "12,00",
        "Example Refund Shop",
        "R1",
    )];
    import_to_first(&mut data, &export(&rows), Currency::Eur).unwrap();
    let totals = data.yearly_totals();
    let names: Vec<&str> = totals
        .first()
        .unwrap()
        .categories
        .iter()
        .map(|row| row.name.as_str())
        .collect();
    assert_eq!(names, ["Leisure"], "a credit alone gives a row, Taxes none");
}

#[test]
fn test_csd032_ac_24_9_uncategorized_is_last_and_only_when_the_year_has_it() {
    let data = grid_fixture();
    let totals = data.yearly_totals();
    let ids = |index: usize| -> Vec<String> {
        totals
            .get(index)
            .unwrap()
            .categories
            .iter()
            .map(|row| row.id.clone())
            .collect()
    };
    assert_eq!(ids(0).last().map(String::as_str), Some("uncategorized"));
    assert_eq!(ids(1), ["uncategorized"]);
    let mut only = with_rules(&[("Example Market", "Groceries")]);
    let rows = [simple(
        "01-01-2026",
        "Afschrijvingen",
        "1,00",
        "Example Market",
        "R1",
    )];
    import_to_first(&mut only, &export(&rows), Currency::Eur).unwrap();
    assert!(only
        .yearly_totals()
        .first()
        .unwrap()
        .categories
        .iter()
        .all(|row| row.id != "uncategorized"));
}

#[test]
fn test_csd032_ac_24_10_a_cancelling_month_is_zero_and_an_empty_month_is_none() {
    let data = grid_fixture();
    let totals = data.yearly_totals();
    let leisure = totals
        .first()
        .unwrap()
        .categories
        .iter()
        .find(|row| row.name == "Leisure")
        .unwrap();
    assert_eq!(
        leisure.months.get(2).copied().flatten(),
        Some(money("0.00")),
        "March cancels out"
    );
    assert_eq!(
        leisure.months.get(3).copied().flatten(),
        None,
        "April has no transaction"
    );
    let expected = [
        None,
        None,
        Some("0.00".to_owned()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    ];
    assert_eq!(nets(&leisure.months), expected);
}

#[test]
fn test_csd032_ac_24_11_every_series_has_twelve_months_of_its_own_year() {
    let data = grid_fixture();
    let totals = data.yearly_totals();
    for year in &totals {
        assert_eq!(year.months.len(), 12);
        assert!(year.categories.iter().all(|row| row.months.len() == 12));
    }
    let older = totals.last().unwrap();
    assert_eq!(older.year, 2025);
    assert_eq!(
        older.months.get(11).copied().flatten(),
        Some(money("-100.00"))
    );
    assert_eq!(older.months.iter().flatten().count(), 1);
}
