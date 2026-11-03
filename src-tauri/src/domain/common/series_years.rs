//! The years of an Insights series (CSD-006, CSD-007, CSD-010, CSD-014,
//! CSD-016, CSD-017, CSD-018): one slot per calendar year from the earliest
//! record to the later of the current year and the latest record.

/// The most slots a series holds.
pub const MAX_SERIES_YEARS: i32 = 50;

/// The years of a series, oldest first, for the years that hold a record.
///
/// The series runs from the earliest record year up to and including the later
/// of `current_year` and the latest record year, so a year after the current
/// year gets a slot. It holds at most [`MAX_SERIES_YEARS`] years, counted back
/// from its last year; a record older than that gets no slot. The caller keeps
/// counting such a record in every total that spans all years. Without a record
/// the series is empty.
#[must_use]
pub fn series_years(record_years: impl IntoIterator<Item = i32>, current_year: i32) -> Vec<i32> {
    let (mut earliest, mut latest) = (i32::MAX, i32::MIN);
    for year in record_years {
        earliest = earliest.min(year);
        latest = latest.max(year);
    }
    if earliest > latest {
        return Vec::new();
    }
    let last = latest.max(current_year);
    let first = earliest.max(last.saturating_sub(MAX_SERIES_YEARS - 1));
    (first..=last).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_record_gives_no_years() {
        assert_eq!(series_years([], 2026).len(), 0);
    }

    #[test]
    fn runs_to_the_current_year() {
        assert_eq!(series_years([2024, 2025], 2026), vec![2024, 2025, 2026]);
        assert_eq!(series_years([2026], 2026), vec![2026]);
    }

    #[test]
    fn runs_to_the_latest_record_after_the_current_year() {
        assert_eq!(
            series_years([2024, 2027], 2026),
            vec![2024, 2025, 2026, 2027]
        );
    }

    #[test]
    fn a_series_in_the_future_only_starts_at_its_first_record() {
        assert_eq!(series_years([2028], 2026), vec![2028]);
    }

    #[test]
    fn holds_at_most_fifty_years_back_from_the_last_year() {
        let years = series_years([1950, 2026], 2026);
        assert_eq!(years.len(), 50);
        assert_eq!(years.first(), Some(&1977));
        assert_eq!(years.last(), Some(&2026));
        let years = series_years([1950, 2060], 2026);
        assert_eq!(years.first(), Some(&2011));
        assert_eq!(years.last(), Some(&2060));
    }

    #[test]
    fn fifty_years_exactly_are_all_kept() {
        let years = series_years([1977, 2026], 2026);
        assert_eq!(years.len(), 50);
        assert_eq!(years.first(), Some(&1977));
    }
}
