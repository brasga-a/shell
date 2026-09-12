# Architecture Decision Records

This directory contains the 35 initial ADRs for the Rust/Wayland Linux shell project.

The ADR set is intended to be read together with `decisions.md`, `invariants.md`, and `gates.md` from the project context.

## Index

- [ADR-001 — GPUI as the Initial Frontend](ADR-001-gpui-as-the-initial-frontend.md)
- [ADR-002 — Hexagonal Architecture for Shell Boundaries](ADR-002-hexagonal-architecture-for-shell-boundaries.md)
- [ADR-003 — Frontend Must Remain Replaceable](ADR-003-frontend-must-remain-replaceable.md)
- [ADR-004 — Wayland Layer-Shell Integration Strategy](ADR-004-wayland-layer-shell-integration-strategy.md)
- [ADR-005 — Shell Surface Topology](ADR-005-shell-surface-topology.md)
- [ADR-006 — Multi-Monitor State and Surface Ownership](ADR-006-multi-monitor-state-and-surface-ownership.md)
- [ADR-007 — Hyprland as a Compositor Adapter](ADR-007-hyprland-as-a-compositor-adapter.md)
- [ADR-008 — Compositor Port and Capability Model](ADR-008-compositor-port-and-capability-model.md)
- [ADR-009 — Shell State Model and Unidirectional Data Flow](ADR-009-shell-state-model-and-unidirectional-data-flow.md)
- [ADR-010 — Persistent Configuration Model](ADR-010-persistent-configuration-model.md)
- [ADR-011 — Async Runtime and Concurrency Model](ADR-011-async-runtime-and-concurrency-model.md)
- [ADR-012 — Linux Service Adapter Pattern](ADR-012-linux-service-adapter-pattern.md)
- [ADR-013 — D-Bus Stack](ADR-013-d-bus-stack.md)
- [ADR-014 — Audio Backend](ADR-014-audio-backend.md)
- [ADR-015 — Notification Daemon Ownership](ADR-015-notification-daemon-ownership.md)
- [ADR-016 — System Tray and StatusNotifierItem](ADR-016-system-tray-and-statusnotifieritem.md)
- [ADR-017 — Focus, Keyboard and Input-Region Coordination](ADR-017-focus-keyboard-and-input-region-coordination.md)
- [ADR-018 — Notch as a Feature Container](ADR-018-notch-as-a-feature-container.md)
- [ADR-019 — Custom Geometry Rendering](ADR-019-custom-geometry-rendering.md)
- [ADR-020 — Animation System](ADR-020-animation-system.md)
- [ADR-021 — Centralized Design System](ADR-021-centralized-design-system.md)
- [ADR-022 — UI Primitive Extraction Policy](ADR-022-ui-primitive-extraction-policy.md)
- [ADR-023 — Cargo Workspace Boundaries](ADR-023-cargo-workspace-boundaries.md)
- [ADR-024 — Error Handling and Recovery](ADR-024-error-handling-and-recovery.md)
- [ADR-025 — Logging, Diagnostics and Observability](ADR-025-logging-diagnostics-and-observability.md)
- [ADR-026 — Native Dependency Policy](ADR-026-native-dependency-policy.md)
- [ADR-027 — Application Discovery and Launcher Execution](ADR-027-application-discovery-and-launcher-execution.md)
- [ADR-028 — Icons and Asset Pipeline](ADR-028-icons-and-asset-pipeline.md)
- [ADR-029 — Text Rendering Strategy](ADR-029-text-rendering-strategy.md)
- [ADR-030 — Lockscreen Security Architecture](ADR-030-lockscreen-security-architecture.md)
- [ADR-031 — Process and Daemon Strategy](ADR-031-process-and-daemon-strategy.md)
- [ADR-032 — Compositor Compatibility Policy](ADR-032-compositor-compatibility-policy.md)
- [ADR-033 — Future Custom Renderer Exit Path](ADR-033-future-custom-renderer-exit-path.md)
- [ADR-034 — Performance Budgets and Regression Policy](ADR-034-performance-budgets-and-regression-policy.md)
- [ADR-035 — Ambxst as Behavioral Reference, Not Port Target](ADR-035-ambxst-as-behavioral-reference-not-port-target.md)

## Status conventions

- **Accepted** — current architectural direction.
- **Proposed** — intentionally unresolved pending validation.
- **Superseded** — replaced by a newer ADR.
- **Rejected** — considered and not selected.