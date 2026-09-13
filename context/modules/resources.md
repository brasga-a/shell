# Task Manager / Resources Module

## Crate

`crates/modules/resources` → package `shell-module-resources`

## Purpose

Provide a compact, real-time system resource surface inside the Notch for answering:

```text
Is the system under load?
Which resource is saturated?
Which processes are responsible?
```

The Notch module is intentionally lighter than a full task-manager application.

---

# Reference applications and architecture

## Mission Center

Mission Center is a modern Linux system monitor focused on hardware/resource visualization and process/application usage. Its value as a Luna reference is the split between overview metrics and deeper task-manager views.

Useful lesson:

```text
summary first
→ CPU / memory / disk / network / GPU
→ drill into process/application details only when needed
```

Reference:

- https://missioncenter.io/
- https://github.com/missioncenter-dev/mission-center

## Resources

GNOME Resources provides a modern task-manager experience with resource graphs, applications/processes and system information. It reinforces that sampling, aggregation and process control should be separated from presentation.

Reference:

- https://apps.gnome.org/Resources/

## GNOME System Monitor / KDE System Monitor

Traditional system monitors provide a useful boundary reference:

```text
resource overview
process table
process actions
hardware/history views
```

For Luna, only the first two belong naturally in the Notch. A complete process-management workspace should become a standalone app if it grows beyond quick inspection.

---

# Shell vs full app boundary

The Notch should support:

```text
CPU usage
memory usage
swap
GPU usage when available
disk activity
network activity
top CPU processes
top memory processes
quick process search
open full task manager
```

A future `luna-resources` / `luna-task-manager` app should own:

```text
large sortable process table
process tree
per-process details
open files/connections
signals and priority management
historical graphs
hardware details
per-device drill-down
advanced GPU/process data
```

Rule:

> The module diagnoses quickly. The app manages deeply.

---

# Target architecture

```text
Linux metrics sources
├── /proc
├── /sys
├── cgroups/systemd where useful
├── GPU-specific adapters
└── network/disk counters
          │
          ▼
   ResourceService
   ├── sampling
   ├── normalization
   ├── deltas/rates
   ├── process aggregation
   └── bounded history
          │
          ▼
 ResourceSnapshot / ProcessSnapshot
          │
          ▼
 luna-module-resources
          │
          ▼
        Notch
```

The service layer owns data collection. The module owns only presentation and user interaction.

---

# Sampling model

Many Linux counters are cumulative, so values such as CPU, disk and network activity must be derived from deltas between samples.

Conceptual flow:

```text
sample N-1
sample N
   ↓
delta / elapsed time
   ↓
normalized rate
```

The module must not compute these deltas independently for each widget.

Sampling cadence should be bounded and configurable internally. A reasonable design is:

```text
visible module
→ higher refresh cadence

module hidden
→ reduced cadence or summary-only collection
```

Do not tie collection frequency to GPUI frame rate.

---

# Data model

Conceptual snapshot:

```text
ResourceSnapshot
├── timestamp
├── cpu
├── memory
├── swap
├── disks[]
├── network[]
├── gpus[]
└── processes[]
```

Conceptual process model:

```text
ProcessSnapshot
├── pid
├── parent_pid
├── name
├── executable
├── command
├── cpu_percent
├── memory_bytes
├── read_rate
├── write_rate
├── user
└── state
```

The domain should use typed values/units rather than passing raw formatted strings from the collector.

---

# Process grouping

Linux desktop process trees often contain many helper processes. Luna should eventually support two views:

```text
Applications
→ aggregate processes belonging to the same desktop application

Processes
→ raw process-level view
```

The Notch MVP can begin with top processes and defer sophisticated application grouping.

---

# GPU metrics

GPU support must be capability-based because Linux exposes different data by vendor/driver.

Conceptually:

```text
GpuMetricsPort
├── AMD adapter
├── Intel adapter
└── NVIDIA adapter
```

A missing metric is represented as unavailable, not as zero.

The Resources module must degrade gracefully on systems where GPU utilization/temperature cannot be read.

---

# Process actions

Process control is a separate privileged/sensitive command path.

Forbidden in widgets:

```text
kill(pid)
Command::new("kill")
renice
raw signal calls
```

Instead:

```text
ResourcesModule
→ ProcessCommand::Terminate(pid)
→ ProcessControlPort
→ Linux adapter
```

Actions such as kill/stop/renice should be outside the first Notch MVP and should require explicit confirmation when added.

---

# State

Presentation state:

```text
selected_metric
active_tab
sort_key
filter_text
selected_process
history_window
```

Service state:

```text
latest resource snapshot
bounded metric history
process snapshots
collector capabilities
```

---

# Notch UX

Recommended default view:

```text
┌──────────────────────────────────┐
│ Resources                        │
│ CPU     23%   ━━━━━━━            │
│ Memory  61%   ━━━━━━━━━━━        │
│ GPU     14%   ━━━                │
│                                  │
│ Top processes                    │
│ firefox             12%   1.8GB  │
│ code                 8%   1.2GB  │
│ cargo                4%   420MB   │
│                                  │
│              Open Task Manager   │
└──────────────────────────────────┘
```

Optional tabs:

```text
Overview | Processes
```

Avoid putting a giant desktop process table inside the Notch.

---

# GPUI Kit usage

Useful components:

```text
Tabs
Progress
ScrollView
Table/List
Input
Dropdown
Tooltip
Popover
Button
Badge
```

Luna-specific visualization components:

```text
MetricGraph
ResourceBar
ProcessRow
Sparkline
UsageLegend
```

Graphs should use bounded histories and avoid allocating a new unbounded data structure every sample.

---

# Future full app

A future task-manager application should reuse the exact same `ResourceService` and process models:

```text
ResourceService
      │
 ┌────┴──────────┐
 ▼               ▼
Notch module   full task manager
```

The full app may add process tree, hardware panels and destructive process actions without moving those responsibilities into the shell module.

---

# MVP

## MVP 1

```text
CPU usage
memory usage
swap
top processes by CPU
top processes by memory
bounded refresh
```

## MVP 2

```text
network activity
disk activity
GPU metrics where available
small metric history graphs
process filtering
```

## Later

```text
application grouping
process actions
full task-manager app
process tree
hardware details
```

---

# Completion criteria

- no `/proc` or `/sys` reads occur from GPUI render code;
- sampling is independent from frame rendering;
- cumulative counters are converted to rates centrally;
- hidden/visible module state can influence sampling cost without losing correctness;
- unavailable GPU/system capabilities degrade cleanly;
- process actions, when introduced, flow through an explicit port.

---

# Non-goals

```text
full htop replacement inside the Notch
kernel profiler
perf/eBPF frontend
process debugger
systemd service manager
hardware overclocking controls
```
