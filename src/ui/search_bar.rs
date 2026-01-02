use std::ops::Neg;

use gpui::prelude::FluentBuilder;
use gpui::{
    AppContext, Context, Corners, ElementId, Entity, Fill, InteractiveElement, IntoElement,
    MouseButton, Negate, ParentElement, Pixels, Point, Render, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Window, div, img,
};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{ActiveTheme, StyledExt};

use crate::apps::app_string::AppString;
use crate::fs::config::config_file_path;
use crate::ui::gpui_app::GpuiApp;
use crate::ui::search_engine::GpuiSearchEngine;
use crate::{EnterPressed, EscPressed, OpenSettings, TabBackSelectApp, TabSelectApp};

pub struct SearchBar {
    search_engine: Entity<GpuiSearchEngine>,
    input_state: Entity<InputState>,
    all_queries: Vec<AppString>,
    #[expect(unused)]
    subscriptions: Vec<Subscription>,
    selected_result: usize,
    hovered_result: usize,
    scroll_handle: ScrollHandle,
}

/// The number of elements to render in gpui. This corresponds
/// to how many search results at once are physically able to
/// appear in the GUI (whose window height is a fixed size)
const MAX_RENDERED_ELS: usize = 4;
const RESULT_EL_HEIGHT: usize = 44;
const RESULT_EL_PADDING: usize = 8;

impl SearchBar {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        search_engine: Entity<GpuiSearchEngine>,
    ) -> Self {
        let input_state = cx.new(|cx| {
            let is = InputState::new(window, cx).placeholder("Search an app");
            is.focus(window, cx);
            is
        });

        let all_queries = vec![];

        let subscriptions = vec![cx.subscribe_in(&input_state, window, {
            let input_state = input_state.clone();
            move |this, _, ev: &InputEvent, window, cx| {
                if let InputEvent::Change = ev {
                    let value = input_state.read(cx).value();
                    let value: AppString = value.into();

                    this.search_engine.update(cx, |this, cx| {
                        this.deferred_search(cx, window, value.clone());
                    });
                    this.selected_result = 0;

                    this.all_queries.push(value);
                    cx.notify();
                }
            }
        })];

        Self {
            search_engine,
            input_state,
            all_queries,
            subscriptions,
            selected_result: 0,
            hovered_result: 0,
            scroll_handle: ScrollHandle::new(),
        }
    }
}

impl Render for SearchBar {
    #[allow(clippy::too_many_lines, reason = "Results entity needs refactor")]
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .p_2()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .bg(cx.theme().secondary)
            .on_action(cx.listener(|this, &TabSelectApp, _, cx| {
                let results_len = this.search_engine.read(cx).results.len();
                if results_len > 0 {
                    let wrap_around_needed = this.selected_result + this.hovered_result + 1 >= results_len;
                    if !wrap_around_needed && this.hovered_result <MAX_RENDERED_ELS-1 {
                        this.hovered_result += 1;
                    } else if wrap_around_needed {
                        this.selected_result = 0;
                        this.hovered_result = 0;
                    } else {
                        this.selected_result += 1;
                    }
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, &TabBackSelectApp, _, cx| {
                let results_len = this.search_engine.read(cx).results.len();
                if results_len > 0 {
                    if this.hovered_result > 0 {
                        this.hovered_result -= 1;
                    } else {
                        this.selected_result =
                            (this.selected_result + results_len - 1).rem_euclid(results_len);
                    }
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, &EscPressed, window, cx| {
                window.remove_window();
                this.search_engine.update(cx, |search_engine, cx| {
                    search_engine.update(cx);
                });
                cx.notify();
            }))
            .on_action(cx.listener(|_, &OpenSettings, window, cx| {
                window.remove_window();
                if let Ok(cfg_path) = config_file_path() {
                    cx.open_with_system(&cfg_path);
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, &EnterPressed, window, cx| {
                let app_opt = this
                    .search_engine
                    .read(cx)
                    .results
                    .get(this.selected_result)
                    .cloned();
                if let Some(app) = app_opt {
                    cx.open_with_system(app.path.as_path());
                    this.search_engine.update(cx, |search_engine, cx| {
                        search_engine.selected(cx, this.all_queries.clone(), app);
                    });
                    window.remove_window();
                }
                cx.notify();
            }))
            .child(
                Input::new(&self.input_state)
                    .bg(cx.theme().sidebar_border)
                    .corner_radii(Corners::all(10.0f64.into()))
                    .border_color(cx.theme().window_border)
                    .m_auto()
                    .h_16()
                    .text_xl(),
            )
            .child(
                div()
                    .v_flex()
                    .gap_2()
                    .size_full()
                    .overflow_y_hidden()
                    .child(
                        div()
                            .id("apps-list")
                            .size_full()
                            .flex()
                            .flex_col()
                            .track_scroll(&self.scroll_handle)
                            .children(self
                                .search_engine
                                .read(cx)
                                .results.clone().into_iter().skip(self.selected_result)
                                .take(MAX_RENDERED_ELS)
                                .map(|app| GpuiApp::load(app, cx)).enumerate().map(|(i, app)| {
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
                                    .pl(Pixels::from(40.0 / ((self.hovered_result.abs_diff(i) + 1) as f64).powf(1.67)))
                                    .when(i == self.hovered_result, |mut this| {
                                        this.style().background =
                                            Some(Fill::Color(cx.theme().secondary_hover.into()));

                                        self.scroll_handle.set_offset(Point::new(
                                            0f64.into(),
                                            // 32px: height of el
                                            // 8px: padding top
                                            // 8px: padding bottom
                                            ((i * (RESULT_EL_HEIGHT + 2 * RESULT_EL_PADDING))
                                                as f64)
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
                                    .on_hover(cx.listener(move |this, hovered, _window, cx| {
                                        if *hovered {
                                            this.hovered_result = i;
                                            cx.notify();
                                        }
                                    }))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .when_some(app_icon, |this, icon_img| {
                                                this.child(
                                                    img(icon_img)
                                                        .h(Pixels::from(
                                                            RESULT_EL_HEIGHT - RESULT_EL_PADDING,
                                                        ))
                                                        .w(Pixels::from(
                                                            RESULT_EL_HEIGHT - RESULT_EL_PADDING,
                                                        ))
                                                        .p(Pixels::from(RESULT_EL_PADDING)),
                                                )
                                            })
                                            .child(div().child(app_name).text_xl()),
                                    )
                            })),
                    ),
            )
    }
}
