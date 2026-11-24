# Image Resizer 圖片調整工具

A GUI image resizing tool for Ubuntu, written in Rust (egui/eframe). Resize a single image or a whole batch at once.
一個用 Rust 撰寫的 Ubuntu 圖形介面圖片調整工具,可單張或批次調整尺寸。

## Features 功能

- Select images, add a whole folder, or drag and drop files into the window
- Batch processing with a progress bar and a result log
- Resize modes: percentage, fixed width (keep ratio), fixed height (keep ratio), exact width x height
- Output formats: keep original, JPEG, PNG, WebP, BMP, GIF, TIFF
- Adjustable JPEG quality
- Choose an output folder, save next to the original (`_resized` suffix), or overwrite the original file
- Supported input formats: PNG, JPG/JPEG, GIF, BMP, WebP, TIFF, ICO, TGA, PNM/PPM/PGM/PBM, QOI, HDR, EXR, Farbfeld
- Traditional Chinese UI, large readable fonts

## Requirements 需求

- Ubuntu (tested with GTK3 / X11 / Wayland)
- Rust toolchain (cargo)

```bash
sudo apt update && sudo apt install -y build-essential pkg-config curl git \
  libgtk-3-dev libxkbcommon-dev libwayland-dev libx11-dev libxcursor-dev \
  libxrandr-dev libxi-dev libgl1-mesa-dev fonts-noto-cjk
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

## Build and Run 編譯與執行

```bash
git clone https://github.com/shineday999/image-resizer.git
cd image-resizer
cargo build --release
./target/release/image-resizer
```

## Install to App Menu 安裝到應用程式選單

```bash
install -Dm755 target/release/image-resizer ~/.local/bin/image-resizer
cp packaging/image-resizer.desktop ~/.local/share/applications/
update-desktop-database ~/.local/share/applications 2>/dev/null
```

## Usage 使用方式

1. Click "選擇圖片" (or drag files in) to add images.
2. Pick a resize mode and value.
3. Pick the output format and save location.
4. Click "開始調整".

> Warning: "覆蓋原檔案" replaces the original files and cannot be undone.

## License

MIT
