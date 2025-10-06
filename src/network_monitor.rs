use sysinfo::Networks;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub name: String,
    pub bytes_received: u64,
    pub bytes_transmitted: u64,
    pub download_speed: f64, // bytes per second
    pub upload_speed: f64,   // bytes per second
    pub is_active: bool,
}

pub struct NetworkMonitor {
    networks: Networks,
    previous_stats: HashMap<String, (u64, u64, Instant)>, // interface -> (rx, tx, timestamp)
}

impl NetworkMonitor {
    pub fn new() -> Self {
        // Create networks instance and refresh to get initial data
        let networks = Networks::new_with_refreshed_list();
        
        Self {
            networks,
            previous_stats: HashMap::new(),
        }
    }

    pub fn refresh(&mut self, _show_virtual: bool) -> Vec<NetworkStats> {
        self.networks.refresh();
        let current_time = Instant::now();
        let mut stats = Vec::new();

        for (interface_name, data) in &self.networks {
            let current_rx = data.total_received();
            let current_tx = data.total_transmitted();

            // Skip loopback interfaces
            if interface_name.contains("Loopback") || interface_name == "lo" {
                continue;
            }

            let (download_speed, upload_speed) = if let Some((prev_rx, prev_tx, prev_time)) = 
                self.previous_stats.get(interface_name) {
                
                let duration = current_time.duration_since(*prev_time).as_secs_f64();
                if duration > 0.0 {
                    let download_speed = (current_rx.saturating_sub(*prev_rx) as f64) / duration;
                    let upload_speed = (current_tx.saturating_sub(*prev_tx) as f64) / duration;
                    (download_speed, upload_speed)
                } else {
                    (0.0, 0.0)
                }
            } else {
                (0.0, 0.0)
            };

            // Update previous stats for the next refresh
            self.previous_stats.insert(
                interface_name.clone(), 
                (current_rx, current_tx, current_time)
            );

            // An interface is active if it has any speed or has ever had traffic.
            let is_active = download_speed > 0.0 || upload_speed > 0.0 || current_rx > 0 || current_tx > 0;

            stats.push(NetworkStats {
                name: interface_name.clone(),
                bytes_received: current_rx,
                bytes_transmitted: current_tx,
                download_speed,
                upload_speed,
                is_active,
            });
        }
        
        stats
    }
}

pub fn format_bytes(bytes: f64) -> String {
    const UNITS: &[&str] = &["B/s", "KB/s", "MB/s", "GB/s"];
    let mut size = bytes;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if size >= 100.0 {
        format!("{:.0} {}", size, UNITS[unit_index])
    } else if size >= 10.0 {
        format!("{:.1} {}", size, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

pub fn format_total_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if size >= 100.0 {
        format!("{:.0} {}", size, UNITS[unit_index])
    } else if size >= 10.0 {
        format!("{:.1} {}", size, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}