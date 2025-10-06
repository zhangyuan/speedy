use std::collections::HashMap;
use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo};
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
    
    // 使用 netstat2 获取活动连接信息来增强活动检测
    let active_connections = get_active_connections_by_interface().unwrap_or_default();
    
    let mut stats = Vec::new();
    
    for (interface_name, network) in &networks {
        // 跳过回环接口
        if interface_name == "Loopback Pseudo-Interface 1" || interface_name.contains("Loopback") {
            continue;
        }
        
        let bytes_received = network.total_received();
        let bytes_transmitted = network.total_transmitted();
        
        // 获取此接口的活动连接数
        let connection_count = active_connections.get(interface_name).unwrap_or(&0);
        
        // 改进的活动检测：有流量或有活动连接
        let is_active = bytes_received > 0 || bytes_transmitted > 0 || *connection_count > 0;
        
        // 清理接口名称用于显示
        let display_name = clean_interface_name(interface_name);
        
        stats.push(WindowsNetworkStats {
            name: display_name.clone(),
            bytes_received,
            bytes_transmitted,
            friendly_name: display_name,
            // active_connections: *connection_count,
            is_active,
        });
    }
    
    Ok(stats)
}

fn get_active_connections_by_interface() -> Result<HashMap<String, u32>, Box<dyn std::error::Error>> {
    let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;
    
    let sockets_info = get_sockets_info(af_flags, proto_flags)?;
    let mut interface_connections: HashMap<String, u32> = HashMap::new();
    
    // 获取网络接口信息用于模式匹配
    let networks = Networks::new_with_refreshed_list();
    
    for socket in sockets_info {
        let local_addr = match socket.protocol_socket_info {
            ProtocolSocketInfo::Tcp(tcp_info) => tcp_info.local_addr,
            ProtocolSocketInfo::Udp(udp_info) => udp_info.local_addr,
        };
        
        if !local_addr.is_loopback() && !local_addr.is_unspecified() {
            // 简化方法：按 IP 地址类型分配到可能的接口
            let interface_name = if local_addr.is_ipv4() {
                // 假设 IPv4 地址主要来自以太网或 WiFi
                find_likely_interface(&networks, "Ethernet")
                    .or_else(|| find_likely_interface(&networks, "Wi-Fi"))
                    .or_else(|| find_likely_interface(&networks, "eth"))
                    .or_else(|| find_likely_interface(&networks, "wlan"))
                    .unwrap_or_else(|| "Unknown IPv4 Interface".to_string())
            } else {
                // IPv6 可能来自多种接口
                find_likely_interface(&networks, "Ethernet")
                    .or_else(|| find_likely_interface(&networks, "Wi-Fi"))
                    .unwrap_or_else(|| "Unknown IPv6 Interface".to_string())
            };
            
            *interface_connections.entry(interface_name).or_insert(0) += 1;
        }
    }
    
    Ok(interface_connections)
}

// 辅助函数：查找可能的接口
fn find_likely_interface(networks: &Networks, pattern: &str) -> Option<String> {
    for (name, _) in networks {
        if name.contains(pattern) {
            return Some(name.clone());
        }
    }
    None
}



// 清理接口名称用于显示
fn clean_interface_name(name: &str) -> String {
    name
        // 移除常见的技术后缀
        .replace("-WFP Native MAC Layer LightWeight Filter-0000", "")
        .replace("-WFP Native MAC Layer LightWeight Filter", "")
        .replace("-QoS Packet Scheduler-0000", "")
        .replace("-QoS Packet Scheduler", "")
        .replace(" - Miniport Adapter", "")
        .replace(" - Virtual Switch", "")
        .trim()
        .to_string()
}
