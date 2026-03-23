// ==================== Autostart ====================

use auto_launch::{AutoLaunch, AutoLaunchBuilder};

pub struct Autostart {
    inner: AutoLaunch,
}

impl Autostart {
    pub fn new() -> Self {
        let exe = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .to_string();

        let inner = AutoLaunchBuilder::new()
            .set_app_name("NexoraPrinterManager")
            .set_app_path(&exe)
            // Window starts hidden when launched at login
            .set_args(&["--minimized"])
            .build()
            .expect("Failed to build AutoLaunch");

        Self { inner }
    }

    pub fn is_enabled(&self) -> bool {
        self.inner.is_enabled().unwrap_or(false)
    }

    pub fn enable(&self) -> Result<(), String> {
        self.inner
            .enable()
            .map_err(|e| format!("Failed to enable autostart: {}", e))
    }

    pub fn disable(&self) -> Result<(), String> {
        self.inner
            .disable()
            .map_err(|e| format!("Failed to disable autostart: {}", e))
    }

    pub fn toggle(&self) -> Result<bool, String> {
        if self.is_enabled() {
            self.disable()?;
            log::info!("Autostart disabled");
            Ok(false)
        } else {
            self.enable()?;
            log::info!("Autostart enabled");
            Ok(true)
        }
    }
}
