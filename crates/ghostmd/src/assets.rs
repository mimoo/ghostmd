use std::borrow::Cow;

use anyhow::Result;
use gpui::{AssetSource, SharedString};
use rust_embed::RustEmbed;

/// App-specific assets (fonts, etc.) embedded from `assets/` directory.
#[derive(RustEmbed)]
#[folder = "../../assets"]
struct AppAssets;

/// Composite asset source: tries gpui-component icons first, then app assets.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        // Try gpui-component assets first (icons)
        if let Ok(Some(data)) = gpui_component_assets::Assets.load(path) {
            return Ok(Some(data));
        }
        // Fall back to app-embedded assets (fonts)
        Ok(AppAssets::get(path).map(|f| f.data))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut results: Vec<SharedString> = gpui_component_assets::Assets
            .list(path)
            .unwrap_or_default();
        for name in AppAssets::iter() {
            if name.starts_with(path) {
                results.push(name.into_owned().into());
            }
        }
        Ok(results)
    }
}
