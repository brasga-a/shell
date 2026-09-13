# Debug usage overlay

The panel now opens a visual-only diagnostics surface in the bottom-right
corner of the current Wayland session.

Displayed values:

- process CPU usage;
- process resident memory (RAM);
- GPU busy percentage when the driver exposes
  `/sys/class/drm/card*/device/gpu_busy_percent`;
- process thread count;
- shell uptime;
- compositor output and workspace counts.

The overlay sets an empty Wayland input region with GPUI's
`Window::set_input_region(Some(&[]))`, so pointer and touch events pass through
to the surfaces below it.

GPU usage is marked `(sys)` because Linux does not expose one universal
per-process GPU utilization API across AMD, Intel and NVIDIA drivers. When the
driver does not expose the sysfs metric, the overlay displays `n/a` instead of
inventing a value.
