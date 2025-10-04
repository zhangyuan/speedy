use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use windows::{
    Win32::Foundation::ERROR_SUCCESS,
    Win32::NetworkManagement::IpHelper::{
        GetIfTable2, FreeMibTable, MIB_IF_TABLE2, MIB_IF_ROW2,
    },
};

#[derive(Debug, Clone)]
pub struct WindowsNetworkStats {
    pub name: String,
    pub bytes_received: u64,
    pub bytes_transmitted: u64,
    pub friendly_name: String,
}

pub fn get_network_interface_stats(show_virtual: bool) -> Result<Vec<WindowsNetworkStats>, Box<dyn std::error::Error>> {
    // Use GetIfTable2 with filtering based on show_virtual parameter
    get_iftable2_stats(show_virtual)
}



// Safe wrapper for accessing interface table entries
unsafe fn get_interface_row(table: &MIB_IF_TABLE2, index: usize) -> Option<&MIB_IF_ROW2> {
    if index < table.NumEntries as usize {
        if index < table.Table.len() {
            // Direct access for first entry
            Some(&table.Table[index])
        } else {
            // Pointer arithmetic for additional entries in flexible array
            let base_ptr = table.Table.as_ptr();
            Some(&*base_ptr.add(index))
        }
    } else {
        None
    }
}

fn get_iftable2_stats(show_virtual: bool) -> Result<Vec<WindowsNetworkStats>, Box<dyn std::error::Error>> {
    // RAII wrapper for Windows API table management
    struct InterfaceTable(*mut MIB_IF_TABLE2);
    
    impl Drop for InterfaceTable {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    FreeMibTable(self.0 as *mut _);
                }
            }
        }
    }
    
    let mut if_table_ptr: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    
    // Get the interface table using Windows API
    let result = unsafe { GetIfTable2(&mut if_table_ptr) };
    if result != ERROR_SUCCESS {
        return Err(format!("GetIfTable2 failed with error: {}", result.0).into());
    }

    if if_table_ptr.is_null() {
        return Err("GetIfTable2 returned null table".into());
    }

    // Wrap in RAII for automatic cleanup
    let _table_guard = InterfaceTable(if_table_ptr);
    let table = unsafe { &*if_table_ptr };
    let mut stats = Vec::new();

    // Process each interface with bounds checking
    let num_entries = std::cmp::min(table.NumEntries as usize, 50); // Reasonable safety limit
    
    for i in 0..num_entries {
        // Use safe wrapper function for array access
        let row = unsafe { get_interface_row(table, i) };
        let row = match row {
            Some(r) => r,
            None => continue,
        };
        
        // Get interface name and friendly name - handle errors gracefully
        let name = match get_interface_name(row) {
            Ok(n) => n,
            Err(_) => continue, // Skip if we can't get the name
        };
        
        let friendly_name = match get_interface_friendly_name(row) {
            Ok(n) => n,
            Err(_) => String::new(), // Use empty string if friendly name fails
        };
        
        // No filtering - include all interfaces
        stats.push(WindowsNetworkStats {
            name: name.clone(),
            bytes_received: row.InOctets,
            bytes_transmitted: row.OutOctets,
            friendly_name: friendly_name.clone(),
        });
    }

    // Table automatically freed by RAII guard
    
    // Filter out unwanted interfaces based on show_virtual parameter
    let filtered_stats: Vec<WindowsNetworkStats> = stats.into_iter().filter(|stat| {
            // Always skip loopback interfaces
            if stat.name.contains("Loopback") || stat.friendly_name.contains("Loopback") {
                return false;
            }
            
            // Always skip system debug and Windows network stack components
            if stat.name.contains("Microsoft Kernel Debug") ||
               stat.name.contains("Teredo") ||
               stat.name.contains("6to4") ||
               stat.name.contains("ISATAP") ||
               stat.name.contains("Microsoft Wi-Fi Direct Virtual Adapter") ||
               stat.friendly_name.contains("Teredo") {
                return false;
            }
            
            // Always skip Windows network stack layers (not real interfaces)
            if stat.name.contains("QoS Packet Scheduler") ||
               stat.name.contains("WFP") ||
               stat.name.contains("LightWeight Filter") ||
               stat.name.contains("Native MAC Layer") ||
               stat.friendly_name.contains("QoS Packet Scheduler") ||
               stat.friendly_name.contains("WFP") ||
               stat.friendly_name.contains("LightWeight Filter") {
                return false;
            }
            
            // Virtual adapter filtering: skip only if show_virtual is false
            if !show_virtual {
                if stat.name.contains("Miniport") ||
                   stat.name.contains("Virtual") ||
                   stat.name.contains("TAP-") ||
                   stat.name.contains("OpenVPN") ||
                   stat.name.contains("VMware") ||
                   stat.name.contains("VirtualBox") ||
                   stat.name.contains("Tailscale") ||
                   stat.name.contains("WireGuard") ||
                   stat.friendly_name.contains("Virtual") ||
                   stat.friendly_name.contains("TAP") ||
                   stat.friendly_name.contains("VPN") ||
                   stat.friendly_name.contains("Tailscale") ||
                   stat.friendly_name.contains("WireGuard") {
                    return false;
                }
            }
            
            true
    }).collect();
    
    // Smart deduplication: keep the "best" interface for each friendly name
    let mut interface_map: std::collections::HashMap<String, WindowsNetworkStats> = std::collections::HashMap::new();
    
    for stat in filtered_stats {
        // Create a key for deduplication - use clean friendly name
        let key = if !stat.friendly_name.is_empty() {
            // Remove WFP suffixes for cleaner grouping
            stat.friendly_name
                .replace("-WFP Native MAC Layer LightWeight Filter-0000", "")
                .replace("-WFP Native MAC Layer LightWeight Filter", "")
                .trim()
                .to_string()
        } else {
            stat.name.clone()
        };
        
        // Skip interfaces with empty keys
        if key.is_empty() {
            continue;
        }
        
        // Check if we should keep this interface over any existing one
        let should_keep = if let Some(existing) = interface_map.get(&key) {
            // Prefer interfaces with more total traffic, or if same, prefer cleaner names
            let existing_total = existing.bytes_received + existing.bytes_transmitted;
            let current_total = stat.bytes_received + stat.bytes_transmitted;
            
            if current_total > existing_total {
                true
            } else if current_total == existing_total {
                // If same traffic, prefer the one without WFP in the name
                !stat.name.contains("WFP") && existing.name.contains("WFP")
            } else {
                false
            }
        } else {
            // First interface with this name
            true
        };
        
        if should_keep {
            // Clean up the display name
            let cleaned_stat = WindowsNetworkStats {
                name: key.clone(),
                bytes_received: stat.bytes_received,
                bytes_transmitted: stat.bytes_transmitted,
                friendly_name: key.clone(),
            };
            interface_map.insert(key, cleaned_stat);
        }
    }
    
    // Convert back to Vec
    let unique_stats: Vec<WindowsNetworkStats> = interface_map.into_values().collect();
    
    Ok(unique_stats)
}

fn get_interface_name(row: &MIB_IF_ROW2) -> Result<String, Box<dyn std::error::Error>> {
    // Convert the description from wide string to String
    // Windows uses UTF-16, we need to handle this carefully for Chinese characters
    
    // Find the length by looking for null terminator
    let max_len = std::cmp::min(row.Description.len(), 256); // Reasonable limit
    let mut len = 0;
    
    for i in 0..max_len {
        if row.Description[i] == 0 {
            break;
        }
        len += 1;
    }
    
    if len > 0 {
        // Safe: We're accessing a slice within bounds of the array
        let wide_slice = &row.Description[0..len];
        // Use String::from_utf16_lossy for better Chinese character support
        match String::from_utf16(wide_slice) {
            Ok(s) => Ok(s),
            Err(_) => {
                // Fallback to lossy conversion
                let os_string = OsString::from_wide(wide_slice);
                Ok(os_string.to_string_lossy().to_string())
            }
        }
    } else {
        Ok("Unknown Interface".to_string())
    }
}

fn get_interface_friendly_name(row: &MIB_IF_ROW2) -> Result<String, Box<dyn std::error::Error>> {
    // Convert the alias (friendly name) from wide string to String
    // This is usually the user-friendly name like "以太网", "Wi-Fi" etc.
    
    // Find the length by looking for null terminator
    let max_len = std::cmp::min(row.Alias.len(), 256); // Reasonable limit
    let mut len = 0;
    
    for i in 0..max_len {
        if row.Alias[i] == 0 {
            break;
        }
        len += 1;
    }
    
    if len > 0 {
        // Safe: We're accessing a slice within bounds of the array
        let wide_slice = &row.Alias[0..len];
        // Use String::from_utf16 for better Chinese character support
        match String::from_utf16(wide_slice) {
            Ok(s) => Ok(s),
            Err(_) => {
                // Fallback to lossy conversion
                let os_string = OsString::from_wide(wide_slice);
                Ok(os_string.to_string_lossy().to_string())
            }
        }
    } else {
        Ok(String::new())
    }
}

