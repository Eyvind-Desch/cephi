//! Default configuration files embedded into the binary.
//!
//! Sources live in `src-tauri/assets/`. When adding a default tool, register it in [`TOOLS`].

macro_rules! asset {
    ($path:literal) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/", $path))
    };
}

pub const SETTINGS: &str = asset!("defaults/settings.json");

pub const SCHEMAS: &[(&str, &str)] = &[
    ("settings.schema.json", asset!("schemas/settings.schema.json")),
    ("tool.schema.json", asset!("schemas/tool.schema.json")),
];

/// `(path relative to tools/, content)`
pub const TOOLS: &[(&str, &str)] = &[
    ("apps/open_app.json", asset!("defaults/tools/apps/open_app.json")),
    (
        "media/spotify_play.json",
        asset!("defaults/tools/media/spotify_play.json"),
    ),
    (
        "media/media_control.json",
        asset!("defaults/tools/media/media_control.json"),
    ),
    (
        "web/google_search.json",
        asset!("defaults/tools/web/google_search.json"),
    ),
    (
        "web/youtube_search.json",
        asset!("defaults/tools/web/youtube_search.json"),
    ),
    ("web/open_website.json", asset!("defaults/tools/web/open_website.json")),
    ("web/weather.json", asset!("defaults/tools/web/weather.json")),
    ("system/datetime.json", asset!("defaults/tools/system/datetime.json")),
    (
        "system/clipboard_read.json",
        asset!("defaults/tools/system/clipboard_read.json"),
    ),
    (
        "system/clipboard_write.json",
        asset!("defaults/tools/system/clipboard_write.json"),
    ),
    (
        "system/open_explorer.json",
        asset!("defaults/tools/system/open_explorer.json"),
    ),
    (
        "system/open_settings.json",
        asset!("defaults/tools/system/open_settings.json"),
    ),
    (
        "system/system_info.json",
        asset!("defaults/tools/system/system_info.json"),
    ),
    ("system/lock_pc.json", asset!("defaults/tools/system/lock_pc.json")),
    ("system/radio.json", asset!("defaults/tools/system/radio.json")),
    (
        "system/run_powershell.json",
        asset!("defaults/tools/system/run_powershell.json"),
    ),
];
