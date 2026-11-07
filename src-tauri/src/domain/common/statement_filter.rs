//! The filter of a statement table (CSD-028): a period range and the entities to keep.

use serde::Deserialize;

use super::YearMonth;

/// What the user filtered a statement table on. It only decides which rows are
/// shown; it never changes project data.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct StatementFilter {
    /// First month to show, itself included; no lower bound when `None`.
    #[serde(default)]
    from: Option<YearMonth>,
    /// Last month to show, itself included; no upper bound when `None`.
    #[serde(default)]
    to: Option<YearMonth>,
    /// The entities to keep, as the text of their id (a category name for
    /// expenses). Empty means every entity (REQ-4).
    #[serde(default)]
    ids: Vec<String>,
}

impl StatementFilter {
    /// Whether a row passes the filter. `month` is the month of the row, or
    /// `None` for a row whose period is a year: such a row always passes the
    /// period range (REQ-3). `entity` is the text of the id of what the row
    /// belongs to.
    #[must_use]
    pub fn matches(&self, month: Option<YearMonth>, entity: &str) -> bool {
        self.in_range(month) && self.has_entity(entity)
    }

    fn in_range(&self, month: Option<YearMonth>) -> bool {
        let Some(month) = month else {
            return true;
        };
        self.from.is_none_or(|from| from <= month) && self.to.is_none_or(|to| month <= to)
    }

    fn has_entity(&self, entity: &str) -> bool {
        self.ids.is_empty() || self.ids.iter().any(|id| id == entity)
    }
}

/// Keeps the rows that pass `filter`; every row when there is no filter.
/// `key` gives the month (`None` for a yearly row) and the entity of a row.
pub fn apply<T>(
    rows: impl IntoIterator<Item = T>,
    filter: Option<&StatementFilter>,
    key: impl Fn(&T) -> (Option<YearMonth>, String),
) -> Vec<T> {
    rows.into_iter()
        .filter(|row| {
            filter.is_none_or(|filter| {
                let (month, entity) = key(row);
                filter.matches(month, &entity)
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn month(year: i32, month: u8) -> Option<YearMonth> {
        YearMonth::new(year, month)
    }

    fn filter(from: Option<YearMonth>, to: Option<YearMonth>, ids: &[&str]) -> StatementFilter {
        StatementFilter {
            from,
            to,
            ids: ids.iter().map(|id| (*id).to_owned()).collect(),
        }
    }

    #[test]
    fn test_csd028_ac_3_1_range_includes_both_ends() {
        let range = filter(month(2026, 1), month(2026, 2), &[]);
        assert!(!range.matches(month(2025, 12), "a"));
        assert!(range.matches(month(2026, 1), "a"));
        assert!(range.matches(month(2026, 2), "a"));
        assert!(!range.matches(month(2026, 3), "a"));
    }

    #[test]
    fn test_csd028_ac_3_1_open_ended_ranges() {
        let from_only = filter(month(2026, 2), None, &[]);
        assert!(!from_only.matches(month(2026, 1), "a"));
        assert!(from_only.matches(month(2030, 1), "a"));
        let to_only = filter(None, month(2026, 1), &[]);
        assert!(to_only.matches(month(2025, 12), "a"));
        assert!(!to_only.matches(month(2026, 2), "a"));
    }

    #[test]
    fn test_csd028_ac_3_3_range_that_ends_before_it_starts_matches_nothing() {
        let inverted = filter(month(2026, 3), month(2026, 1), &[]);
        for m in 1..=12 {
            assert!(!inverted.matches(month(2026, m), "a"));
        }
    }

    #[test]
    fn test_csd028_ac_3_4_yearly_row_ignores_the_range() {
        let range = filter(month(2026, 2), month(2026, 2), &[]);
        assert!(range.matches(None, "a"));
    }

    #[test]
    fn test_csd028_ac_3_5_yearly_row_follows_the_entity() {
        let by_entity = filter(month(2026, 2), None, &["bank"]);
        assert!(by_entity.matches(None, "bank"));
        assert!(!by_entity.matches(None, "other"));
    }

    #[test]
    fn test_csd028_ac_4_1_selected_entities_only() {
        let selected = filter(None, None, &["a", "c"]);
        assert!(selected.matches(month(2026, 1), "a"));
        assert!(selected.matches(month(2026, 1), "c"));
        assert!(!selected.matches(month(2026, 1), "b"));
    }

    #[test]
    fn test_csd028_ac_4_3_nothing_selected_means_all() {
        let none = StatementFilter::default();
        assert!(none.matches(month(2026, 1), "anything"));
        assert!(none.matches(None, "anything"));
    }

    #[test]
    fn test_csd028_ac_4_4_period_and_entity_combine() {
        let both = filter(month(2026, 2), None, &["a"]);
        assert!(!both.matches(month(2026, 1), "a"));
        assert!(!both.matches(month(2026, 2), "b"));
        assert!(both.matches(month(2026, 2), "a"));
    }

    #[test]
    fn test_csd028_ac_10_2_unknown_id_matches_nothing_without_error() {
        let unknown = filter(None, None, &["no-such-id"]);
        assert!(!unknown.matches(month(2026, 1), "a"));
    }

    #[test]
    fn test_csd028_ac_10_1_no_filter_keeps_every_row() {
        let rows = vec![1, 2, 3];
        let kept = apply(rows.clone(), None, |_| (month(2026, 1), "a".to_owned()));
        assert_eq!(kept, rows);
    }

    #[test]
    fn test_csd028_ac_10_1_apply_keeps_order() {
        let rows = vec![("a", 3), ("b", 2), ("a", 1)];
        let filter = filter(None, None, &["a"]);
        let kept = apply(rows, Some(&filter), |row| {
            (month(2026, 1), row.0.to_owned())
        });
        assert_eq!(kept, vec![("a", 3), ("a", 1)]);
    }

    #[test]
    fn test_csd028_filter_reads_from_ipc_json() {
        let parsed: StatementFilter =
            serde_json::from_str(r#"{"from":"2026-01","to":null,"ids":["x"]}"#).unwrap();
        assert_eq!(parsed, filter(month(2026, 1), None, &["x"]));
        let empty: StatementFilter = serde_json::from_str("{}").unwrap();
        assert_eq!(empty, StatementFilter::default());
    }
}
