use sysinfo::Networks;

#[derive(Debug, Clone)]
pub struct WindowsNetworkStats {
    pub name: String,
    pub bytes_received: u64,
    pub bytes_transmitted: u64,
    pub friendly_name: String,
    // pub active_connections: u32,
    pub is_active: bool,
}

pub fn get_network_interface_stats(_show_virtual: bool) -> Result<Vec<WindowsNetworkStats>, Box<dyn std::error::Error>> {
    // 使用 sysinfo 获取基础网络接口信息（简单且跨平台）
    let networks = Networks::new_with_refreshed_list();
    
    let mut stats = Vec::new();
    
    for (interface_name, network) in &networks {
        // 跳过回环接口
        if interface_name == "Loopback Pseudo-Interface 1" || interface_name.contains("Loopback") {
            continue;
        }
        
        let bytes_received = network.total_received();
        let bytes_transmitted = network.total_transmitted();
        
        // 简化的活动检测：有流量即为活跃
        let is_active = bytes_received > 0 || bytes_transmitted > 0;
        
        stats.push(WindowsNetworkStats {
            name: interface_name.clone(),
            bytes_received,
            bytes_transmitted,
            friendly_name: interface_name.clone(),
            is_active,
        });
    }
    
    Ok(stats)
}
