// Loads `locales/*.json` at compile time; missing translations fall back to English.
rust_i18n::i18n!("locales", fallback = "en");

mod agent;
mod app;
mod commands;
mod config;
mod desktop;
mod error;
mod history;
mod i18n;
mod integrations;
mod launcher;
mod llm;
mod mcp;
mod migrate;
mod notes;
mod search;
mod secrets;
mod services;
mod state;
mod tools;
mod translate;
mod voice;

use tauri::{Manager, RunEvent, WindowEvent};
use tauri_plugin_log::{Target, TargetKind};

use desktop::window;

pub fn run() {
    tauri::Builder::default()
        // Must be registered first: a second launch just brings the running instance to front.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Err(e) = window::show(app) {
                log::error!("show window: {e}");
            }
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir { file_name: None }),
                ])
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app::setup(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == window::MAIN_LABEL && matches!(event, WindowEvent::Focused(false)) {
                window::on_focus_lost(window.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::config_snapshot,
            commands::config_reload,
            commands::config_open_folder,
            commands::settings_update,
            commands::mcp_save_server,
            commands::mcp_remove_server,
            commands::mcp_login,
            commands::mcp_logout,
            commands::agent_send,
            commands::screenshot_capture,
            commands::screenshot_region,
            commands::screenshot_region_image,
            commands::screenshot_region_ready,
            commands::screenshot_region_done,
            commands::screenshot_region_cancel,
            commands::voice_start,
            commands::voice_stop,
            commands::voice_cancel,
            commands::agent_continue,
            commands::agent_preload,
            commands::agent_cancel,
            commands::agent_confirm,
            commands::agent_reset,
            commands::chat_load,
            commands::chat_save,
            commands::chat_archive_list,
            commands::chat_archive_open,
            commands::chat_archive_delete,
            commands::notes_list,
            commands::notes_save,
            commands::notes_delete,
            commands::window_set_mode,
            commands::window_hide,
            commands::window_set_pinned,
            commands::autostart_state,
            commands::autostart_set,
            commands::open_external,
            commands::web_state,
            commands::web_navigate,
            commands::web_open,
            commands::web_set_layout,
            commands::web_command,
            commands::translate,
            commands::services_status,
            commands::service_start,
            commands::secrets_list,
            commands::secrets_set,
            commands::secrets_delete,
            commands::secrets_test,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Cephi")
        .run(|app, event| {
            // MCP servers are child processes: end them together with Cephi.
            if let RunEvent::Exit = event {
                if let Some(state) = app.try_state::<state::AppState>() {
                    tauri::async_runtime::block_on(state.mcp.shutdown());
                }
            }
        });
}
