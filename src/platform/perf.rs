use std::time::Instant;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct FileTime {
    dw_low_date_time: u32,
    dw_high_date_time: u32,
}

impl FileTime {
    fn to_u64(self) -> u64 {
        ((self.dw_high_date_time as u64) << 32) | (self.dw_low_date_time as u64)
    }
}

#[repr(C)]
#[derive(Default)]
struct ProcessMemoryCounters {
    cb: u32,
    page_fault_count: u32,
    peak_working_set_size: usize,
    working_set_size: usize,
    quota_peak_paged_pool_usage: usize,
    quota_paged_pool_usage: usize,
    quota_peak_non_paged_pool_usage: usize,
    quota_non_paged_pool_usage: usize,
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
}

#[cfg(target_os = "windows")]
extern "system" {
    fn GetSystemTimes(
        lp_idle_time: *mut FileTime,
        lp_kernel_time: *mut FileTime,
        lp_user_time: *mut FileTime,
    ) -> i32;

    fn GetCurrentProcess() -> isize;

    fn K32GetProcessMemoryInfo(
        h_process: isize,
        ppsmem_counters: *mut ProcessMemoryCounters,
        cb: u32,
    ) -> i32;
}

/// Real-time performance monitor for NEXORA Studio.
/// Measures render framerate (FPS), overall system CPU load (%), and process RAM working set (MB).
#[derive(Debug)]
pub struct PerformanceMonitor {
    last_sample_instant: Instant,
    prev_idle: u64,
    prev_kernel: u64,
    prev_user: u64,
    pub cpu_usage_pct: f32,
    pub ram_usage_mb: f32,
    pub render_fps: f32,
    frame_count: u32,
    last_fps_instant: Instant,
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        let mut monitor = Self {
            last_sample_instant: Instant::now(),
            prev_idle: 0,
            prev_kernel: 0,
            prev_user: 0,
            cpu_usage_pct: 0.0,
            ram_usage_mb: 0.0,
            render_fps: 60.0,
            frame_count: 0,
            last_fps_instant: Instant::now(),
        };
        monitor.sample_cpu();
        monitor.sample_memory();
        monitor
    }
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Call this once per frame in `NexoraApp::update` to measure FPS and periodically update CPU/RAM metrics.
    pub fn on_frame(&mut self) {
        self.frame_count += 1;
        let elapsed_fps = self.last_fps_instant.elapsed();
        if elapsed_fps.as_millis() >= 400 {
            let fps = (self.frame_count as f32) / elapsed_fps.as_secs_f32();
            self.render_fps = (self.render_fps * 0.70 + fps * 0.30).clamp(1.0, 360.0);
            self.frame_count = 0;
            self.last_fps_instant = Instant::now();
        }

        if self.last_sample_instant.elapsed().as_millis() >= 800 {
            self.sample_cpu();
            self.sample_memory();
            self.last_sample_instant = Instant::now();
        }
    }

    #[cfg(target_os = "windows")]
    fn sample_cpu(&mut self) {
        let mut idle = FileTime::default();
        let mut kernel = FileTime::default();
        let mut user = FileTime::default();

        let ok = unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) };
        if ok != 0 {
            let cur_idle = idle.to_u64();
            let cur_kernel = kernel.to_u64();
            let cur_user = user.to_u64();

            if self.prev_idle > 0 {
                let idle_diff = cur_idle.saturating_sub(self.prev_idle);
                let kernel_diff = cur_kernel.saturating_sub(self.prev_kernel);
                let user_diff = cur_user.saturating_sub(self.prev_user);
                let total_diff = kernel_diff + user_diff;

                if total_diff > 0 {
                    let busy = total_diff.saturating_sub(idle_diff);
                    let pct = ((busy as f64 / total_diff as f64) * 100.0) as f32;
                    self.cpu_usage_pct = (self.cpu_usage_pct * 0.5 + pct * 0.5).clamp(0.0, 100.0);
                }
            }

            self.prev_idle = cur_idle;
            self.prev_kernel = cur_kernel;
            self.prev_user = cur_user;
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn sample_cpu(&mut self) {
        self.cpu_usage_pct = 0.0;
    }

    #[cfg(target_os = "windows")]
    fn sample_memory(&mut self) {
        let mut pmc = ProcessMemoryCounters::default();
        pmc.cb = std::mem::size_of::<ProcessMemoryCounters>() as u32;

        let ok = unsafe {
            let handle = GetCurrentProcess();
            K32GetProcessMemoryInfo(handle, &mut pmc, pmc.cb)
        };

        if ok != 0 {
            let mb = pmc.working_set_size as f32 / (1024.0 * 1024.0);
            self.ram_usage_mb = mb.max(0.0);
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn sample_memory(&mut self) {
        self.ram_usage_mb = 0.0;
    }
}

