# ADR-035 — Ambxst as Behavioral Reference, Not Port Target

- **Status:** Accepted
- **Date:** 2026-09-11
- **Decision owners:** Project maintainers
- **Related gates:** `G03`, `G07`, `G17`

---

## Context

This project is a Rust-first Linux desktop shell targeting Wayland and Hyprland initially. The architecture must support shell-specific behavior without allowing the UI toolkit, compositor, protocol libraries, or Linux service implementations to become the application architecture.

This ADR records one boundary that would be expensive to change implicitly later.

---

## Decision

Use Ambxst as a reference for shell capabilities, UX patterns, per-output composition, notch behavior, service separation and shared UI concepts. Do not translate its QML/Quickshell architecture one-to-one into Rust.

The implementation must follow the project's `invariants.md`. Any intentional exception requires a superseding ADR or an explicit amendment to this record.

---

## Decision drivers

- Reuse proven UX ideas without importing framework-specific structure.
- Keep architecture aligned with Rust/Wayland/GPUI constraints.

---

## Alternatives considered

- No separate alternative is selected at this stage; implementation details remain open inside the decision boundary.

Alternatives are not permanently prohibited. They may be revisited when a gate or measurement invalidates the current assumptions.

---

## Consequences

- Some Ambxst implementation choices must be revalidated independently.

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

This ADR is validated through: `G03`, `G07`, `G17`.

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

Use Ambxst as a reference for shell capabilities, UX patterns, per-output composition, notch behavior, service separation and shared UI concepts. Do not translate its QML/Quickshell architecture one-to-one into Rust.

The goal is to keep the shell architecture explicit, testable, and replaceable at its real boundaries without turning the project into a generic framework.
