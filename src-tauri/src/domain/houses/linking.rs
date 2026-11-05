//! Linking debts to a house, in the debt form, and settling them when the
//! house is sold (CSD-020, CSD-026).
//!
//! A house and its debts live in two places of the project, so the operations
//! work on both and change them together or not at all (CSD-026 CON-5).

use crate::domain::money::Money;
use serde::{Deserialize, Serialize};

use crate::domain::debts::{
    ClosingInput, Debt, DebtCategory, DebtId, DebtInput, DebtSummary, Debts, OpenDebt, Outcome,
    Settlement,
};
use crate::domain::error::DomainError;
use crate::domain::series::SeriesData;

use super::{House, HouseDate, HouseId, HouseInput, HouseSummary, Houses};

/// The debt side of the Add and Edit house forms (CSD-026 REQ-1, REQ-4).
#[derive(Clone, Debug, Default, Deserialize)]
pub struct DebtLinksInput {
    /// The Repaid or Cancelled choice per debt with an open balance.
    #[serde(default)]
    pub settlements: Vec<SettlementChoice>,
    /// Confirms that undoing a sale also undoes its settlements.
    #[serde(default)]
    pub confirm_undo: bool,
}

/// One choice of the settlement prompt.
#[derive(Clone, Debug, Deserialize)]
pub struct SettlementChoice {
    /// Identifier of the debt, as sent by the frontend and checked before use.
    pub debt_id: String,
    /// `Repaid` or `Cancelled`.
    pub outcome: String,
}

struct Parsed {
    settlements: Vec<(DebtId, Outcome)>,
    confirm_undo: bool,
}

impl DebtLinksInput {
    fn parse(&self) -> Result<Parsed, DomainError> {
        let mut settlements = Vec::new();
        for choice in &self.settlements {
            settlements.push((
                DebtId::parse(&choice.debt_id)?,
                Outcome::parse(&choice.outcome)?,
            ));
        }
        Ok(Parsed {
            settlements,
            confirm_undo: self.confirm_undo,
        })
    }
}

/// Creates a house (CSD-015 REQ-1). It has no linked debts yet, so a sale
/// given at creation settles nothing (CSD-026 REQ-1).
///
/// # Errors
/// Returns the errors of `Houses::create_house` and of the link and
/// settlement rules; nothing changes on an error.
pub fn create_house(
    houses: &mut Houses,
    debts: &mut Debts,
    input: &HouseInput,
    links: &DebtLinksInput,
) -> Result<House, DomainError> {
    let parsed = links.parse()?;
    let mut new_houses = houses.clone();
    let mut new_debts = debts.clone();
    let house = new_houses.create_house(input)?;
    apply(&mut new_debts, None, &house, &parsed)?;
    *houses = new_houses;
    *debts = new_debts;
    Ok(house)
}

/// Changes a house and its linked debts together: the settlement of a new
/// sale, and the undo of a sale (CSD-026 REQ-1 to REQ-5).
///
/// # Errors
/// Returns the errors of `Houses::update_house` and of the link and settlement
/// rules; nothing changes on an error.
pub fn update_house(
    houses: &mut Houses,
    debts: &mut Debts,
    id: HouseId,
    input: &HouseInput,
    links: &DebtLinksInput,
) -> Result<House, DomainError> {
    let parsed = links.parse()?;
    let mut new_houses = houses.clone();
    let mut new_debts = debts.clone();
    let old = new_houses.account(id)?.clone();
    let house = new_houses.update_house(id, input)?;
    apply(&mut new_debts, Some(&old), &house, &parsed)?;
    *houses = new_houses;
    *debts = new_debts;
    Ok(house)
}

/// Deletes a house unless debts are linked to it (CSD-026 REQ-7).
///
/// # Errors
/// Returns `HouseNotFound` or `HouseHasLinkedDebts`.
pub fn delete_house(houses: &mut Houses, debts: &Debts, id: HouseId) -> Result<(), DomainError> {
    houses.account(id)?;
    if !debts.linked_to(id).is_empty() {
        return Err(DomainError::HouseHasLinkedDebts);
    }
    houses.delete_house(id)
}

/// The debts linked to `house_id` with an open balance at a sale on
/// `sell_date`, for the settlement prompt (CSD-026 REQ-13).
///
/// # Errors
/// Returns a date error of CSD-019.
pub fn open_debts(
    debts: &Debts,
    house_id: HouseId,
    sell_date: &str,
) -> Result<Vec<OpenDebt>, DomainError> {
    let date = HouseDate::parse(sell_date.trim())?;
    debts.open_debts(&debts.linked_to(house_id), date)
}

/// The debt form fields CSD-020 adds to those of CSD-011 (REQ-1).
#[derive(Clone, Debug, Default, Deserialize)]
pub struct DebtFormInput {
    /// The debt fields.
    #[serde(flatten)]
    pub debt: DebtInput,
    /// `Mortgage`, `Loan` or `Other`; `Other` on create and unchanged on an
    /// edit when left out.
    #[serde(default)]
    pub category: Option<String>,
    /// The house of a Mortgage or Loan; none or empty for no house.
    #[serde(default)]
    pub house_id: Option<String>,
    /// `Repaid` or `Cancelled` when the debt is linked to a house that is
    /// already sold and has an open balance (CSD-020 REQ-17).
    #[serde(default)]
    pub settlement: Option<String>,
    /// The closing fields of the form: a manual closing of any debt, an edit
    /// of an existing closing, or all empty to clear one (CSD-026 REQ-15 to
    /// REQ-17).
    #[serde(default)]
    pub closing: ClosingInput,
    /// Confirms that clearing a closing removes its settlement statement
    /// (CSD-026 REQ-16).
    #[serde(default)]
    pub confirm_undo: bool,
}

/// Creates a debt with its category and optional house (CSD-020 REQ-15,
/// REQ-16).
///
/// # Errors
/// Returns the errors of `Debts::create_debt` and the category and link rules;
/// nothing changes on an error.
pub fn create_debt(
    houses: &Houses,
    debts: &mut Debts,
    input: &DebtFormInput,
) -> Result<Debt, DomainError> {
    let mut new_debts = debts.clone();
    let id = new_debts.create_debt(&input.debt)?.id;
    apply_category_and_link(houses, &mut new_debts, id, input)?;
    apply_closing(&mut new_debts, id, input, false)?;
    *debts = new_debts;
    debts.get(id).cloned().ok_or(DomainError::DebtNotFound)
}

/// Changes a debt with its category and optional house (CSD-020 REQ-15,
/// REQ-16, CSD-026 REQ-6).
///
/// # Errors
/// Returns the errors of `Debts::update_debt` and the category and link rules;
/// nothing changes on an error.
pub fn update_debt(
    houses: &Houses,
    debts: &mut Debts,
    id: DebtId,
    input: &DebtFormInput,
) -> Result<Debt, DomainError> {
    let mut new_debts = debts.clone();
    let was_settled = new_debts.is_settled(id);
    new_debts.update_debt(id, &input.debt)?;
    apply_category_and_link(houses, &mut new_debts, id, input)?;
    apply_closing(&mut new_debts, id, input, was_settled)?;
    *debts = new_debts;
    debts.get(id).cloned().ok_or(DomainError::DebtNotFound)
}

/// Applies the closing fields of the debt form (CSD-026 REQ-15 to REQ-17).
/// `was_settled` says whether the debt had a settlement before this save. A
/// closing typed on a debt that is not closed is a manual closing, one typed
/// differently on a closed debt is edited in place and becomes manual, one
/// typed as stored changes nothing, and fields emptied on a closed debt clear
/// its closing after confirmation, whatever made it.
fn apply_closing(
    debts: &mut Debts,
    id: DebtId,
    input: &DebtFormInput,
    was_settled: bool,
) -> Result<(), DomainError> {
    let closing = input.closing.parse()?;
    match (was_settled, closing) {
        // Nothing typed and nothing stored.
        (false, None) => Ok(()),
        // The link step settled the debt in this very save (CSD-020 REQ-17):
        // a closing typed besides it is ambiguous.
        (false, Some(_)) if debts.is_settled(id) => Err(DomainError::HouseSettlementInvalid),
        (false, Some(closing)) => debts.set_closing(id, &closing),
        (true, None) if !input.confirm_undo => Err(DomainError::DebtClosingUndoUnconfirmed),
        (true, None) => debts.unsettle(id),
        // The stored closing typed again is unchanged; any other replaces it.
        (true, Some(closing)) => {
            let same = debts
                .get(id)
                .and_then(Debt::settlement)
                .is_some_and(|stored| {
                    stored.outcome == closing.outcome
                        && stored.date == closing.date
                        && stored.amount == closing.amount
                });
            if same {
                Ok(())
            } else {
                debts.set_closing(id, &closing)
            }
        }
    }
}

fn apply_category_and_link(
    houses: &Houses,
    debts: &mut Debts,
    id: DebtId,
    input: &DebtFormInput,
) -> Result<(), DomainError> {
    let category = match input.category.as_deref().map(str::trim) {
        Some(text) if !text.is_empty() => DebtCategory::parse(text)?,
        _ => debts.get(id).map(Debt::category).unwrap_or_default(),
    };
    let wanted = match input.house_id.as_deref().map(str::trim) {
        Some(text) if !text.is_empty() => Some(HouseId::parse(text)?),
        _ => None,
    };
    if wanted.is_some() && !category.can_link() {
        return Err(DomainError::DebtLinkCategoryInvalid);
    }
    let outcome = match input.settlement.as_deref().map(str::trim) {
        Some(text) if !text.is_empty() => Some(Outcome::parse(text)?),
        _ => None,
    };
    let current = debts.house_of(id);
    debts.set_category(id, category)?;
    if wanted == current {
        return if outcome.is_some() {
            Err(DomainError::HouseSettlementInvalid)
        } else {
            Ok(())
        };
    }
    let Some(house_id) = wanted else {
        debts.set_house(id, None)?;
        return if outcome.is_some() {
            Err(DomainError::HouseSettlementInvalid)
        } else {
            Ok(())
        };
    };
    let house = houses.get(house_id).ok_or(DomainError::HouseNotFound)?;
    debts.set_house(id, Some(house_id))?;
    match house.sale {
        Some(sale) => settle_on_link(debts, id, house, sale.date, outcome),
        None if outcome.is_some() => Err(DomainError::HouseSettlementInvalid),
        None => Ok(()),
    }
}

/// Settles a debt that is linked to a house sold on `date`, when it has an
/// open balance (CSD-020 REQ-17).
fn settle_on_link(
    debts: &mut Debts,
    id: DebtId,
    house: &House,
    date: HouseDate,
    outcome: Option<Outcome>,
) -> Result<(), DomainError> {
    // A debt that is closed already keeps its closing and is never asked
    // about (CSD-020 REQ-17, CSD-026 REQ-18).
    let open = if debts.is_settled(id) {
        None
    } else {
        debts.open_balance(id, date)?
    };
    match (open, outcome) {
        (Some(_), Some(outcome)) => debts.settle(id, outcome, house),
        (Some(_), None) => Err(DomainError::HouseSettlementRequired),
        (None, Some(_)) => Err(DomainError::HouseSettlementInvalid),
        (None, None) => Ok(()),
    }
}

/// The open balance of a debt at the sell date of the sold house it would be
/// linked to, `None` for a held house or without an open balance, for the
/// settlement prompt (CSD-026 REQ-13, CSD-020 REQ-17).
///
/// # Errors
/// Returns `DebtNotFound` or `HouseNotFound`.
pub fn debt_open_balance(
    houses: &Houses,
    debts: &Debts,
    debt_id: DebtId,
    house_id: HouseId,
) -> Result<Option<OpenDebt>, DomainError> {
    let house = houses.get(house_id).ok_or(DomainError::HouseNotFound)?;
    let debt = debts.get(debt_id).ok_or(DomainError::DebtNotFound)?;
    let Some(sale) = house.sale else {
        return Ok(None);
    };
    if debts.is_settled(debt_id) {
        return Ok(None);
    }
    Ok(debts
        .open_balance(debt_id, sale.date)?
        .map(|amount| OpenDebt {
            debt_id,
            debt_name: debt.name.clone(),
            amount,
        }))
}

/// Applies the debt side of a saved house. `old` is the house before the save,
/// `None` for a new house.
fn apply(
    debts: &mut Debts,
    old: Option<&House>,
    house: &House,
    parsed: &Parsed,
) -> Result<(), DomainError> {
    let was_sold = old.is_some_and(|old| !old.is_held());
    let current = debts.linked_to(house.id);
    // Only a closing by sale belongs to the sale: a manual closing neither
    // locks the sell date nor goes with an undone sale (CSD-026 REQ-4, REQ-5).
    let settled: Vec<DebtId> = current
        .iter()
        .copied()
        .filter(|id| debts.is_settled_by_sale(*id))
        .collect();
    if was_sold && !house.is_held() {
        if old.is_some_and(|old| old.sale.map(|sale| sale.date) != house.sale.map(|sale| sale.date))
            && !settled.is_empty()
        {
            return Err(DomainError::HouseSellDateLocked);
        }
        if !parsed.settlements.is_empty() {
            return Err(DomainError::HouseSettlementInvalid);
        }
        return Ok(());
    }
    if was_sold {
        if !settled.is_empty() && !parsed.confirm_undo {
            return Err(DomainError::HouseSaleUndoUnconfirmed);
        }
        for id in settled {
            debts.unsettle(id)?;
        }
    }
    match house.sale {
        Some(sale) => settle_all(debts, house, sale.date, &parsed.settlements),
        None if parsed.settlements.is_empty() => Ok(()),
        None => Err(DomainError::HouseSettlementInvalid),
    }
}

fn settle_all(
    debts: &mut Debts,
    house: &House,
    date: HouseDate,
    choices: &[(DebtId, Outcome)],
) -> Result<(), DomainError> {
    let open = debts.open_debts(&debts.linked_to(house.id), date)?;
    if choices
        .iter()
        .any(|(id, _)| !open.iter().any(|debt| debt.debt_id == *id))
    {
        return Err(DomainError::HouseSettlementInvalid);
    }
    for debt in &open {
        let (_, outcome) = choices
            .iter()
            .find(|(id, _)| *id == debt.debt_id)
            .ok_or(DomainError::HouseSettlementRequired)?;
        debts.settle(debt.debt_id, *outcome, house)?;
    }
    Ok(())
}

/// A debt linked to a house, as the Houses view shows it (CSD-026 REQ-12).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LinkedDebt {
    /// Identifier of the debt.
    pub debt_id: DebtId,
    /// Name of the debt.
    pub debt_name: String,
    /// The debt category.
    pub category: DebtCategory,
    /// The latest balance, `None` without statements.
    pub latest_balance: Option<Money>,
    /// How the debt was settled, `None` while it is open.
    pub settlement: Option<Settlement>,
}

/// A house row with its linked debts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HouseRow {
    /// The summary the row extends.
    #[serde(flatten)]
    pub summary: HouseSummary,
    /// The debts linked to the house.
    pub linked_debts: Vec<LinkedDebt>,
}

/// The houses overview with the linked debts of each house.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HousesView {
    /// The houses of the overview.
    pub houses: Vec<HouseRow>,
    /// The total of the latest values, `None` when there is nothing to total.
    pub total: Option<Money>,
}

/// A debt row with the name of its house, as the Debts view shows it
/// (CSD-020 REQ-30).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DebtRow {
    /// The summary the row extends.
    #[serde(flatten)]
    pub summary: DebtSummary,
    /// Name of the linked house, `None` when the debt is not linked.
    pub house_name: Option<String>,
}

/// The debts overview with the house of each debt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DebtsView {
    /// The debts of the overview.
    pub debts: Vec<DebtRow>,
    /// The total of the latest values, `None` when there is nothing to total.
    pub total: Option<Money>,
}

/// The houses overview (CSD-015 REQ-17) with the debts linked to each house.
#[must_use]
pub fn houses_view(houses: &Houses, debts: &Debts) -> HousesView {
    let overview = houses.overview();
    let summaries = debts.overview().debts;
    let rows = overview
        .houses
        .into_iter()
        .map(|summary| {
            let linked_debts = summaries
                .iter()
                .filter(|row| row.debt.house_id() == Some(summary.house.id))
                .map(|row| LinkedDebt {
                    debt_id: row.debt.id,
                    debt_name: row.debt.name.clone(),
                    category: row.debt.category(),
                    latest_balance: row.latest_balance,
                    settlement: row.debt.settlement().cloned(),
                })
                .collect();
            HouseRow {
                summary,
                linked_debts,
            }
        })
        .collect();
    HousesView {
        houses: rows,
        total: overview.total,
    }
}

/// The debts overview (CSD-011 REQ-17) with the name of each linked house.
#[must_use]
pub fn debts_view(debts: &Debts, houses: &Houses) -> DebtsView {
    let overview = debts.overview();
    let rows = overview
        .debts
        .into_iter()
        .map(|summary| {
            let house_name = summary
                .debt
                .house_id()
                .and_then(|id| houses.get(id))
                .map(|house| house.name.clone());
            DebtRow {
                summary,
                house_name,
            }
        })
        .collect();
    DebtsView {
        debts: rows,
        total: overview.total,
    }
}
