use std::fs;

use tempfile::TempDir;

use super::*;
use crate::domain::houses::linking::{self, DebtFormInput, DebtLinksInput, SettlementChoice};
use crate::domain::houses::{house_note, parse_period, HouseInput, HouseValue};
use crate::domain::HouseId;

fn house(project: &mut Project, name: &str, period: &str) -> HouseId {
    project
        .houses_mut()
        .create_house(&HouseInput {
            name: name.to_owned(),
            period: Some(period.to_owned()),
            purchase_date: Some("01/01/1900".to_owned()),
            purchase_amount: Some("0.00".to_owned()),
            ..HouseInput::default()
        })
        .unwrap()
        .id
}

#[test]
fn test_csd015_ac_24_1_houses_survive_save_and_open_including_notes_and_slots() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    house(&mut project, "Example House", "Monthly");
    let id = house(&mut project, "Example Lodge", "Yearly");
    project
        .houses_mut()
        .add_valuation(
            id,
            parse_period("2025").unwrap(),
            HouseValue::parse("8000.00").unwrap(),
            house_note("Line one\nLine two").unwrap(),
        )
        .unwrap();
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();

    let mut fresh = Project::new();
    fresh.open(&path).unwrap();
    assert_eq!(fresh.houses(), project.houses());
    let rows = fresh.houses().all_valuations();
    assert_eq!(rows[0].1.note.as_str(), "Line one\nLine two");
    assert_eq!(rows[0].0.colour_slot.get(), 1);
    assert!(!fresh.status().unsaved_changes);

    // The section sits next to the others; the version stays 1.
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(value["format_version"], 1);
    assert_eq!(value["houses"]["houses"][1]["name"], "Example Lodge");
    assert_eq!(value["houses"]["houses"][1]["colour_slot"], 1);
    assert_eq!(value["houses"]["valuations"][0]["value"], "8000.00");
}

#[test]
fn test_csd015_ac_24_3_a_zero_value_survives_the_round_trip() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    let id = house(&mut project, "Example House", "Monthly");
    project
        .houses_mut()
        .add_valuation(
            id,
            parse_period("2026-09").unwrap(),
            HouseValue::parse("0.00").unwrap(),
            house_note("").unwrap(),
        )
        .unwrap();
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();
    let mut fresh = Project::new();
    fresh.open(&path).unwrap();
    let rows = fresh.houses().all_valuations();
    assert_eq!(rows[0].1.value.value().to_string(), "0.00");
}

#[test]
fn test_csd015_ac_24_4_the_other_sections_are_untouched() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    house(&mut project, "Example House", "Monthly");
    project
        .bank_accounts_mut()
        .create_account(&crate::domain::bank_accounts::BankAccountInput {
            name: "Example Current Account".to_owned(),
            period: None,
        })
        .unwrap();
    project
        .investments_mut()
        .create_account(&crate::domain::InvestmentAccountInput {
            name: "Example Brokerage".to_owned(),
            period: None,
        })
        .unwrap();
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();
    let mut fresh = Project::new();
    fresh.open(&path).unwrap();
    assert_eq!(fresh.houses(), project.houses());
    assert_eq!(fresh.bank_accounts(), project.bank_accounts());
    assert_eq!(fresh.investments(), project.investments());
}

#[test]
fn test_csd015_ac_27_3_a_damaged_section_is_an_invalid_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    // A valuation of a house that does not exist.
    fs::write(
        &path,
        r#"{"format_version":1,"income":{"jobs":[],"paychecks":[]},
            "houses":{"houses":[],"valuations":[
            {"house_id":"671166f4-5827-4794-bcdc-54d7824fb650","period":"2026-09","value":"1.00"}]}}"#,
    )
    .unwrap();
    let mut project = Project::new();
    let error = project.open(&path).unwrap_err();
    assert_eq!(error.code(), "PROJECT_INVALID_FILE");
}

#[test]
fn test_csd020_ac_33_1_links_and_settlements_survive_the_round_trip() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    let loan = project
        .debts_mut()
        .create_debt(&crate::domain::debts::DebtInput {
            name: "Example Loan".to_owned(),
            period: None,
        })
        .unwrap()
        .id;
    project
        .debts_mut()
        .add_statement(
            loan,
            crate::domain::debts::parse_period("2026-06").unwrap(),
            crate::domain::debts::DebtBalance::parse("3500.00").unwrap(),
            crate::domain::debts::debt_note("Mid-June").unwrap(),
        )
        .unwrap();
    let input = HouseInput {
        name: "Example House".to_owned(),
        purchase_date: Some("15/03/2024".to_owned()),
        purchase_amount: Some("250000.00".to_owned()),
        ..HouseInput::default()
    };
    let (houses, debts) = project.houses_and_debts_mut();
    let house = linking::create_house(houses, debts, &input, &DebtLinksInput::default())
        .unwrap()
        .id;
    let (houses, debts) = project.houses_and_debts_mut();
    linking::update_debt(
        houses,
        debts,
        loan,
        &DebtFormInput {
            debt: crate::domain::debts::DebtInput {
                name: "Example Loan".to_owned(),
                period: None,
            },
            category: Some("Loan".to_owned()),
            house_id: Some(house.to_string()),
            settlement: None,
        },
    )
    .unwrap();
    let sold = HouseInput {
        sell_date: Some("10/06/2026".to_owned()),
        sell_amount: Some("300000.00".to_owned()),
        ..input
    };
    let settle = DebtLinksInput {
        settlements: vec![SettlementChoice {
            debt_id: loan.to_string(),
            outcome: "Cancelled".to_owned(),
        }],
        ..DebtLinksInput::default()
    };
    let (houses, debts) = project.houses_and_debts_mut();
    linking::update_house(houses, debts, house, &sold, &settle).unwrap();
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();
    let mut reopened = Project::new();
    reopened.open(&path).unwrap();
    assert_eq!(reopened.debts(), project.debts());
    assert_eq!(reopened.debts().house_of(loan), Some(house));
    assert!(reopened.debts().is_settled(loan));
}

#[test]
fn test_csd020_ac_33_2_a_damaged_link_is_an_invalid_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    fs::write(
        &path,
        r#"{"format_version":1,"income":{"jobs":[],"paychecks":[]},
            "debts":{"debts":[{"id":"671166f4-5827-4794-bcdc-54d7824fb650",
            "name":"Example Loan","period":"Monthly","colour_slot":0,
            "house_id":"11111111-1111-4111-8111-111111111111"}],
            "statements":[]}}"#,
    )
    .unwrap();
    let mut project = Project::new();
    let error = project.open(&path).unwrap_err();
    assert_eq!(error.code(), "PROJECT_INVALID_FILE");
}

#[test]
fn test_csd020_ac_33_2_a_link_on_a_debt_of_category_other_is_an_invalid_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    fs::write(
        &path,
        r#"{"format_version":1,"income":{"jobs":[],"paychecks":[]},
            "houses":{"houses":[{"id":"11111111-1111-4111-8111-111111111111",
            "name":"Example House","period":"Monthly","colour_slot":0}],
            "valuations":[]},
            "debts":{"debts":[{"id":"671166f4-5827-4794-bcdc-54d7824fb650",
            "name":"Example Loan","period":"Monthly","colour_slot":0,
            "category":"Other","house_id":"11111111-1111-4111-8111-111111111111"}],
            "statements":[]}}"#,
    )
    .unwrap();
    let mut project = Project::new();
    let error = project.open(&path).unwrap_err();
    assert_eq!(error.code(), "PROJECT_INVALID_FILE");
}

#[test]
fn test_csd015_a_house_change_is_an_unsaved_change() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    project.save_as(dir.path().join("example-a.varde")).unwrap();
    assert!(!project.status().unsaved_changes);
    house(&mut project, "Example House", "Monthly");
    assert!(project.status().unsaved_changes);
}

#[test]
fn test_csd020_ac_33_2_a_settlement_statement_without_a_settlement_is_an_invalid_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    fs::write(
        &path,
        r#"{"format_version":1,"income":{"jobs":[],"paychecks":[]},
            "debts":{"debts":[{"id":"671166f4-5827-4794-bcdc-54d7824fb650",
            "name":"Example Loan","period":"Monthly","colour_slot":0}],
            "statements":[{"debt_id":"671166f4-5827-4794-bcdc-54d7824fb650",
            "period":"2026-06","balance":"0.00","note":"","settlement":true}]}}"#,
    )
    .unwrap();
    let mut project = Project::new();
    let error = project.open(&path).unwrap_err();
    assert_eq!(error.code(), "PROJECT_INVALID_FILE");
}

#[test]
fn test_csd019_ac_13_4_a_house_without_a_purchase_is_an_invalid_file() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    house(&mut project, "Example House", "Monthly");
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    for field in ["purchase_date", "purchase_amount"] {
        let mut damaged = value.clone();
        damaged["houses"]["houses"][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
        let bad = dir.path().join("bad.varde");
        fs::write(&bad, damaged.to_string()).unwrap();
        let error = Project::new().open(&bad).unwrap_err();
        assert_eq!(error.code(), "PROJECT_INVALID_FILE", "{field}");
    }
    value["houses"]["houses"][0]["former_field"] = serde_json::json!("x");
    let bad = dir.path().join("bad.varde");
    fs::write(&bad, value.to_string()).unwrap();
    assert_eq!(
        Project::new().open(&bad).unwrap_err().code(),
        "PROJECT_INVALID_FILE"
    );
}

/// Saves a project with one house, lets `change` edit the file's JSON and
/// opens the result.
fn open_changed_house_file(
    change: impl FnOnce(&mut serde_json::Value),
) -> Result<(), StorageError> {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    house(&mut project, "Example House", "Monthly");
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    change(&mut value);
    fs::write(&path, value.to_string()).unwrap();
    Project::new().open(&path)
}

#[test]
fn test_csd019_ac_5_1_a_file_with_only_a_sell_date_or_only_a_sell_amount_is_invalid() {
    for (field, value) in [
        ("sell_date", serde_json::json!("2026-06-10")),
        ("sell_amount", serde_json::json!("300000.00")),
    ] {
        let error = open_changed_house_file(|file| {
            file["houses"]["houses"][0][field] = value;
        })
        .err()
        .unwrap();
        assert_eq!(error.code(), "PROJECT_INVALID_FILE", "{field}");
    }
}

#[test]
fn test_csd019_ac_5_1_a_file_with_both_sale_fields_or_none_opens() {
    assert!(open_changed_house_file(|_| {}).is_ok());
    assert!(open_changed_house_file(|file| {
        file["houses"]["houses"][0]["sell_date"] = serde_json::json!("2026-06-10");
        file["houses"]["houses"][0]["sell_amount"] = serde_json::json!("300000.00");
    })
    .is_ok());
    assert!(open_changed_house_file(|file| {
        let house = file["houses"]["houses"][0].as_object_mut().unwrap();
        house.remove("sell_date");
        house.remove("sell_amount");
    })
    .is_ok());
}

#[test]
fn test_csd015_ac_3_1_a_house_colour_slot_of_8_is_an_invalid_file() {
    let error = open_changed_house_file(|file| {
        file["houses"]["houses"][0]["colour_slot"] = serde_json::json!(8);
    })
    .err()
    .unwrap();
    assert_eq!(error.code(), "PROJECT_INVALID_FILE");
    assert!(open_changed_house_file(|file| {
        file["houses"]["houses"][0]["colour_slot"] = serde_json::json!(7);
    })
    .is_ok());
}
