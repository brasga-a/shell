/// Runtime metrics shown by the non-interactive debug overlay.
///
/// `gpu_percent` is an aggregate system busy percentage when the Linux
/// driver exposes one through sysfs. Linux does not provide one universal
/// per-process GPU percentage API, so `gpu_is_system` makes that limitation
/// explicit in the UI.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AppUsageMetrics {
    pub cpu_percent: Option<f32>,
    pub ram_bytes: u64,
    pub gpu_percent: Option<f32>,
    pub gpu_is_system: bool,
    pub thread_count: u32,
    pub uptime_seconds: u64,
}
