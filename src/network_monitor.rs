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
    #[cfg(target_os = "linux")]
    linux_totals: HashMap<String, (u64, u64)>, // interface -> (total_rx, total_tx) from /proc/net/dev
    #[cfg(target_os = "windows")]
    windows_totals: HashMap<String, (u64, u64)>, // interface -> (total_rx, total_tx) from Windows API
}

impl NetworkMonitor {
    pub fn new() -> Self {
        // Create networks instance and refresh to get initial data
        let networks = Networks::new_with_refreshed_list();
        
        Self {
            networks,
            previous_stats: HashMap::new(),
            #[cfg(target_os = "linux")]
            linux_totals: HashMap::new(),
            #[cfg(target_os = "windows")]
            windows_totals: HashMap::new(),
        }
    }

    pub fn refresh(&mut self, show_virtual: bool) -> Vec<NetworkStats> {
        self.networks.refresh();
        let current_time = Instant::now();
        let mut stats = Vec::new();

        // On Linux, try to get true totals from /proc/net/dev
        #[cfg(target_os = "linux")]
        {
            if let Ok(proc_stats) = crate::network_linux::read_proc_net_dev() {
                self.linux_totals.clear();
                for stat in proc_stats {
                    self.linux_totals.insert(stat.name.clone(), (stat.bytes_received, stat.bytes_transmitted));
                }
            }
        }

        // On Windows, use Windows API directly instead of trying to match sysinfo names
        #[cfg(target_os = "windows")]
        {
            if let Ok(win_stats) = crate::network_windows::get_network_interface_stats(show_virtual) {
                for stat in win_stats {
                    let current_rx = stat.bytes_received;
                    let current_tx = stat.bytes_transmitted;
                    
                    // Clean up the interface name for better display
                    let mut interface_name = if !stat.friendly_name.is_empty() {
                        stat.friendly_name
                    } else {
                        stat.name
                    };
                    
                    // Remove WFP and other technical suffixes for cleaner display
                    interface_name = interface_name
                        .replace("-WFP Native MAC Layer LightWeight Filter-0000", "")
                        .replace("-WFP Native MAC Layer LightWeight Filter", "")
                        .replace("-Miniport Adapter", "")
                        .replace("-Virtual Switch", "")
                        .trim()
                        .to_string();
                    
                    let (download_speed, upload_speed) = if let Some((prev_rx, prev_tx, prev_time)) = 
                        self.previous_stats.get(&interface_name) {
                        
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

                    // Update previous stats
                    self.previous_stats.insert(
                        interface_name.clone(), 
                        (current_rx, current_tx, current_time)
                    );

                    // Consider interface active if it has received or transmitted data
                    let is_active = current_rx > 0 || current_tx > 0;

                    stats.push(NetworkStats {
                        name: interface_name,
                        bytes_received: current_rx,
                        bytes_transmitted: current_tx,
                        download_speed,
                        upload_speed,
                        is_active,
                    });
                }
                
                // Sort by name for consistent ordering and return early
                stats.sort_by(|a, b| a.name.cmp(&b.name));
                return stats;
            }
        }



        for (interface_name, network) in &self.networks {
            // Use platform-specific totals if available, otherwise fall back to sysinfo
            let (current_rx, current_tx) = {
                #[cfg(target_os = "linux")]
                {
                    if let Some((linux_rx, linux_tx)) = self.linux_totals.get(interface_name) {
                        (*linux_rx, *linux_tx)
                    } else {
                        (network.received(), network.transmitted())
                    }
                }
                #[cfg(target_os = "windows")]
                {
                    // Try exact match first
                    if let Some((win_rx, win_tx)) = self.windows_totals.get(interface_name) {
                        (*win_rx, *win_tx)
                    } else {
                        // Try partial matching for common interface names
                        let mut found = None;
                        for (win_name, (rx, tx)) in &self.windows_totals {
                            if interface_name.contains(win_name) || win_name.contains(interface_name) {
                                found = Some((*rx, *tx));
                                break;
                            }
                        }
                        
                        found.unwrap_or_else(|| (network.received(), network.transmitted()))
                    }
                }
                #[cfg(not(any(target_os = "linux", target_os = "windows")))]
                {
                    (network.received(), network.transmitted())
                }
            };
            
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

            // Update previous stats
            self.previous_stats.insert(
                interface_name.clone(), 
                (current_rx, current_tx, current_time)
            );

            // Consider interface active if it has received or transmitted data
            let is_active = current_rx > 0 || current_tx > 0;

            stats.push(NetworkStats {
                name: interface_name.clone(),
                bytes_received: current_rx,
                bytes_transmitted: current_tx,
                download_speed,
                upload_speed,
                is_active,
            });
        }

        // Sort by name for consistent ordering
        stats.sort_by(|a, b| a.name.cmp(&b.name));
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