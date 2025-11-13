use gpui::{
    AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement, Render, Styled,
    Subscription, Window, div,
};
use gpui_component::StyledExt;
use gpui_component::input::{Input, InputEvent, InputState};

use crate::apps::app_string::AppString;
use crate::search::SearchEngine;
use crate::ui::app_list::SearchResultsList;
use crate::{EnterPressed, EscPressed};

pub struct SearchBar {
    search_engine: Entity<SearchEngine>,
    input_state: Entity<InputState>,
    all_queries: Vec<AppString>,
    search_results: Vec<crate::apps::App>,
    #[expect(unused)]
    subscriptions: Vec<Subscription>,
}

impl SearchBar {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        search_engine: Entity<SearchEngine>,
    ) -> Self {
        let input_state = cx.new(|cx| {
            let is = InputState::new(window, cx).placeholder("Search app");
            is.focus(window, cx);
            is
        });

        let search_results = vec![];
        let all_queries = vec![];

        let subscriptions = vec![cx.subscribe_in(&input_state, window, {
            let input_state = input_state.clone();
            move |this, _, ev: &InputEvent, _window, cx| match ev {
                InputEvent::Change => {
                    let value = input_state.read(cx).value();
                    let value: AppString = value.into();

                    this.search_results = this.search_engine.read(cx).search(&value);

                    this.all_queries.push(value);
                    cx.notify();
                }
                InputEvent::PressEnter {
                    secondary: _secondary,
                } => {
                    if let Some(app) = this.search_results.first() {
                        this.search_engine
                            .read(cx)
                            .selected(this.all_queries.clone(), app);
                        cx.notify();
                    }
                }
                _ => {}
            }
        })];

        Self {
            search_engine,
            input_state,
            all_queries,
            search_results,
            subscriptions,
        }
    }
}

impl Render for SearchBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .p_5()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .on_action(|&EscPressed, window, _cx| {
                window.remove_window();
            })
            .on_action(cx.listener(|this, &EnterPressed, window, cx| {
                if let Some(app) = this.search_results.first() {
                    cx.open_with_system(app.path.as_path());
                    this.search_engine
                        .read(cx)
                        .selected(this.all_queries.clone(), app);
                    window.remove_window();
                }
            }))
            .child(Input::new(&self.input_state).border_2())
            .child(
                div()
                    .v_flex()
                    .gap_2()
                    .size_full()
                    .overflow_y_hidden()
                    .child(cx.new(|_cx| SearchResultsList::new(self.search_results.clone()))),
            )
    }
}
