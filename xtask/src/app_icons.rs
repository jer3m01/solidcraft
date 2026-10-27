//! Regenerate all exports of the shared app mark together. Packaging copies these files.
use std::path::Path;

#[path = "../../assets/icons/app/shared/source/app_icon.rs"]
mod app_icon;
#[path = "../../assets/icons/app/shared/source/windows_ico.rs"]
mod windows_ico;

/// RGBA pixels (`w × h`) to PNG bytes, matching `solidcraft_render::png`.
fn encode_png(w: usize, h: usize, rgba: Vec<u8>) -> Result<Vec<u8>, String> {
    use image::{ImageEncoder, RgbaImage};
    let img = RgbaImage::from_raw(
        u32::try_from(w).map_err(|_| "icon too wide".to_string())?,
        u32::try_from(h).map_err(|_| "icon too tall".to_string())?,
        rgba,
    )
    .ok_or_else(|| "icon: bad RGBA buffer".to_string())?;
    let mut buf = Vec::new();
    image::codecs::png::PngEncoder::new(&mut buf)
        .write_image(
            img.as_raw(),
            u32::try_from(w).unwrap(),
            u32::try_from(h).unwrap(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| format!("icon: PNG encoding failed: {e}"))?;
    Ok(buf)
}

/// A Windows icon file holding PNG images (Vista and later read these), matching
/// `solidcraft-cli icon`'s `.ico` output.
fn ico(images: &[(usize, Vec<u8>)]) -> Result<Vec<u8>, String> {
    let n = u16::try_from(images.len()).map_err(|_| "too many icon sizes".to_string())?;
    let mut out = Vec::new();
    out.extend_from_slice(&[0, 0, 1, 0]);
    out.extend_from_slice(&n.to_le_bytes());
    let mut offset = 6 + 16 * images.len();
    for (size, data) in images {
        let dim = if *size >= 256 { 0 } else { *size as u8 };
        out.extend_from_slice(&[dim, dim, 0, 0, 1, 0, 32, 0]);
        out.extend_from_slice(&u32::try_from(data.len()).map_err(|_| "icon too large".to_string())?.to_le_bytes());
        out.extend_from_slice(&u32::try_from(offset).map_err(|_| "icon too large".to_string())?.to_le_bytes());
        offset += data.len();
    }
    for (_, data) in images {
        out.extend_from_slice(data);
    }
    Ok(out)
}

pub fn run(root: &Path) -> Result<(), String> {
    let stage = root.join("target/resources/app-icons");
    std::fs::create_dir_all(&stage).map_err(|e| e.to_string())?;
    let mut exports = Vec::new();
    for size in [16, 24, 32, 48, 64, 128, 256, 512] {
        let png = stage.join(format!("{size}.png"));
        let bytes = encode_png(size, size, app_icon::rgba(size).map_err(|e| e.to_string())?)?;
        std::fs::write(&png, &bytes).map_err(|e| e.to_string())?;
        exports.push((png, format!("linux/exports/hicolor/{size}x{size}/apps/ai.storyteller.solidcraft.png")));
    }
    for (density, size) in [("mdpi", 48), ("hdpi", 72), ("xhdpi", 96), ("xxhdpi", 144), ("xxxhdpi", 192)] {
        let png = stage.join(format!("android-{density}.png"));
        let bytes = encode_png(size, size, app_icon::rgba(size).map_err(|e| e.to_string())?)?;
        std::fs::write(&png, &bytes).map_err(|e| e.to_string())?;
        exports.push((png, format!("android/exports/res/mipmap-{density}/ic_launcher.png")));
    }
    let installer = stage.join("solidcraft.ico");
    let installer_bytes = ico(&[16, 20, 24, 32, 40, 48, 64, 128, 256]
        .into_iter()
        .map(|s| Ok((s, encode_png(s, s, app_icon::rgba(s).map_err(|e| e.to_string())?)?)))
        .collect::<Result<Vec<_>, String>>()?)?;
    std::fs::write(&installer, &installer_bytes).map_err(|e| e.to_string())?;
    exports.push((installer, "windows/exports/solidcraft.ico".into()));
    let executable = stage.join("solidcraft-executable.ico");
    std::fs::write(&executable, windows_ico::encode().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    exports.push((executable, "windows/exports/solidcraft-executable.ico".into()));

    // Finish generation before replacing any canonical exports.
    for (source, relative) in exports {
        let target = root.join("assets/icons/app").join(relative);
        if let Some(dir) = target.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::copy(source, target).map_err(|e| e.to_string())?;
    }
    Ok(())
}
