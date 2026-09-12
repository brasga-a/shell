# ADR-022 — UI Primitive Extraction Policy

- **Status:** Accepted
- **Date:** 2026-09-11
- **Decision owners:** Project maintainers
- **Related gates:** `G14`, `G22`

---

## Context

This project is a Rust-first Linux desktop shell targeting Wayland and Hyprland initially. The architecture must support shell-specific behavior without allowing the UI toolkit, compositor, protocol libraries, or Linux service implementations to become the application architecture.

This ADR records one boundary that would be expensive to change implicitly later.

---

## Decision

Extract reusable primitives only from repeated real usage. Do not pre-build a generic UI toolkit or wrap every GPUI primitive for hypothetical portability.

The implementation must follow the project's `invariants.md`. Any intentional exception requires a superseding ADR or an explicit amendment to this record.

---

## Decision drivers

- Avoid framework-within-a-framework.
- Keep development focused on shell features.

---

## Alternatives considered

- No separate alternative is selected at this stage; implementation details remain open inside the decision boundary.

Alternatives are not permanently prohibited. They may be revisited when a gate or measurement invalidates the current assumptions.

---

## Consequences

- Some visual rewrite remains expected if GPUI is replaced.

---

## Implementation constraints

- Keep infrastructure-specific types at their owning adapter boundary.
- Preserve explicit ownership of state and lifecycle.
- Do not add abstractions solely for theoretical portability.
- Prefer event-driven integration where reliable signals/subscriptions exist.
- Keep failure of optional subsystems recoverable.
- Any performance claim that influences this ADR must be measured.

---

## Validation

This ADR is validated through: `G14`, `G22`.

A gate failure that directly contradicts the decision must reopen this ADR instead of being hidden behind permanent workarounds.

---

## Revisit when

Revisit this ADR when one of the following is true:

- a mandatory gate fails because of this decision;
- production behavior exposes a correctness, security, or lifecycle problem;
- the maintenance cost becomes materially higher than an available alternative;
- measured performance shows the decision is a bottleneck;
- the project's supported platform scope changes.

Do not revisit it merely because another implementation is aesthetically cleaner.

---

## Rationale

Extract reusable primitives only from repeated real usage. Do not pre-build a generic UI toolkit or wrap every GPUI primitive for hypothetical portability.

The goal is to keep the shell architecture explicit, testable, and replaceable at its real boundaries without turning the project into a generic framework.
