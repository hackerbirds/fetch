use std::cmp::min;

use gpui::prelude::FluentBuilder;
use gpui::{
    AppContext, Context, Corners, ElementId, Entity, Fill, InteractiveElement, IntoElement,
    MouseButton, Negate, ParentElement, Pixels, Point, Render, ScrollHandle,
    StatefulInteractiveElement, Styled, Subscription, Window, div, img,
};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{ActiveTheme, StyledExt};

use crate::apps::app_string::AppString;
use crate::apps::url::Url;
use crate::extensions::{SearchEngine, SearchResult};
use crate::fs::config::config_file_path;
use crate::ui::gpui_app::{GpuiApp, GpuiAppLoader};
use crate::ui::search_engine::GpuiSearchEngine;
use crate::{EnterPressed, EscPressed, OpenSettings, TabBackSelectApp, TabSelectApp};

pub struct SearchBar<SE: SearchEngine> {
    search_engine: Entity<GpuiSearchEngine<SE>>,
    input_state: Entity<InputState>,
    #[expect(unused)]
    subscriptions: Vec<Subscription>,
    /// The index of the first result the user has scrolled to
    scrolled_result_idx: usize,
    /// The offset of the hovered/selected result. This means that
    /// if the user has scrolled 3 indices down, but selected the second
    /// app result (offset 1),
    /// then `scrolled_result_idx` = 3 and `hovered_offset_idx` = 1
    ///
    /// `scrolled_result_idx` + `hovered_offset_idx` = selected app index
    hovered_offset_idx: usize,
    scroll_handle: ScrollHandle,
    gpui_app_renderer: GpuiAppLoader,
}

/// The number of elements to render in gpui. This corresponds
/// to how many search results at once are physically able to
/// appear in the GUI (whose window height is a fixed size)
const MAX_RENDERED_ELS: usize = 4;
/// The height of the element containing a search result (icon + app name)
const RESULT_EL_HEIGHT: usize = 44;
/// The padding (all sides) of the element containing a search result (icon + app name)
const RESULT_EL_PADDING: usize = 8;

impl<SE: SearchEngine> SearchBar<SE> {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        search_engine: Entity<GpuiSearchEngine<SE>>,
    ) -> Self {
        let input_state = cx.new(|cx| {
            let is = InputState::new(window, cx).placeholder("Search an app");
            is.focus(window, cx);
            is
        });

        search_engine.update(cx, |this, cx| {
            this.preload(cx);
        });

        let subscriptions = vec![cx.subscribe_in(&input_state, window, {
            let input_state = input_state.clone();
            move |this, _, ev: &InputEvent, window, cx| {
                if let InputEvent::Change = ev {
                    let value = input_state.read(cx).value();
                    let value: AppString = value.into();

                    this.search_engine.update(cx, |this, cx| {
                        this.deferred_search(cx, window, value);
                    });

                    this.scrolled_result_idx = 0;
                    this.hovered_offset_idx = 0;

                    cx.notify();
                }
            }
        })];

        Self {
            search_engine,
            input_state,
            subscriptions,
            scrolled_result_idx: 0,
            hovered_offset_idx: 0,
            scroll_handle: ScrollHandle::new(),
            gpui_app_renderer: GpuiAppLoader::default(),
        }
    }
}

impl<SE: SearchEngine> Render for SearchBar<SE> {
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
                    let selected_app_idx = this.scrolled_result_idx + this.hovered_offset_idx;
                    // User scrolled down at the last index, so we need to loop back up
                    let wrap_around_needed = selected_app_idx >= results_len - 1;
                    if wrap_around_needed {
                        this.scrolled_result_idx = 0;
                        this.hovered_offset_idx = 0;
                    } else if this.hovered_offset_idx < (MAX_RENDERED_ELS - 1) {
                        this.hovered_offset_idx += 1;
                    } else {
                        this.scrolled_result_idx += 1;
                    }
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, &TabBackSelectApp, _, cx| {
                let results_len = this.search_engine.read(cx).results.len();
                if results_len > 0 {
                    let selected_app_idx = this.scrolled_result_idx + this.hovered_offset_idx;
                    // User scrolled down at the first index, so we need to loop back down
                    let wrap_around_needed = selected_app_idx == 0;
                    if wrap_around_needed {
                        this.hovered_offset_idx = min(results_len, MAX_RENDERED_ELS) - 1;
                        this.scrolled_result_idx = (results_len - 1).saturating_sub(this.hovered_offset_idx);
                    } else if this.hovered_offset_idx > 0 {
                        if this.scrolled_result_idx > 0 && this.hovered_offset_idx == 1  {
                            // Lock hovered index to 1 when we're scrolling back
                            // so that the user can visually tell that there are more apps
                            // at the top of the list (and also see which app it is, so if
                            // the user knows that this is the app they want, they'll know
                            // before the last keypress)
                            this.scrolled_result_idx =
                                (this.scrolled_result_idx + results_len - 1).rem_euclid(results_len);
                        } else {
                            this.hovered_offset_idx -= 1;
                        }
                    } else {
                        this.scrolled_result_idx =
                            (this.scrolled_result_idx + results_len - 1).rem_euclid(results_len);
                    }
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, &EscPressed, window, cx| {
                window.remove_window();
                this.search_engine.update(cx, |search_engine, cx| {
                    search_engine.after_search(cx, None);
                });
                cx.notify();
            }))
            .on_action(cx.listener(|_, &OpenSettings, window, cx| {
                window.remove_window();
                if let Ok(cfg_path) = config_file_path() {
                    Url::File(cfg_path).open().ok();
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, &EnterPressed, window, cx| {
                let selected_app_idx = this.scrolled_result_idx + this.hovered_offset_idx;
                let app_opt = this
                    .search_engine
                    .read(cx)
                    .results.get(selected_app_idx)
                    // Cloning removes `cx` lifetime
                    .cloned();

                if let Some(SearchResult::Executable(app)) = app_opt {
                    let _ = Url::File(app.path.clone()).open();
                    this.search_engine.update(cx, |search_engine, cx| {
                        search_engine.after_search(cx, Some(app));
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
                                .results
                                .iter()
                                .skip(self.scrolled_result_idx)
                                .take(MAX_RENDERED_ELS + 1)
                                .map(|app| self.gpui_app_renderer.load(app, cx)).enumerate().map(|(i, GpuiApp { name, path, is_open, icon })| {
                                    #[allow(
                                        clippy::cast_precision_loss,
                                        reason = "we don't need high precision, div el height is tiny"
                                    )]
                                    div()
                                        .id(ElementId::named_usize(name.clone(), i))
                                        .flex()
                                        .items_center()
                                        .p(Pixels::from(RESULT_EL_PADDING))
                                        .min_h(Pixels::from(RESULT_EL_HEIGHT))
                                        .h(Pixels::from(RESULT_EL_HEIGHT))
                                        .pl(Pixels::from(40.0 / ((self.hovered_offset_idx.abs_diff(i) + 1) as f64).powf(1.67)))
                                        .when(i == self.hovered_offset_idx, |mut this| {
                                            this.style().background =
                                                Some(Fill::Color(cx.theme().secondary_hover.into()));

                                            self.scroll_handle.set_offset(Point::new(
                                                0f64.into(),
                                                // RESULT_EL_HEIGHT: height of el
                                                // RESULT_EL_PADDING: padding top
                                                // RESULT_EL_PADDING: padding bottom
                                                Pixels::from((i * (RESULT_EL_HEIGHT + 2 * RESULT_EL_PADDING))
                                                    as f64).negate(),
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
                                        .on_mouse_down(MouseButton::Left, move |_, window, _cx| {
                                            Url::File(path.clone()).open().ok();
                                            window.remove_window();
                                        })
                                        .on_hover(cx.listener(move |this, hovered, _window, cx| {
                                            if *hovered {
                                                this.hovered_offset_idx = i;
                                                cx.notify();
                                            }
                                        }))
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_1()
                                                .when_some(icon, |this, icon_img| {
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
                                                .child(div().child(name).text_xl().when(!is_open, |this| {
                                                    this.opacity(0.5f32)
                                                })),
                                        )
                                })),
                    ),
            )
    }
}
