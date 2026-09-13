//! Linux service adapter boundary.
//!
//! Concrete D-Bus, audio, power, network, and media integrations belong here
//! as they acquire an owned use case. The application does not depend on
//! those native clients directly.

use std::{fs, time::Instant};

use shell_core::{AppUsageMetrics, PlatformError};

#[derive(Debug, Default)]
pub struct LinuxServices;

impl LinuxServices {
    pub fn initialize() -> Result<Self, PlatformError> {
        Ok(Self)
    }
}

#[derive(Clone, Copy, Debug)]
struct CpuSample {
    process_jiffies: u64,
    total_jiffies: u64,
}

/// Samples process and Linux device metrics without invoking system CLIs.
///
/// The sampler is intentionally pull-based. The application owns its cadence
/// and can forward immutable samples to a renderer without blocking GPUI.
#[derive(Debug)]
pub struct UsageSampler {
    previous_cpu: Option<CpuSample>,
    started: Instant,
}

impl Default for UsageSampler {
    fn default() -> Self {
        Self::new()
    }
}

impl UsageSampler {
    pub fn new() -> Self {
        Self {
            previous_cpu: None,
            started: Instant::now(),
        }
    }

    pub fn sample(&mut self) -> AppUsageMetrics {
        let current_cpu = read_cpu_sample();
        let cpu_percent = current_cpu.and_then(|current| {
            let previous = self.previous_cpu.replace(current)?;
            let process_delta = current
                .process_jiffies
                .saturating_sub(previous.process_jiffies) as f32;
            let total_delta = current.total_jiffies.saturating_sub(previous.total_jiffies) as f32;
            if total_delta == 0.0 {
                return None;
            }
            let cpu_count = std::thread::available_parallelism()
                .map(std::num::NonZeroUsize::get)
                .unwrap_or(1) as f32;
            Some((process_delta / total_delta) * cpu_count * 100.0)
        });

        AppUsageMetrics {
            cpu_percent,
            ram_bytes: read_resident_memory().unwrap_or_default(),
            gpu_percent: read_gpu_busy_percent(),
            gpu_is_system: true,
            thread_count: read_thread_count().unwrap_or_default(),
            uptime_seconds: self.started.elapsed().as_secs(),
        }
    }
}

fn read_cpu_sample() -> Option<CpuSample> {
    let process = fs::read_to_string("/proc/self/stat").ok()?;
    let process_end = process.rfind(')')?;
    let fields = process.get(process_end + 1..)?.split_whitespace();
    let process_fields = fields.collect::<Vec<_>>();
    let process_jiffies = process_fields.get(11)?.parse::<u64>().ok()?
        + process_fields.get(12)?.parse::<u64>().ok()?;

    let stat = fs::read_to_string("/proc/stat").ok()?;
    let cpu_line = stat.lines().find(|line| line.starts_with("cpu "))?;
    let total_jiffies = cpu_line
        .split_whitespace()
        .skip(1)
        .take(8)
        .filter_map(|value| value.parse::<u64>().ok())
        .sum();

    Some(CpuSample {
        process_jiffies,
        total_jiffies,
    })
}

fn read_resident_memory() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let kilobytes = status.lines().find_map(|line| {
        line.strip_prefix("VmRSS:")
            .and_then(|value| value.split_whitespace().next())
            .and_then(|value| value.parse::<u64>().ok())
    })?;
    Some(kilobytes * 1024)
}

fn read_thread_count() -> Option<u32> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        line.strip_prefix("Threads:")
            .and_then(|value| value.trim().parse::<u32>().ok())
    })
}

fn read_gpu_busy_percent() -> Option<f32> {
    let entries = fs::read_dir("/sys/class/drm").ok()?;
    let mut values = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(card_number) = name.strip_prefix("card") else {
            continue;
        };
        if card_number.is_empty()
            || !card_number
                .chars()
                .all(|character| character.is_ascii_digit())
        {
            continue;
        }
        let Ok(value) = fs::read_to_string(entry.path().join("device/gpu_busy_percent")) else {
            continue;
        };
        let Ok(value) = value.trim().parse::<f32>() else {
            continue;
        };
        if value.is_finite() {
            values.push(value.clamp(0.0, 100.0));
        }
    }
    (!values.is_empty()).then(|| values.iter().sum::<f32>() / values.len() as f32)
}

#[cfg(test)]
mod tests {
    use super::UsageSampler;

    #[test]
    fn sampler_returns_process_metrics_without_external_commands() {
        let mut sampler = UsageSampler::new();
        let metrics = sampler.sample();

        assert!(metrics.ram_bytes > 0);
        assert!(metrics.thread_count > 0);
        assert_eq!(metrics.uptime_seconds, 0);
    }
}
