use std::{
    ops::{Deref, DerefMut},
    thread,
    time::Duration,
};

use gpui::{Entity, EventEmitter};

use crate::{
    apps::{App, app_string::AppString},
    search::SearchEngine,
};

pub enum SearchEvent {
    Results(Vec<App>),
}

type SearchEngineDyn = Box<dyn SearchEngine>;
pub struct GpuiSearchEngine {
    pub results: Vec<App>,
    pub current_query: AppString,
    engine: SearchEngineDyn,
}

pub type SearchEngineEntity = Entity<SearchEngineDyn>;

impl GpuiSearchEngine {
    pub fn new(search_engine: impl SearchEngine + 'static) -> Self {
        Self {
            results: Vec::new(),
            current_query: AppString::default(),
            engine: Box::new(search_engine),
        }
    }

    pub fn blocking_search(&mut self, query: &AppString, cx: &mut gpui::Context<'_, Self>) {
        thread::sleep(Duration::from_secs(1));
        cx.emit(SearchEvent::Results(self.engine.blocking_search(query)));
    }

    pub fn deferred_search(
        &mut self,
        query: &AppString,
        cx: &mut gpui::Context<'_, Self>,
        window: &gpui::Window,
    ) {
        let (token, mut rx) = self.engine.deferred_search(&mut cx.to_async(), query);
        self.current_query = query.clone();
        cx.spawn_in(window, async move |w, cx| {
            loop {
                if rx.changed().await.is_err() {
                    // Closed rx, abort.
                    return;
                }

                let search_token = rx.borrow().0;
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
