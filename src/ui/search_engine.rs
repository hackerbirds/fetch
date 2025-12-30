use std::sync::Arc;

use gpui::{AppContext, Entity, EventEmitter};

use crate::{
    apps::{App, app_string::AppString},
    extensions::{DeferredReceiver, DeferredToken, SearchEngine},
};

pub enum SearchEvent {
    Results(Vec<App>),
}

type SearchEngineDyn = Arc<dyn SearchEngine>;
pub struct GpuiSearchEngine {
    pub results: Vec<App>,
    engine: SearchEngineDyn,
}

pub type SearchEngineEntity = Entity<SearchEngineDyn>;

impl GpuiSearchEngine {
    pub fn new(search_engine: impl SearchEngine + 'static) -> Self {
        Self {
            results: Vec::new(),
            engine: Arc::new(search_engine),
        }
    }

    pub fn blocking_search(&mut self, cx: &mut gpui::Context<'_, Self>, query: AppString) {
        cx.emit(SearchEvent::Results(self.engine.blocking_search(query)));
    }

    pub fn deferred_search(
        &mut self,
        cx: &mut gpui::Context<'_, Self>,
        window: &gpui::Window,
        query: AppString,
    ) {
        cx.spawn_in(window, async move |w, cx| {
            #[allow(clippy::missing_panics_doc, reason = "entity has not been released")]
            let (token, mut rx): (DeferredToken, DeferredReceiver) = w
                .read_with(cx, |this, _cx| this.engine.deferred_search(query.clone()))
                .expect("entity has not been released");

            loop {
                let search_token: DeferredToken = rx.borrow().0;
                if search_token > token {
                    // New search executed on a different task,
                    // abort this one
                    return;
                } else if let Some(www) = w.upgrade() {
                    // Update search results and notify UI
                    let _ = www.update(cx, |this, cx| {
                        let search_results = rx.borrow().1.clone();
                        this.results = search_results;
                        cx.notify();
                    });
                }

                if rx.changed().await.is_err() {
                    // Closed rx, abort.
                    return;
                }
            }
        })
        .detach();
    }

    pub fn selected(
        &self,
        cx: &mut gpui::Context<'_, Self>,
        query_history: Vec<crate::apps::AppName>,
        opened_app: App,
    ) {
        let engine = self.engine.clone();

        cx.background_spawn(async move {
            engine.selected(query_history, &opened_app);
        })
        .detach();
    }

    pub fn update(&self, cx: &mut gpui::Context<'_, Self>) {
        let engine = self.engine.clone();

        cx.background_spawn(async move {
            engine.update();
        })
        .detach();
    }
}

impl EventEmitter<SearchEvent> for GpuiSearchEngine {}
