mod network_monitor;

use eframe::egui;
use network_monitor::{NetworkMonitor, NetworkStats, format_bytes, format_total_bytes};
use std::time::{Duration, Instant};
use std::cmp::Ordering;
const STORAGE_KEY: &str = "speedy.sort_mode";

struct SpeedyApp {
    network_monitor: NetworkMonitor,
    network_stats: Vec<NetworkStats>,
    last_update: Instant,
    update_interval: Duration,
    always_on_top: bool,
    first_frame: bool,
    sort_mode: SortMode,
    search_query: String,
}

impl Default for SpeedyApp {
    fn default() -> Self {
        Self {
            network_monitor: NetworkMonitor::new(),
            network_stats: Vec::new(),
            last_update: Instant::now(),
            update_interval: Duration::from_secs(1),
            always_on_top: true,
            first_frame: true,
            sort_mode: SortMode::Name,
            search_query: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortMode {
    Name,
    Download,
}

impl eframe::App for SpeedyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply always-on-top on first frame (since builder settings don't work reliably)
        if self.first_frame && self.always_on_top {
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                egui::WindowLevel::AlwaysOnTop,
            ));
            self.first_frame = false;
        }

        // Update network stats periodically
        if self.last_update.elapsed() >= self.update_interval {
            self.network_stats = self.network_monitor.refresh();
            self.last_update = Instant::now();
        }

        // Request repaint to keep updating
        ctx.request_repaint_after(self.update_interval);

        egui::CentralPanel::default().show(ctx, |ui| {
            // Controls
            ui.horizontal(|ui| {
                ui.separator();
                ui.label("Search:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("Filter by name")
                        .desired_width(80.0), // ~10 ASCII chars
                );
                ui.separator();
                ui.label("Sort:");
                ui.selectable_value(&mut self.sort_mode, SortMode::Name, "Name");
                ui.selectable_value(&mut self.sort_mode, SortMode::Download, "Download");
                ui.separator();
                if ui
                    .checkbox(&mut self.always_on_top, "Always on top")
                    .changed()
                {
                    // Try to update always-on-top behavior
                    ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                        if self.always_on_top {
                            egui::WindowLevel::AlwaysOnTop
                        } else {
                            egui::WindowLevel::Normal
                        },
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

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let s = match self.sort_mode {
            SortMode::Name => "Name",
            SortMode::Download => "Download",
        };
        storage.set_string(STORAGE_KEY, s.to_string());
    }
}

impl SpeedyApp {
    fn show_network_interfaces(&self, ui: &mut egui::Ui) {
        use egui::{Color32, RichText};

        // helper to pick color for a speed value
        let speed_color = |value: f64| -> Color32 {
            if value > 1024.0 * 1024.0 {
                Color32::from_rgb(0, 200, 0)
            } else if value > 1024.0 {
                Color32::from_rgb(200, 150, 0)
            } else {
                Color32::from_rgb(80, 80, 80)
            }
        };

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Sort interfaces according to user's choice. We create a vector
            // of (index, &NetworkStats) so we can use the original index as
            // a stable tiebreaker.
            let mut indexed: Vec<(usize, &network_monitor::NetworkStats)> =
                self.network_stats.iter().enumerate().collect();

            // Apply search filter (case-insensitive) before sorting
            let query = self.search_query.to_lowercase();
            if !query.is_empty() {
                indexed.retain(|(_i, s)| s.name.to_lowercase().contains(&query));
            }

            match self.sort_mode {
                SortMode::Name => indexed.sort_by(|(i, a), (j, b)| {
                    let ord = a.name.to_lowercase().cmp(&b.name.to_lowercase());
                    if ord != Ordering::Equal {
                        ord
                    } else {
                        i.cmp(j)
                    }
                }),
                SortMode::Download => indexed.sort_by(|(i, a), (j, b)| {
                    // Descending by download_speed
                    match b
                        .download_speed
                        .partial_cmp(&a.download_speed)
                        .unwrap_or(Ordering::Equal)
                    {
                        Ordering::Equal => i.cmp(j),
                        other => other,
                    }
                }),
            }

            for (_idx, stats) in indexed {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        // Interface name
                        ui.label(RichText::new(&stats.name).strong().size(16.0));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!(
                                "Total: Down:{} Up:{}",
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
                                ui.label(
                                    RichText::new("Download")
                                        .color(Color32::from_rgb(20, 100, 200)),
                                );
                                let speed_text = format_bytes(stats.download_speed);
                                let speed_color = speed_color(stats.download_speed);
                                // Ensure a minimum width so values align between download/upload
                                const SPEED_MIN_W: f32 = 110.0;
                                const SPEED_H: f32 = 28.0;
                                ui.add_sized(
                                    [SPEED_MIN_W, SPEED_H],
                                    egui::Label::new(
                                        RichText::new(speed_text).color(speed_color).size(18.0).strong(),
                                    ),
                                );
                            });
                        });

                        ui.add_space(20.0);

                        // Upload speed
                        ui.group(|ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new("Upload").color(Color32::from_rgb(200, 100, 20)),
                                );
                                let speed_text = format_bytes(stats.upload_speed);
                                let speed_color = speed_color(stats.upload_speed);
                                // Ensure the same minimum width as download
                                const SPEED_MIN_W: f32 = 110.0;
                                const SPEED_H: f32 = 28.0;
                                ui.add_sized(
                                    [SPEED_MIN_W, SPEED_H],
                                    egui::Label::new(
                                        RichText::new(speed_text).color(speed_color).size(18.0).strong(),
                                    ),
                                );
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
    // Estimate an initial window width based on the top control line (search, sort, labels).
    // This is a simple heuristic (avg char width * chars + padding) that adapts the
    // initial size to the UI content so the first line is unlikely to be clipped.
    fn estimate_initial_width() -> f32 {
        // Rough average character width in px for typical UI font.
        const AVG_CHAR_W: f32 = 8.0;

        // Components we display on the first row (approximate character counts):
        let search_label = "Search:".len();
        let search_box_chars = 10; // user requested ~10 ASCII chars
        let sort_label = "Sort:".len();
        let name_label = "Name".len();
        let download_label = "Download".len();
        let always_label = "Always on top".len();
        let total_label = "Total interfaces: 999".len(); // reserve space for counts

        let char_count = search_label + search_box_chars + sort_label + name_label + download_label + always_label + total_label;

        // Add padding for separators, margins and icon area
        let padding = 140.0_f32;
        let width = (char_count as f32) * AVG_CHAR_W + padding;

        // Clamp to reasonable bounds
        width.clamp(420.0, 1400.0)
    }

    let initial_width = estimate_initial_width();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([initial_width, 360.0])
            .with_min_inner_size([450.0, 300.0])
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
                    fonts
                        .families
                        .get_mut(&egui::FontFamily::Proportional)
                        .unwrap()
                        .insert(0, "Microsoft YaHei".to_owned());
                } else if let Ok(font_data) = std::fs::read("C:\\Windows\\Fonts\\simhei.ttf") {
                    fonts.font_data.insert(
                        "SimHei".to_owned(),
                        egui::FontData::from_owned(font_data).into(),
                    );

                    fonts
                        .families
                        .get_mut(&egui::FontFamily::Proportional)
                        .unwrap()
                        .insert(0, "SimHei".to_owned());
                }
            }

            cc.egui_ctx.set_fonts(fonts);

            // Initialize app and restore saved settings (sort mode)
            let mut app = SpeedyApp::default();
            if let Some(storage) = &cc.storage {
                if let Some(val) = storage.get_string(STORAGE_KEY) {
                    app.sort_mode = match val.as_str() {
                        "Download" => SortMode::Download,
                        _ => SortMode::Name,
                    }
                }
            }

            Ok(Box::new(app))
        }),
    )
}
