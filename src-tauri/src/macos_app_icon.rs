use objc2::{AllocAnyThread, MainThreadMarker};
use objc2_app_kit::{NSApplication, NSImage};
use objc2_foundation::NSData;
use tauri::Theme;

const LIGHT_ICON: &[u8] = include_bytes!("../icons/icon.png");
const DARK_ICON: &[u8] = include_bytes!("../icons/icon-dark.png");

pub fn set_dock_icon(theme: Theme) -> Result<(), String> {
    let data = NSData::with_bytes(icon_bytes(theme));
    let icon = NSImage::initWithData(NSImage::alloc(), &data)
        .ok_or_else(|| "macOS could not decode the selected Dock icon.".to_string())?;
    let main_thread = MainThreadMarker::new()
        .ok_or_else(|| "the macOS Dock icon must be updated on the main thread.".to_string())?;
    let application = NSApplication::sharedApplication(main_thread);

    unsafe { application.setApplicationIconImage(Some(&icon)) };
    Ok(())
}

fn icon_bytes(theme: Theme) -> &'static [u8] {
    match theme {
        Theme::Dark => DARK_ICON,
        _ => LIGHT_ICON,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_theme_icons_are_distinct_png_files() {
        assert!(LIGHT_ICON.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(DARK_ICON.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert_ne!(icon_bytes(Theme::Light), icon_bytes(Theme::Dark));
    }
}
