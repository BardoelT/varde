---
id: CSD-019
title: House purchase and sale
status: deprecated
owner: Thijs Bardoel
version: 1.0
last_updated: 03/11/2026
related: [CSD-000, CSD-020]
superseded_by: CSD-020
---

# CSD-019: House purchase and sale

## 1. Intent
Moved to CSD-020 ("House purchase, sale and linked debts", `docs/csd/CSD-020-debt-linked-to-house.md`). This CSD is deprecated and read only: it holds no requirements, and no code shall be built against it. The purchase and sale of a house are specified together with the debts linked to a house, because a sale settles those debts.

## 2. Scope
### 2.1 In scope
Nothing. Everything moved to CSD-020.

### 2.2 Non-goals
- NG-1: (removed in v1.0, moved to CSD-020) Every former non-goal now lives in CSD-020 section 2.2. See the changelog of CSD-020 for the old-to-new mapping.

## 4. Requirements
The former REQ-1 to REQ-14 (with their ACs) were removed in v1.0 and moved to CSD-020: REQ-1 to REQ-5 and REQ-12 to REQ-14 keep their numbers there, REQ-7 to REQ-11 became REQ-6 to REQ-10 (the net-worth part of the former AC-11.6 is in CSD-020 REQ-11), and the former REQ-6 (houses without a purchase) was dropped because every house now has a purchase.

## 7. Verification
None. See CSD-020.

## 9. Changelog
- 1.0 (03/11/2026): Deprecated and merged into CSD-020, which renumbers once; the complete mapping is in the changelog of CSD-020. This file is a pointer only.
- 0.3 (30/10/2026): The house row shows only the valuation period and Held or Sold; the purchase and sale stay in the Edit house form, and the result is no longer shown. Back to `draft` until re-approved.
- 0.2 (30/10/2026): Assets are renamed to houses. NG-4 now excludes only the cash and net-worth effect of the amounts. Back to `draft` until re-approved (GitHub issue 31).
- 0.1 (29/10/2026): initial draft, from GitHub issue 27 and the grill-me interview.
