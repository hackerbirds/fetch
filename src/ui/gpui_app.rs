use std::{path::PathBuf, sync::Arc};

use gpui::{ImageFormat, RenderImage};

use crate::apps::{App, AppName};

/// This struct contains the elements used to render an app in the search results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuiApp {
    pub(super) name: AppName,
    pub(super) path: PathBuf,
    pub(super) icon: Option<Arc<RenderImage>>,
}

impl GpuiApp {
    pub fn load(app: App, cx: &gpui::App) -> Self {
        const IMAGE_FORMAT: ImageFormat = ImageFormat::Png;
        let image = gpui::Image::from_bytes(IMAGE_FORMAT, app.icon_png_img);
        let rendered_image = image.to_image_data(cx.svg_renderer()).ok();

        Self {
            name: app.name,
            path: app.path,
            icon: rendered_image,
        }
    }
}
