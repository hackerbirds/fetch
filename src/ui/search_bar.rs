use std::cmp;
use std::path::Path;

use gpui::{
    AppContext, Context, Corners, Entity, InteractiveElement, IntoElement, ParentElement, Render,
    Styled, Subscription, Window, div,
};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{ActiveTheme, StyledExt};

use crate::apps::app_string::AppString;
use crate::fs::config::config_file_path;
use crate::ui::gpui_app::GpuiApp;
use crate::ui::search_engine::GpuiSearchEngine;
use crate::ui::search_results::SearchResultsList;
use crate::{EnterPressed, EscPressed, OpenSettings, TabSelectApp};

pub struct SearchBar {
    search_engine: Entity<GpuiSearchEngine>,
    input_state: Entity<InputState>,
    all_queries: Vec<AppString>,
    #[expect(unused)]
    subscriptions: Vec<Subscription>,
    selected_result: usize,
}

impl SearchBar {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        search_engine: Entity<GpuiSearchEngine>,
    ) -> Self {
        let input_state = cx.new(|cx| {
            let is = InputState::new(window, cx).placeholder("Search app");
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
        }
    }
}

impl Render for SearchBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .p_2()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .on_action(cx.listener(|this, &TabSelectApp, _, cx| {
                let results_len = this.search_engine.read(cx).results.len();
                this.selected_result = (this.selected_result + 1).rem_euclid(results_len);
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
                cx.open_with_system(Path::new(config_file_path().to_str().unwrap()));
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
                    .bg(cx.theme().secondary)
                    .corner_radii(Corners::all(10.0f64.into()))
                    .m_auto()
                    .h_20()
                    .text_2xl(),
            )
            .child(
                div()
                    .v_flex()
                    .gap_2()
                    .size_full()
                    .overflow_y_hidden()
                    .child(cx.new(|cx| {
                        let search_results = self
                            .search_engine
                            .read(cx)
                            .results
                            .clone()
                            .into_iter()
                            .skip(self.selected_result)
                            .take(3)
                            .map(|app| GpuiApp::load(app, cx))
                            .collect();

                        SearchResultsList::new(search_results)
                    })),
            )
    }
}
