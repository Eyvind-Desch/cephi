use std::path::Path;
use std::time::Duration;

use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use tauri::{AppHandle, Manager};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Keeps the file watcher alive for the lifetime of the app.
struct WatcherGuard(#[allow(dead_code)] Debouncer<RecommendedWatcher>);

/// Watches the config folder and hot-reloads when a JSON file changes.
pub fn start(app: &AppHandle) -> AppResult<()> {
    let state = app.state::<AppState>();
    let schemas = state.paths.schemas.clone();
    let handle = app.clone();

    let mut debouncer = new_debouncer(Duration::from_millis(300), move |result: DebounceEventResult| {
        let Ok(events) = result else { return };
        let relevant = events.iter().any(|e| is_config_file(&e.path, &schemas));
        if relevant {
            let app = handle.clone();
            let _ = handle.run_on_main_thread(move || {
                crate::app::reload_config(&app);
            });
        }
    })
    .map_err(|e| AppError::msg(format!("failed to start file watcher: {e}")))?;

    debouncer
        .watcher()
        .watch(&state.paths.root, RecursiveMode::Recursive)
        .map_err(|e| AppError::msg(format!("failed to watch config folder: {e}")))?;

    app.manage(WatcherGuard(debouncer));
    Ok(())
}

fn is_config_file(path: &Path, schemas: &Path) -> bool {
    path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("json")) && !path.starts_with(schemas)
}
