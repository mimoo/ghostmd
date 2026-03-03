#[cfg(test)]
mod ai;
mod app;
mod app_view;
mod assets;
#[cfg(test)]
mod editor;
mod editor_view;
mod file_tree;
mod file_tree_view;
mod keybindings;
mod palette;
mod search;
#[cfg(test)]
mod splits;
#[cfg(test)]
mod tabs;
mod theme;

use gpui::*;
use gpui_component::Root;

use app_view::GhostAppView;
use assets::Assets;
use keybindings::register_keybindings;
use theme::apply_ghost_theme;

fn main() {
    let root = ghostmd_core::diary::ghostmd_root();
    std::fs::create_dir_all(&root).ok();

    Application::new()
        .with_assets(Assets)
        .run(|cx: &mut App| {
            apply_ghost_theme(cx);
            register_keybindings(cx);

            // Load JetBrains Mono font
            if let Ok(Some(font)) = cx.asset_source().load("fonts/JetBrainsMono-Regular.ttf") {
                cx.text_system().add_fonts(vec![font]).ok();
            }
            if let Ok(Some(font)) = cx.asset_source().load("fonts/JetBrainsMono-Bold.ttf") {
                cx.text_system().add_fonts(vec![font]).ok();
            }
            if let Ok(Some(font)) = cx.asset_source().load("fonts/JetBrainsMono-Italic.ttf") {
                cx.text_system().add_fonts(vec![font]).ok();
            }

            let bounds = Bounds::centered(None, size(px(1200.), px(800.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    focus: true,
                    titlebar: Some(TitlebarOptions {
                        appears_transparent: true,
                        traffic_light_position: Some(point(px(9.0), px(9.0))),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |window, cx| {
                    let app_view = cx.new(|cx| GhostAppView::new(root, window, cx));
                    cx.new(|cx| Root::new(app_view, window, cx))
                },
            )
            .unwrap();
            cx.activate(true);
        });
}
