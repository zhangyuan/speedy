# Speedy Network Monitor - Windows 重构版本

## 概述
这是一个跨平台的网络速度监控应用程序，使用 Rust 和 egui 构建。在 Windows 系统上经过重构，使用了 `netstat2` 库和 Windows API 来获取准确的网络接口信息。

## Windows 特性

### 网络接口检测
- 使用 Windows `GetIfTable2` API 获取详细的网络接口信息
- 支持中文接口名称显示（如"以太网"、"Wi-Fi"等）
- 智能过滤虚拟适配器和系统组件

### 活动连接监控  
- 集成 `netstat2` 库检测活动网络连接
- 实时统计 TCP/UDP 连接数
- 准确判断接口活跃状态

### 接口管理
- 自动去重相似接口名称
- 清理 WFP (Windows Filtering Platform) 后缀
- 智能选择最佳显示名称

### UI 增强
- 支持中文字体（Microsoft YaHei/SimHei）
- 窗口置顶功能
- 实时速度显示（下载/上传）
- 可切换显示虚拟适配器

## 依赖项

### Windows 特定依赖
```toml
[target.'cfg(target_os = "windows")'.dependencies]
eframe = { version = "0.32", default-features = false, features = ["wgpu", "default_fonts"] }
wgpu = { version = "25.0", features = ["dx12", "vulkan"] }
netstat2 = "0.9"
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_NetworkManagement_IpHelper",
    "Win32_NetworkManagement_Ndis",
    "Win32_System_Registry", 
    "Win32_Networking_WinSock",
] }
```

## 构建和运行

```bash
# 构建
cargo build

# 运行
cargo run
```

## 架构说明

### 模块结构
- `main.rs` - 主程序和 GUI
- `network_monitor.rs` - 跨平台网络监控抽象层
- `network_windows.rs` - Windows 特定实现
- `network_linux.rs` - Linux 特定实现

### Windows 实现特色
1. **准确的字节统计**: 直接从 Windows API 获取接口统计
2. **连接状态检测**: 使用 netstat2 实时检测活动连接
3. **智能接口过滤**: 自动过滤系统组件和重复接口
4. **中文支持**: 完美支持中文接口名称显示
5. **内存安全**: 使用 RAII 模式管理 Windows API 资源

### 性能优化
- 智能去重算法减少接口数量
- 高效的字符串转换处理
- 最小化 Windows API 调用频率
- 安全的内存访问模式

## 从 Linux 版本的重构要点

1. **网络接口获取**: 从 `/proc/net/dev` 切换到 `GetIfTable2`
2. **连接检测**: 集成 `netstat2` 替代简单的字节计数
3. **字符编码**: 处理 UTF-16 到 UTF-8 转换支持中文
4. **接口过滤**: Windows 特有的虚拟适配器过滤逻辑
5. **UI适配**: Windows 下使用 WGPU 渲染后端

重构确保了与 Linux 版本相同的功能接口，同时充分利用了 Windows 平台的特性和 API。