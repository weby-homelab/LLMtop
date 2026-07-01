use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct GpuMetrics {
    pub index: u32,
    pub name: String,
    pub temp_c: u64,
    pub utilization_pct: u32,
    pub vram_used_mb: u64,
    pub vram_total_mb: u64,
    pub power_w: u32,
    pub power_max_w: u32,
}

#[derive(Debug, Default)]
pub struct GpuSampler {
    #[cfg(feature = "nvidia")]
    nvml: Option<nvml_wrapper::Nvml>,
    #[cfg(not(feature = "nvidia"))]
    _phantom: (),
}

impl GpuSampler {
    pub fn new() -> Self {
        #[cfg(feature = "nvidia")]
        {
            let nvml = nvml_wrapper::Nvml::init().ok();
            Self { nvml }
        }
        #[cfg(not(feature = "nvidia"))]
        {
            Self { _phantom: () }
        }
    }

    pub fn sample(&self) -> Vec<GpuMetrics> {
        #[cfg(feature = "nvidia")]
        {
            let Some(nvml) = &self.nvml else {
                return Vec::new();
            };
            let Ok(count) = nvml.device_count() else {
                return Vec::new();
            };
            let mut metrics = Vec::new();
            for i in 0..count {
                let Ok(device) = nvml.device_by_index(i) else {
                    continue;
                };
                let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
                let temp_c = device
                    .temperature(nvml_wrapper::enum_wrappers::TemperatureSensor::Gpu)
                    .unwrap_or(0);
                let utilization_pct = device.utilization_rates().map(|u| u.gpu).unwrap_or(0);
                let mem_info = device.memory_info().ok();
                let vram_used_mb = mem_info.map(|m| m.used / (1024 * 1024)).unwrap_or(0);
                let vram_total_mb = mem_info.map(|m| m.total / (1024 * 1024)).unwrap_or(0);
                let power_w = device.power_usage().map(|p| p / 1000).unwrap_or(0);
                let power_max_w = device.enforced_power_limit().map(|p| p / 1000).unwrap_or(0);
                metrics.push(GpuMetrics {
                    index: i,
                    name,
                    temp_c,
                    utilization_pct,
                    vram_used_mb,
                    vram_total_mb,
                    power_w,
                    power_max_w,
                });
            }
            metrics
        }
        #[cfg(not(feature = "nvidia"))]
        {
            Vec::new()
        }
    }
}
