use std::{
    ops::{Deref, DerefMut},
    thread,
    time::Duration,
};

use gpui::{Entity, EventEmitter};

use crate::{
    apps::{App, app_string::AppString},
    extensions::{DeferredReceiver, DeferredToken, SearchEngine},
};

pub enum SearchEvent {
    Results(Vec<App>),
}

type SearchEngineDyn = Box<dyn SearchEngine>;
pub struct GpuiSearchEngine {
    pub results: Vec<App>,
    engine: SearchEngineDyn,
}

pub type SearchEngineEntity = Entity<SearchEngineDyn>;

impl GpuiSearchEngine {
    pub fn new(search_engine: impl SearchEngine + 'static) -> Self {
        Self {
            results: Vec::new(),
            engine: Box::new(search_engine),
        }
    }

    pub fn blocking_search(&mut self, query: AppString, cx: &mut gpui::Context<'_, Self>) {
        thread::sleep(Duration::from_secs(1));
        cx.emit(SearchEvent::Results(self.engine.blocking_search(query)));
    }

    pub fn deferred_search(
        &mut self,
        query: AppString,
        cx: &mut gpui::Context<'_, Self>,
        window: &gpui::Window,
    ) {
        cx.spawn_in(window, async move |w, cx| {
            #[allow(clippy::missing_panics_doc, reason = "entity has not been released")]
            let (token, mut rx): (DeferredToken, DeferredReceiver) = w
                .read_with(cx, |this, _cx| this.engine.deferred_search(query.clone()))
                .expect("entity has not been released");

            loop {
                if rx.changed().await.is_err() {
                    // Closed rx, abort.
                    return;
                }

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
            }
        })
        .detach();
    }
}

impl EventEmitter<SearchEvent> for GpuiSearchEngine {}

impl Deref for GpuiSearchEngine {
    type Target = dyn SearchEngine;

    fn deref(&self) -> &Self::Target {
        self.engine.as_ref()
    }
}

impl DerefMut for GpuiSearchEngine {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.engine.as_mut()
    }
}
