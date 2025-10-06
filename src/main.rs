mod network_monitor;

use eframe::egui;
use network_monitor::{NetworkMonitor, NetworkStats, format_bytes, format_total_bytes};
use std::time::{Duration, Instant};

struct SpeedyApp {
    network_monitor: NetworkMonitor,
    network_stats: Vec<NetworkStats>,
    last_update: Instant,
    update_interval: Duration,
    show_inactive: bool,
    show_virtual: bool,
    always_on_top: bool,
    first_frame: bool,
}

impl Default for SpeedyApp {
    fn default() -> Self {
        Self {
            network_monitor: NetworkMonitor::new(),
            network_stats: Vec::new(),
            last_update: Instant::now(),
            update_interval: Duration::from_secs(1),
            show_inactive: false,
            show_virtual: false,
            always_on_top: true,
            first_frame: true,
        }
    }
}

impl eframe::App for SpeedyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply always-on-top on first frame (since builder settings don't work reliably)
        if self.first_frame && self.always_on_top {
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
            self.first_frame = false;
        }

        // Update network stats periodically
        if self.last_update.elapsed() >= self.update_interval {
            self.network_stats = self.network_monitor.refresh(self.show_virtual);
            self.last_update = Instant::now();
        }

        // Request repaint to keep updating
        ctx.request_repaint_after(self.update_interval);

        egui::CentralPanel::default().show(ctx, |ui| {
            // Controls
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.show_inactive, "Show inactive interfaces");
                ui.checkbox(&mut self.show_virtual, "Show virtual adapters");
                ui.separator();
                if ui.checkbox(&mut self.always_on_top, "Always on top").changed() {
                    // Try to update always-on-top behavior
                    ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                        if self.always_on_top {
                            egui::WindowLevel::AlwaysOnTop
                        } else {
                            egui::WindowLevel::Normal
                        }
                    ));
                }
                ui.separator();
                ui.label(format!("Total interfaces: {}", self.network_stats.len()));
            });

            ui.separator();

            // Show network interfaces
            if self.network_stats.is_empty() {
                ui.label("Scanning for network interfaces...");
            } else {
                self.show_network_interfaces(ui);
            }
        });
    }
}

impl SpeedyApp {
    fn show_network_interfaces(&self, ui: &mut egui::Ui) {
        use egui::{Color32, RichText};

        egui::ScrollArea::vertical().show(ui, |ui| {
            for stats in &self.network_stats {
                // Skip inactive interfaces if not showing them
                if !self.show_inactive && !stats.is_active {
                    continue;
                }

                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        // Interface name and status
                        let status_color = if stats.is_active {
                            Color32::from_rgb(0, 200, 0)
                        } else {
                            Color32::from_rgb(128, 128, 128)
                        };

                        let status_text = if stats.is_active { "[ON]" } else { "[OFF]" };
                        ui.label(RichText::new(status_text).color(status_color).size(14.0).strong());
                        
                        ui.label(RichText::new(&stats.name).strong().size(16.0));
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("Total: Down:{} Up:{}", 
                                format_total_bytes(stats.bytes_received),
                                format_total_bytes(stats.bytes_transmitted)
                            ));
                        });
                    });

                    ui.separator();

                    // Speed display
                    ui.horizontal(|ui| {
                        // Download speed
                        ui.group(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Download").color(Color32::from_rgb(20, 100, 200)));
                                let speed_text = format_bytes(stats.download_speed);
                                let speed_color = if stats.download_speed > 1024.0 * 1024.0 { // > 1 MB/s
                                    Color32::from_rgb(0, 200, 0)  // 深绿色，在浅色背景下清晰
                                } else if stats.download_speed > 1024.0 { // > 1 KB/s
                                    Color32::from_rgb(200, 150, 0)  // 深黄色，在浅色背景下清晰
                                } else {
                                    Color32::from_rgb(80, 80, 80)  // 深灰色，替代白色
                                };
                                ui.label(RichText::new(speed_text).color(speed_color).size(18.0).strong());
                            });
                        });

                        ui.add_space(20.0);

                        // Upload speed
                        ui.group(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Upload").color(Color32::from_rgb(200, 100, 20)));
                                let speed_text = format_bytes(stats.upload_speed);
                                let speed_color = if stats.upload_speed > 1024.0 * 1024.0 { // > 1 MB/s
                                    Color32::from_rgb(0, 200, 0)  // 深绿色，在浅色背景下清晰
                                } else if stats.upload_speed > 1024.0 { // > 1 KB/s
                                    Color32::from_rgb(200, 150, 0)  // 深黄色，在浅色背景下清晰
                                } else {
                                    Color32::from_rgb(80, 80, 80)  // 深灰色，替代白色
                                };
                                ui.label(RichText::new(speed_text).color(speed_color).size(18.0).strong());
                            });
                        });
                    });
                });

                ui.add_space(10.0);
            }
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([450.0, 300.0])
            .with_min_inner_size([350.0, 250.0])
            .with_always_on_top()
            .with_window_level(egui::WindowLevel::AlwaysOnTop)
            .with_icon(eframe::icon_data::from_png_bytes(&[]).unwrap_or_default()),
        ..Default::default()
    };

    eframe::run_native(
        "Speedy - Network Speed Monitor",
        options,
        Box::new(|cc| {
            // Load Chinese fonts for better character support
            #[cfg(target_os = "windows")]
            let mut fonts = egui::FontDefinitions::default();
            #[cfg(not(target_os = "windows"))]
            let fonts = egui::FontDefinitions::default();
            
            // Add system fonts that support Chinese characters
            #[cfg(target_os = "windows")]
            {
                // Try to load system fonts that support Chinese
                if let Ok(font_data) = std::fs::read("C:\\Windows\\Fonts\\msyh.ttc") {
                    fonts.font_data.insert(
                        "Microsoft YaHei".to_owned(),
                        egui::FontData::from_owned(font_data).into(),
                    );
                    
                    // Set as primary font for better Chinese support
                    fonts.families.get_mut(&egui::FontFamily::Proportional)
                        .unwrap()
                        .insert(0, "Microsoft YaHei".to_owned());
                } else if let Ok(font_data) = std::fs::read("C:\\Windows\\Fonts\\simhei.ttf") {
                    fonts.font_data.insert(
                        "SimHei".to_owned(),
                        egui::FontData::from_owned(font_data).into(),
                    );
                    
                    fonts.families.get_mut(&egui::FontFamily::Proportional)
                        .unwrap()
                        .insert(0, "SimHei".to_owned());
                }
            }
            
            cc.egui_ctx.set_fonts(fonts);
            
            Ok(Box::<SpeedyApp>::default())
        }),
    )
}
