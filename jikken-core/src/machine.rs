use log::trace;
use sha2::{Digest, Sha256};
use sysinfo::{CpuExt, CpuRefreshKind, System, SystemExt};

pub fn new() -> Machine {
    let mut system = System::new();
    system.refresh_cpu_specifics(
        CpuRefreshKind::everything()
            .without_frequency()
            .without_cpu_usage(),
    );

    Machine { system }
}

pub struct Machine {
    system: sysinfo::System,
}

impl Machine {
    pub fn generate_machine_id(&self) -> String {
        let cpu_count = self.system.cpus().len();
        let cpu_vendor = if cpu_count > 0 {
            self.system.cpus()[0].brand()
        } else {
            ""
        };

        let system_os = self.system.long_os_version().unwrap_or_default();
        let system_kernel = self.system.kernel_version().unwrap_or_default();
        let system_hostname = self.system.host_name().unwrap_or_default();

        let machine_id = format!(
            "{}:{}:{}:{}:{}",
            cpu_count, cpu_vendor, system_os, system_kernel, system_hostname
        );

        trace!("machine_id: {}", machine_id);
        let h = hash(&machine_id);
        trace!("hashed machine_id: {}", h);

        h
    }
}

fn hash(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hex::encode(hasher.finalize().as_slice())
}
