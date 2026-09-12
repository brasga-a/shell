# ADR-015 — Notification Daemon Ownership

- **Status:** Proposed
- **Date:** 2026-09-11
- **Decision owners:** Project maintainers
- **Related gates:** `G12`

---

## Context

This project is a Rust-first Linux desktop shell targeting Wayland and Hyprland initially. The architecture must support shell-specific behavior without allowing the UI toolkit, compositor, protocol libraries, or Linux service implementations to become the application architecture.

This ADR records one boundary that would be expensive to change implicitly later.

---

## Decision

The shell should own org.freedesktop.Notifications if notification history, actions, popup lifecycle and visual integration are core product features. Otherwise use an external daemon through a documented integration contract. Resolve before G12.

The implementation must follow the project's `invariants.md`. Any intentional exception requires a superseding ADR or an explicit amendment to this record.

---

## Decision drivers

- Tight UX integration favors owning the daemon.
- Security and protocol correctness require explicit ownership.

---

## Alternatives considered

- **Shell owns notification daemon**
- **External daemon remains authoritative**

Alternatives are not permanently prohibited. They may be revisited when a gate or measurement invalidates the current assumptions.

---

## Consequences

- Owning the daemon adds compatibility responsibility.

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

This ADR is validated through: `G12`.

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

The shell should own org.freedesktop.Notifications if notification history, actions, popup lifecycle and visual integration are core product features. Otherwise use an external daemon through a documented integration contract. Resolve before G12.

The goal is to keep the shell architecture explicit, testable, and replaceable at its real boundaries without turning the project into a generic framework.
