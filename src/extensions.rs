use gpui::AsyncApp;
use tokio::sync::watch::{Receiver, Sender};

pub mod deterministic_search;

use crate::apps::{App, AppName, app_string::AppString};

pub type DeferredToken = usize;
pub type DeferredMessage = (DeferredToken, Vec<App>);
pub type DeferredSender = Sender<DeferredMessage>;
pub type DeferredReceiver = Receiver<DeferredMessage>;

pub trait SearchEngine {
    fn blocking_search(&self, query: &AppString) -> Vec<App>;
    fn deferred_search(
        &self,
        cx: &mut AsyncApp,
        query: &AppString,
    ) -> (DeferredToken, DeferredReceiver);
    fn selected(&mut self, query_history: Vec<AppName>, opened_app: &App);
    /// If needed, update the search engine.
    fn update(&mut self);
}
