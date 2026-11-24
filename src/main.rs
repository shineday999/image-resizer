use eframe::egui;
use image::{codecs::jpeg::JpegEncoder, imageops::FilterType, DynamicImage, ImageFormat};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

const EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "bmp", "webp", "tif", "tiff", "ico", "tga", "pnm", "ppm", "pgm",
    "pbm", "qoi", "hdr", "exr", "ff",
];

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Percent,
    Width,
    Height,
    Exact,
}

#[derive(PartialEq, Clone, Copy)]
enum OutFmt {
    Original,
    Jpeg,
    Png,
    WebP,
    Bmp,
    Gif,
    Tiff,
}

impl OutFmt {
    const ALL: [OutFmt; 7] = [
        OutFmt::Original,
        OutFmt::Jpeg,
        OutFmt::Png,
        OutFmt::WebP,
        OutFmt::Bmp,
        OutFmt::Gif,
        OutFmt::Tiff,
    ];
    fn label(&self) -> &'static str {
        match self {
            OutFmt::Original => "保持原格式",
            OutFmt::Jpeg => "JPEG",
            OutFmt::Png => "PNG",
            OutFmt::WebP => "WebP",
            OutFmt::Bmp => "BMP",
            OutFmt::Gif => "GIF",
            OutFmt::Tiff => "TIFF",
        }
    }
}

enum Msg {
    Item(String),
    Done,
}

fn is_image(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| EXTS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn process(
    path: &Path,
    out_dir: &Option<PathBuf>,
    mode: Mode,
    percent: f32,
    w: u32,
    h: u32,
    fmt: OutFmt,
    quality: u8,
    overwrite: bool,
) -> Result<PathBuf, String> {
    let img = image::open(path).map_err(|e| e.to_string())?;
    let (ow, oh) = (img.width() as f64, img.height() as f64);
    let (nw, nh) = match mode {
        Mode::Percent => (
            (ow * percent as f64 / 100.0).round().max(1.0) as u32,
            (oh * percent as f64 / 100.0).round().max(1.0) as u32,
        ),
        Mode::Width => (w.max(1), (oh * w as f64 / ow).round().max(1.0) as u32),
        Mode::Height => ((ow * h as f64 / oh).round().max(1.0) as u32, h.max(1)),
        Mode::Exact => (w.max(1), h.max(1)),
    };
    let resized = img.resize_exact(nw, nh, FilterType::Lanczos3);

    let target = match fmt {
        OutFmt::Original => ImageFormat::from_path(path).map_err(|e| e.to_string())?,
        OutFmt::Jpeg => ImageFormat::Jpeg,
        OutFmt::Png => ImageFormat::Png,
        OutFmt::WebP => ImageFormat::WebP,
        OutFmt::Bmp => ImageFormat::Bmp,
        OutFmt::Gif => ImageFormat::Gif,
        OutFmt::Tiff => ImageFormat::Tiff,
    };
    let ext = if fmt == OutFmt::Original {
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
            .to_string()
    } else {
        target.extensions_str()[0].to_string()
    };

    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
    let (out, write_to) = if overwrite {
        let dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        let out = dir.join(format!("{}.{}", stem, ext));
        let tmp = dir.join(format!(".{}.tmp_resize", stem));
        (out, tmp)
    } else {
        let dir = match out_dir {
            Some(d) => d.clone(),
            None => path.parent().map(|p| p.to_path_buf()).unwrap_or_default(),
        };
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let out = dir.join(format!("{}_resized.{}", stem, ext));
        (out.clone(), out)
    };

    let res: Result<(), String> = if target == ImageFormat::Jpeg {
        let rgb = DynamicImage::ImageRgb8(resized.to_rgb8());
        std::fs::File::create(&write_to)
            .map_err(|e| e.to_string())
            .and_then(|f| {
                let enc = JpegEncoder::new_with_quality(BufWriter::new(f), quality);
                rgb.write_with_encoder(enc).map_err(|e| e.to_string())
            })
    } else {
        resized
            .save_with_format(&write_to, target)
            .map_err(|e| e.to_string())
    };
    if let Err(e) = res {
        if overwrite {
            let _ = std::fs::remove_file(&write_to);
        }
        return Err(e);
    }
    if overwrite {
        std::fs::rename(&write_to, &out).map_err(|e| e.to_string())?;
        if out != path {
            let _ = std::fs::remove_file(path);
        }
    }
    Ok(out)
}

struct App {
    files: Vec<PathBuf>,
    mode: Mode,
    percent: f32,
    width: u32,
    height: u32,
    fmt: OutFmt,
    quality: u8,
    out_dir: Option<PathBuf>,
    overwrite: bool,
    log: Vec<String>,
    rx: Option<mpsc::Receiver<Msg>>,
    done: usize,
    running: bool,
}

impl App {
    fn new() -> Self {
        Self {
            files: vec![],
            mode: Mode::Percent,
            percent: 50.0,
            width: 1280,
            height: 720,
            fmt: OutFmt::Original,
            quality: 90,
            out_dir: None,
            overwrite: false,
            log: vec![],
            rx: None,
            done: 0,
            running: false,
        }
    }

    fn add(&mut self, v: Vec<PathBuf>) {
        for p in v {
            if is_image(&p) && !self.files.contains(&p) {
                self.files.push(p);
            }
        }
    }

    fn start(&mut self, ctx: &egui::Context) {
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.running = true;
        self.done = 0;
        self.log.clear();
        let files = self.files.clone();
        let out_dir = self.out_dir.clone();
        let (mode, percent, w, h, fmt, q, ov) = (
            self.mode,
            self.percent,
            self.width,
            self.height,
            self.fmt,
            self.quality,
            self.overwrite,
        );
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            for p in files {
                let msg = match process(&p, &out_dir, mode, percent, w, h, fmt, q, ov) {
                    Ok(o) => format!("OK  {}", o.display()),
                    Err(e) => format!("失敗  {}: {}", p.display(), e),
                };
                let _ = tx.send(Msg::Item(msg));
                ctx.request_repaint();
            }
            let _ = tx.send(Msg::Done);
            ctx.request_repaint();
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let msgs: Vec<Msg> = self
            .rx
            .as_ref()
            .map(|rx| rx.try_iter().collect())
            .unwrap_or_default();
        for m in msgs {
            match m {
                Msg::Item(s) => {
                    self.done += 1;
                    self.log.push(s);
                }
                Msg::Done => self.running = false,
            }
        }

        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        if !dropped.is_empty() {
            self.add(dropped);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("圖片調整工具");
            ui.horizontal(|ui| {
                if ui.button("選擇圖片").clicked() {
                    if let Some(v) = rfd::FileDialog::new()
                        .add_filter("Images", EXTS)
                        .pick_files()
                    {
                        self.add(v);
                    }
                }
                if ui.button("加入資料夾").clicked() {
                    if let Some(d) = rfd::FileDialog::new().pick_folder() {
                        if let Ok(rd) = std::fs::read_dir(d) {
                            let v: Vec<PathBuf> = rd
                                .filter_map(|e| e.ok())
                                .map(|e| e.path())
                                .filter(|p| is_image(p))
                                .collect();
                            self.add(v);
                        }
                    }
                }
                if ui.button("清空").clicked() {
                    self.files.clear();
                }
                ui.label(format!("共 {} 張(可拖曳檔案進來)", self.files.len()));
            });

            let mut remove: Option<usize> = None;
            egui::ScrollArea::vertical()
                .id_salt("files")
                .max_height(200.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (i, p) in self.files.iter().enumerate() {
                        ui.horizontal(|ui| {
                            if ui.small_button("x").clicked() {
                                remove = Some(i);
                            }
                            ui.label(p.display().to_string());
                        });
                    }
                });
            if let Some(i) = remove {
                self.files.remove(i);
            }

            ui.separator();
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.mode, Mode::Percent, "百分比");
                ui.radio_value(&mut self.mode, Mode::Width, "指定寬度(等比)");
                ui.radio_value(&mut self.mode, Mode::Height, "指定高度(等比)");
                ui.radio_value(&mut self.mode, Mode::Exact, "指定寬x高");
            });
            match self.mode {
                Mode::Percent => {
                    ui.add(egui::Slider::new(&mut self.percent, 1.0..=400.0).suffix("%"));
                }
                Mode::Width => {
                    ui.horizontal(|ui| {
                        ui.label("寬度 px");
                        ui.add(egui::DragValue::new(&mut self.width).range(1..=30000));
                    });
                }
                Mode::Height => {
                    ui.horizontal(|ui| {
                        ui.label("高度 px");
                        ui.add(egui::DragValue::new(&mut self.height).range(1..=30000));
                    });
                }
                Mode::Exact => {
                    ui.horizontal(|ui| {
                        ui.label("寬 px");
                        ui.add(egui::DragValue::new(&mut self.width).range(1..=30000));
                        ui.label("高 px");
                        ui.add(egui::DragValue::new(&mut self.height).range(1..=30000));
                    });
                }
            }

            ui.horizontal(|ui| {
                egui::ComboBox::from_label("輸出格式")
                    .selected_text(self.fmt.label())
                    .show_ui(ui, |ui| {
                        for f in OutFmt::ALL {
                            ui.selectable_value(&mut self.fmt, f, f.label());
                        }
                    });
                ui.label("JPEG 品質");
                ui.add(egui::Slider::new(&mut self.quality, 1..=100));
            });

            ui.checkbox(&mut self.overwrite, "覆蓋原檔案(無法復原)");
            ui.horizontal(|ui| {
                if ui.button("選擇輸出資料夾").clicked() {
                    if let Some(d) = rfd::FileDialog::new().pick_folder() {
                        self.out_dir = Some(d);
                    }
                }
                if ui.button("輸出到原資料夾").clicked() {
                    self.out_dir = None;
                }
                ui.label(match &self.out_dir {
                    _ if self.overwrite => "(將覆蓋原檔案)".to_string(),
                    Some(d) => d.display().to_string(),
                    None => "(與原圖相同資料夾,檔名加 _resized)".to_string(),
                });
            });

            ui.separator();
            ui.horizontal(|ui| {
                let can = !self.files.is_empty() && !self.running;
                let btn = egui::Button::new(
                    egui::RichText::new("▶ 開始調整")
                        .size(28.0)
                        .strong()
                        .color(egui::Color32::WHITE),
                )
                .fill(if can {
                    egui::Color32::from_rgb(30, 150, 60)
                } else {
                    egui::Color32::from_gray(90)
                })
                .min_size(egui::vec2(260.0, 64.0));
                if ui.add_enabled(can, btn).clicked() {
                    self.start(ctx);
                }
                if self.running || self.done > 0 {
                    ui.add(
                        egui::ProgressBar::new(self.done as f32 / self.files.len().max(1) as f32)
                            .text(format!("{}/{}", self.done, self.files.len())),
                    );
                }
            });

            egui::ScrollArea::vertical()
                .id_salt("log")
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for l in &self.log {
                        ui.label(l);
                    }
                });
        });
    }
}

fn setup_style(ctx: &egui::Context) {
    use egui::{FontFamily, FontId, TextStyle};
    ctx.set_pixels_per_point(1.1);
    ctx.style_mut(|st| {
        st.text_styles = [
            (TextStyle::Heading, FontId::new(30.0, FontFamily::Proportional)),
            (TextStyle::Body, FontId::new(20.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(20.0, FontFamily::Proportional)),
            (TextStyle::Small, FontId::new(16.0, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(18.0, FontFamily::Monospace)),
        ]
        .into();
        st.spacing.item_spacing = egui::vec2(12.0, 10.0);
        st.spacing.button_padding = egui::vec2(14.0, 8.0);
        st.spacing.interact_size.y = 36.0;
        st.spacing.slider_width = 260.0;
    });
}

fn setup_fonts(ctx: &egui::Context) {
    let candidates = [
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
    ];
    for p in candidates {
        if let Ok(bytes) = std::fs::read(p) {
            let mut fonts = egui::FontDefinitions::default();
            fonts
                .font_data
                .insert("cjk".to_owned(), egui::FontData::from_owned(bytes));
            for fam in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts.families.get_mut(&fam).unwrap().push("cjk".to_owned());
            }
            ctx.set_fonts(fonts);
            break;
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 820.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "Image Resizer",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            setup_style(&cc.egui_ctx);
            Ok(Box::new(App::new()))
        }),
    )
}
