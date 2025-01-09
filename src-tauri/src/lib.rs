use tauri::Manager;



pub mod asm;
pub mod config;
pub mod database;
pub mod dns_collect;
pub mod plugin;
pub mod task;
pub mod utils;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

use plugin::plugin::{edit_plugin, get_plugins, new_plugin};

use asm::{
    domain::get_domains,
    enterprise::get_enterprise_list,
    rootdomain::{add_root_domain, del_rootdomain_by_id, get_ent_domain, get_root_domains},
};

#[tauri::command]
async fn close_splashscreen(window: tauri::Window) {
    // 关闭初始屏幕
    if let Some(splashscreen) = window.get_webview_window("splashscreen") {
        std::thread::sleep(std::time::Duration::from_secs(2));
        splashscreen.close().unwrap();
    }
    // 显示主窗口
    window.get_webview_window("main").unwrap().show().unwrap();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // let menu = Menu::new()?;
    tauri::Builder::default()
        // .menu()
        // .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            close_splashscreen,
            get_enterprise_list,
            add_root_domain,
            get_root_domains,
            get_ent_domain,
            del_rootdomain_by_id,
            get_domains,
            get_plugins,
            new_plugin,
            edit_plugin
        ])
        .setup(|app| {
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Show RShiled", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            TrayIconBuilder::new()
                .menu(&menu)
                .menu_on_left_click(true)
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        app.show().unwrap();
                    }
                    _ => {
                        println!("menu item {:?} not handled", event.id);
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // tauri::WindowEvent::Focused(focused) => {
            //   if !focused {
            //     window.hide().unwrap();
            //   }
            // }
            tauri::WindowEvent::CloseRequested { api, .. } => {
                window.hide().unwrap();
                api.prevent_close();
            }

            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
