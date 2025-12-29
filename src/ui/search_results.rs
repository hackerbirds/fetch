use std::{ops::Neg, sync::Arc};

use gpui::{
    Context, ElementId, Fill, InteractiveElement, IntoElement, MouseButton, ParentElement, Point,
    Render, RenderImage, ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Window,
    div, img, prelude::FluentBuilder,
};
use gpui_component::ActiveTheme;

#[derive(Clone)]
pub struct SearchResultsList {
    pub(crate) results: Vec<(crate::apps::App, Option<Arc<RenderImage>>)>,
    selected_result: usize,
    scroll_handle: ScrollHandle,
}

impl SearchResultsList {
    #[must_use]
    pub fn new(
        results: Vec<(crate::apps::App, Option<Arc<RenderImage>>)>,
        selected_result: usize,
    ) -> Self {
        Self {
            results,
            selected_result,
            scroll_handle: ScrollHandle::new(),
        }
    }
}

impl Render for SearchResultsList {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("apps-list")
            .size_full()
            .flex()
            .flex_col()
            .track_scroll(&self.scroll_handle)
            .children(self.results.iter().enumerate().map(|(i, app)| {
                let app_name = SharedString::from(app.0.name.clone());
                let path = app.0.path.clone();
                let app_icon = app.1.clone();

                div()
                    .id(ElementId::named_usize(app_name.clone(), i))
                    .p_0p5()
                    .pl_2()
                    // 32 px element
                    .min_h_8()
                    .h_8()
                    // 2px margin
                    .m_0p5()
                    .when(i == self.selected_result, |mut this| {
                        this.style().background =
                            Some(Fill::Color(cx.theme().secondary_hover.into()));

                        #[allow(
                            clippy::cast_precision_loss,
                            reason = "we don't need high precision, div el height is tiny"
                        )]
                        self.scroll_handle.set_offset(Point::new(
                            0f64.into(),
                            // 32px: height of el
                            // 2px: margin top
                            // 2px: margin bottom
                            ((i * (32 + 2 + 2)) as f64).neg().into(),
                        ));

                        this
                    })
                    .hover(|style| style.bg(cx.theme().secondary_hover))
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        cx.open_with_system(path.as_path());
                        window.remove_window();
                    })
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap_1()
                            .when_some(app_icon, |this, ic| this.child(img(ic).h_7().w_7()))
                            .child(app_name),
                    )
            }))
    }
}
