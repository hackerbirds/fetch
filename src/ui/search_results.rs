use std::ops::Neg;

use gpui::{
    Context, ElementId, Fill, InteractiveElement, IntoElement, MouseButton, Negate, ParentElement,
    Pixels, Point, Render, ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Window,
    div, img, prelude::FluentBuilder,
};
use gpui_component::ActiveTheme;

use crate::ui::gpui_app::GpuiApp;

#[derive(Clone)]
pub struct SearchResultsList {
    pub(crate) results: Vec<GpuiApp>,
    scroll_handle: ScrollHandle,
}

const RESULT_EL_HEIGHT: usize = 44;
const RESULT_EL_PADDING: usize = 8;

impl SearchResultsList {
    #[must_use]
    pub fn new(results: Vec<GpuiApp>) -> Self {
        Self {
            results,
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
                let app_name = SharedString::from(app.name.clone());
                let path = app.path.clone();
                let app_icon = app.icon.clone();
                #[allow(
                    clippy::cast_precision_loss,
                    reason = "we don't need high precision, div el height is tiny"
                )]
                div()
                    .id(ElementId::named_usize(app_name.clone(), i))
                    .flex()
                    .items_center()
                    .p(Pixels::from(RESULT_EL_PADDING))
                    .min_h(Pixels::from(RESULT_EL_HEIGHT))
                    .h(Pixels::from(RESULT_EL_HEIGHT))
                    .pl(Pixels::from(40.0 / ((1.3 * i as f64) + 1.0).powf(1.4)))
                    .when(i == 0, |mut this| {
                        this.style().background =
                            Some(Fill::Color(cx.theme().secondary_hover.into()));

                        self.scroll_handle.set_offset(Point::new(
                            0f64.into(),
                            // 32px: height of el
                            // 8px: padding top
                            // 8px: padding bottom
                            ((i * (RESULT_EL_HEIGHT + 2 * RESULT_EL_PADDING)) as f64)
                                .neg()
                                .into(),
                        ));

                        this.pl_3().child(
                            div()
                                .relative()
                                .left(Pixels::from(RESULT_EL_PADDING).negate())
                                .w_6()
                                .h_6()
                                .ml_2()
                                .bg(cx.theme().sidebar_border)
                                .border_1()
                                .border_color(cx.theme().window_border)
                                .rounded_md()
                                .flex()
                                .items_center()
                                .justify_center()
                                .pt_1()
                                .child("↵"),
                        )
                    })
                    .hover(|style| style.bg(cx.theme().secondary_hover))
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        cx.open_with_system(path.as_path());
                        window.remove_window();
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .when_some(app_icon, |this, icon_img| {
                                this.child(
                                    img(icon_img)
                                        .h(Pixels::from(RESULT_EL_HEIGHT - RESULT_EL_PADDING))
                                        .w(Pixels::from(RESULT_EL_HEIGHT - RESULT_EL_PADDING))
                                        .p(Pixels::from(RESULT_EL_PADDING)),
                                )
                            })
                            .child(div().child(app_name).text_xl()),
                    )
            }))
    }
}
