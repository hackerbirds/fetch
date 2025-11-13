use std::{ops::Deref, path::PathBuf};

use crate::apps::App;
use gpui::{ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div};
use gpui_component::{h_flex, select::SelectItem};

#[derive(IntoElement, Clone)]
pub struct AppEntry {
    pub app: App,
    pub element_id: ElementId,
}

impl AppEntry {
    #[must_use]
    pub fn new(app: App) -> Self {
        let element_id = ElementId::Name(app.name.clone().into());
        Self { app, element_id }
    }
}

impl SelectItem for AppEntry {
    type Value = PathBuf;

    fn title(&self) -> SharedString {
        self.app.name.clone().into()
    }

    fn display_title(&self) -> Option<gpui::AnyElement> {
        Some(
            h_flex()
                .items_center()
                .gap_2()
                .child(self.title())
                .into_any_element(),
        )
    }

    fn value(&self) -> &Self::Value {
        &self.app.path
    }
}

impl RenderOnce for AppEntry {
    fn render(self, _window: &mut Window, _cx: &mut gpui::App) -> impl IntoElement {
        div()
            .max_h_8()
            .size_full()
            .items_center()
            .justify_center()
            .child(self.app.name.deref().to_string())
    }
}
