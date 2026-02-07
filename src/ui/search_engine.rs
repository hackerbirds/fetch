use std::sync::Arc;

use gpui::{AppContext, Entity};

use crate::{
    apps::{ExecutableApp, app_string::AppString},
    extensions::{DeferredReceiver, DeferredToken, SearchEngine, SearchResult},
};

pub struct GpuiSearchEngine<SE: SearchEngine> {
    pub(super) results: Vec<SearchResult>,
    engine: Arc<SE>,
}

pub type SearchEngineEntity<SE> = Entity<Arc<SE>>;

impl<SE: SearchEngine> GpuiSearchEngine<SE> {
    pub fn new(search_engine: SE) -> GpuiSearchEngine<SE> {
        GpuiSearchEngine::<SE> {
            results: Vec::new(),
            engine: Arc::new(search_engine),
        }
    }

    pub fn preload(&self, cx: &mut gpui::Context<'_, Self>) {
        let engine = self.engine.clone();

        cx.background_spawn(async move {
            engine.preload();
        })
        .detach();
    }

    pub fn blocking_search(&mut self, query: AppString) {
        self.engine.blocking_search(query);
    }

    pub fn deferred_search(
        &mut self,
        cx: &mut gpui::Context<'_, Self>,
        window: &gpui::Window,
        query: AppString,
    ) {
        cx.spawn_in(window, async move |w, cx| {
            let (token, mut rx): (DeferredToken, DeferredReceiver) = w
                .read_with(cx, |this, _cx| this.engine.deferred_search(query))
                .expect("entity has not been released");

            loop {
                let search_token: DeferredToken = rx.borrow().0;
                if search_token > token {
                    // New search executed on a different task,
                    // abort this one
                    return;
                } else if let Some(view) = w.upgrade() {
                    // Update search results and notify UI
                    let _ = view.update(cx, |this, cx| {
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

    pub fn after_search(
        &self,
        cx: &mut gpui::Context<'_, Self>,
        opened_app: Option<ExecutableApp>,
    ) {
        let engine = self.engine.clone();

        cx.background_spawn(async move {
            engine.after_search(opened_app.map(SearchResult::Executable));
        })
        .detach();
    }
}
