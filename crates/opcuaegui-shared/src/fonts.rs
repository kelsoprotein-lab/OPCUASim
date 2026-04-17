use std::path::Path;
use std::sync::Arc;

const CANDIDATES: &[&str] = &[
    #[cfg(target_os = "macos")]
    "/System/Library/Fonts/PingFang.ttc",
    #[cfg(target_os = "macos")]
    "/System/Library/Fonts/Hiragino Sans GB.ttc",
    #[cfg(target_os = "macos")]
    "/System/Library/Fonts/STHeiti Medium.ttc",
    #[cfg(target_os = "windows")]
    "C:\\Windows\\Fonts\\msyh.ttc",
    #[cfg(target_os = "windows")]
    "C:\\Windows\\Fonts\\msyh.ttf",
    #[cfg(target_os = "windows")]
    "C:\\Windows\\Fonts\\simhei.ttf",
    #[cfg(target_os = "linux")]
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    #[cfg(target_os = "linux")]
    "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
    #[cfg(target_os = "linux")]
    "/usr/share/fonts/truetype/arphic/uming.ttc",
];

pub fn install_cjk_fonts(ctx: &egui::Context) {
    let Some((path, bytes)) = load_first_available() else {
        log::warn!("no CJK font found on system; Chinese glyphs will render as tofu");
        return;
    };
    log::info!("loaded CJK font: {}", path);

    let mut fonts = egui::FontDefinitions::default();
    let name = "cjk";
    fonts
        .font_data
        .insert(name.to_owned(), Arc::new(egui::FontData::from_owned(bytes)));
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(1, name.to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push(name.to_owned());
    ctx.set_fonts(fonts);
}

fn load_first_available() -> Option<(&'static str, Vec<u8>)> {
    for path in CANDIDATES {
        if Path::new(path).exists() {
            match std::fs::read(path) {
                Ok(bytes) => return Some((path, bytes)),
                Err(e) => log::warn!("failed to read {}: {}", path, e),
            }
        }
    }
    None
}
