//! Acceptance-criteria tests of CSD-015 that need the commands or the project file:
//! an invalid edit over IPC, the unsaved indicator of a valuation edit, and the files
//! that are rejected as invalid.

use std::fs;

use serde_json::{json, Value};
use tempfile::TempDir;

use crate::domain::houses::{house_note, parse_period, HouseInput, HouseValue};
use crate::domain::HouseId;
use crate::ipc_tests::harness;
use crate::storage::Project;

fn house_input(name: &str, period: &str) -> Value {
    json!({
        "name": name,
        "period": period,
        "purchase_date": "01/01/1900",
        "purchase_amount": "0.00",
    })
}

/// A project with one monthly house "Example House" and a valuation for 2026-09 of 300000.00.
fn project_with_a_valuation() -> (Project, HouseId) {
    let mut project = Project::new();
    let id = project
        .houses_mut()
        .create_house(&HouseInput {
            name: "Example House".to_owned(),
            period: Some("Monthly".to_owned()),
            purchase_date: Some("01/01/1900".to_owned()),
            purchase_amount: Some("0.00".to_owned()),
            ..HouseInput::default()
        })
        .unwrap()
        .id;
    project
        .houses_mut()
        .add_valuation(
            id,
            parse_period("2026-09").unwrap(),
            HouseValue::parse("300000.00").unwrap(),
            house_note("").unwrap(),
        )
        .unwrap();
    (project, id)
}

#[test]
fn test_csd015_ac_12_9_an_edit_is_an_unsaved_change() {
    let dir = TempDir::new().unwrap();
    let (mut project, id) = project_with_a_valuation();
    project.save_as(dir.path().join("example-a.varde")).unwrap();
    assert!(!project.status().unsaved_changes);
    project
        .houses_mut()
        .update_valuation(
            id,
            parse_period("2026-09").unwrap(),
            parse_period("2026-08").unwrap(),
            HouseValue::parse("300000.00").unwrap(),
            house_note("").unwrap(),
        )
        .unwrap();
    assert!(project.status().unsaved_changes);
}

#[test]
fn test_csd015_ac_13_4_an_invalid_value_rejects_the_whole_edit_and_changes_nothing() {
    let app = harness();
    let house = app.ok(
        "create_house",
        json!({ "input": house_input("Example House", "Monthly") }),
    );
    app.ok(
        "add_house_valuation",
        json!({ "houseId": house["id"], "period": "2026-09", "value": "300000.00" }),
    );
    let before = app.ok("project_status", json!({}));
    let error = app
        .call(
            "update_house_valuation",
            json!({ "houseId": house["id"], "period": "2026-09", "newPeriod": "2026-08", "value": "-1.00", "note": "" }),
        )
        .unwrap_err();
    assert_eq!(error["code"], "HOUSE_VALUATION_VALUE_NEGATIVE");
    let rows = app.ok("list_house_valuations", json!({}));
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(rows[0]["period"], "2026-09");
    assert_eq!(rows[0]["value"], "300000.00");
    assert_eq!(
        app.ok("project_status", json!({})),
        before,
        "the indicator did not change"
    );
}

/// The text of a valid project file with one house, to damage in a test.
fn valid_file(dir: &TempDir) -> (std::path::PathBuf, Value) {
    let (mut project, _) = project_with_a_valuation();
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();
    let value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    (path, value)
}

fn open_code(path: &std::path::Path, value: &Value) -> &'static str {
    fs::write(path, serde_json::to_string(value).unwrap()).unwrap();
    Project::new().open(path).unwrap_err().code()
}

#[test]
fn test_csd015_ac_27_1_a_file_without_the_houses_section_is_invalid() {
    let dir = TempDir::new().unwrap();
    let (path, mut value) = valid_file(&dir);
    // The undamaged file opens.
    Project::new().open(&path).unwrap();
    value.as_object_mut().unwrap().remove("houses");
    assert_eq!(open_code(&path, &value), "PROJECT_INVALID_FILE");
}

#[test]
fn test_csd015_ac_27_2_an_unknown_field_in_a_house_is_invalid() {
    let dir = TempDir::new().unwrap();
    let (path, mut value) = valid_file(&dir);
    value["houses"]["houses"][0]["colour"] = json!("red");
    assert_eq!(open_code(&path, &value), "PROJECT_INVALID_FILE");
}

#[test]
fn test_csd015_ac_27_4_a_house_without_a_purchase_date_is_invalid() {
    let dir = TempDir::new().unwrap();
    let (path, mut value) = valid_file(&dir);
    value["houses"]["houses"][0]
        .as_object_mut()
        .unwrap()
        .remove("purchase_date");
    assert_eq!(open_code(&path, &value), "PROJECT_INVALID_FILE");
}
