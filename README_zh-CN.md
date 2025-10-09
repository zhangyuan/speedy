# Speedy - 网络速度监控器

[中文文档](README_zh-CN.md) | [English](README.md)

## 应用介绍

Speedy 是一个使用 Rust 和 egui 框架构建的网络速度监控工具。它可以实时显示系统中所有网络接口的状态和网络传输速率。

### 主要功能

- **实时监控**：显示所有网络接口的下载和上传速度
- **接口列表**：展示系统中可用的网络接口
- **速度显示**：以易读的格式显示网络传输速率
- **搜索过滤**：支持按接口名称搜索和过滤
- **排序功能**：可按名称或下载速度排序
- **置顶显示**：支持窗口置顶显示，方便实时监控

## 界面截图

![Speedy Network Monitor](assets/macos.png)

## 下载和使用

### macOS
从 GitHub Actions 的构建产物下载 `speedy-macos.zip`，解压后双击 `Speedy.app` 运行。

### Windows
从 GitHub Actions 的构建产物下载 `speedy-windows.exe`，双击运行。

### Linux
从 GitHub Actions 的构建产物下载 `speedy` 可执行文件，在终端中运行。

## 技术特点

- **跨平台**：支持 macOS、Windows、Linux
- **轻量级**：使用 Rust 编写，性能高效
- **原生界面**：基于 egui 框架，提供原生用户体验
- **无依赖**：单个可执行文件，无需额外安装

## 构建说明

### 从源码构建

```bash
git clone https://github.com/zhangyuan/speedy
cd speedy
cargo build --release
```

## 致谢

这个项目利用了 AI 进行编写和开发。

## 许可证

MIT License
