#![allow(unexpected_cfgs)]

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

/// Set the macOS dock icon from embedded PNG data.
#[cfg(target_os = "macos")]
fn set_dock_icon() {
    let icon_data = include_bytes!("../../../assets/icon.png");
    unsafe {
        use objc::runtime::{Class, Object};
        use objc::{msg_send, sel, sel_impl};
        use std::ffi::c_void;

        let ns_data_cls = Class::get("NSData").unwrap();
        let data: *mut Object = msg_send![
            ns_data_cls,
            dataWithBytes: icon_data.as_ptr() as *const c_void
            length: icon_data.len()
        ];

        let ns_image_cls = Class::get("NSImage").unwrap();
        let image: *mut Object = msg_send![ns_image_cls, alloc];
        let image: *mut Object = msg_send![image, initWithData: data];

        let ns_app_cls = Class::get("NSApplication").unwrap();
        let app: *mut Object = msg_send![ns_app_cls, sharedApplication];
        let _: () = msg_send![app, setApplicationIconImage: image];
    }
}

#[cfg(not(target_os = "macos"))]
fn set_dock_icon() {}

fn open_main_window(root: std::path::PathBuf, cx: &mut App) {
    let bounds = Bounds::centered(None, size(px(1200.), px(800.)), cx);

    #[cfg(target_os = "macos")]
    let titlebar = Some(TitlebarOptions {
        appears_transparent: true,
        traffic_light_position: Some(point(px(9.0), px(9.0))),
        ..Default::default()
    });
    #[cfg(not(target_os = "macos"))]
    let titlebar = None;

    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            focus: true,
            titlebar,
            ..Default::default()
        },
        |window, cx| {
            let app_view = cx.new(|cx| GhostAppView::new(root, true, window, cx));
            cx.new(|cx| Root::new(app_view, window, cx))
        },
    )
    .unwrap();
}

fn main() {
    let root = ghostmd_core::diary::ghostmd_root();
    std::fs::create_dir_all(&root).ok();

    let reopen_root = root.clone();
    let app = Application::new().with_assets(Assets);
    app.on_reopen(move |cx| {
        open_main_window(reopen_root.clone(), cx);
        cx.activate(true);
    });
    app.run(|cx: &mut App| {
            apply_ghost_theme(cx);
            register_keybindings(cx);
            set_dock_icon();

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

            open_main_window(root, cx);
            cx.activate(true);
        });
}
