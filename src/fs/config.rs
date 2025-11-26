use gpui::Global;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    pub open_search_hotkey: HotkeyString,
    pub launch_on_boot: bool,
}

/// Format is "[Modifiers]-Key"
/// Key is a key code in a string format defined in [`global_hotkey::hotkey::Code`]
///
/// Examples:
///   - alt-space
///   - ctrl-win-KeyC
pub type HotkeyString = String;

impl Default for Configuration {
    fn default() -> Self {
        Self {
            open_search_hotkey: "alt-space".to_string(),
            launch_on_boot: true,
        }
    }
}

impl Global for Configuration {}
