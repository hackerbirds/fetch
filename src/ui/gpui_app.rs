use std::{path::PathBuf, sync::Arc};

use gpui::{ImageFormat, RenderImage, SharedString};

use crate::extensions::SearchResult;

/// This struct contains the elements used to render an app in the search results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuiApp {
    pub(super) name: SharedString,
    pub(super) path: PathBuf,
    pub(super) icon: Option<Arc<RenderImage>>,
}

/// This loads apps ready for gpui to render, with
/// an internal cache.
pub struct GpuiAppLoader(scc::HashMap<SearchResult, GpuiApp>);

impl Default for GpuiAppLoader {
    fn default() -> Self {
        Self(scc::HashMap::new())
    }
}

impl GpuiAppLoader {
    pub fn load(&self, result: &SearchResult, cx: &gpui::App) -> GpuiApp {
        if let Some(cached_entry) = self.0.get_sync(result) {
            cached_entry.get().clone()
        } else {
            match result.clone() {
                SearchResult::Executable(executable_app) => {
                    let icon = executable_app
                        .icon_png_data
                        .clone()
                        .and_then(|data: Vec<u8>| {
                            let im = gpui::Image::from_bytes(ImageFormat::Png, data);
                            im.to_image_data(cx.svg_renderer()).ok()
                        });

                    let gpui_app = GpuiApp {
                        name: SharedString::from(executable_app.name),
                        path: executable_app.path,
                        icon,
                    };

                    let _ = self.0.insert_sync(result.clone(), gpui_app.clone());

                    gpui_app
                }
            }
        }
    }
}
