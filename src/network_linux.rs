use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Debug, Clone)]
pub struct LinuxNetworkStats {
    pub name: String,
    pub bytes_received: u64,
    pub bytes_transmitted: u64,
}

pub fn read_proc_net_dev() -> Result<Vec<LinuxNetworkStats>, Box<dyn std::error::Error>> {
    let file = File::open("/proc/net/dev")?;
    let reader = BufReader::new(file);
    let mut stats = Vec::new();

    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;
        
        // Skip header lines
        if line_num < 2 {
            continue;
        }

        // Parse the line format:
        // Inter-|   Receive                                                |  Transmit
        //  face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
        //     lo: 2776770   11307    0    0    0     0          0         0  2776770   11307    0    0    0     0       0          0
        
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 17 {
            continue;
        }

        // Interface name is the first part, remove the colon
        let interface_name = parts[0].trim_end_matches(':').to_string();
        
        // Skip loopback interface if you want (optional)
        // if interface_name == "lo" { continue; }

        // Parse received bytes (column 1) and transmitted bytes (column 9)
        if let (Ok(rx_bytes), Ok(tx_bytes)) = (
            parts[1].parse::<u64>(),
            parts[9].parse::<u64>()
        ) {
            stats.push(LinuxNetworkStats {
                name: interface_name,
                bytes_received: rx_bytes,
                bytes_transmitted: tx_bytes,
            });
        }
    }

    Ok(stats)
}