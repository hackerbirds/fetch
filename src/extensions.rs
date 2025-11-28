use gpui::AsyncApp;
use tokio::sync::watch::{self, Receiver, Sender};

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
        _cx: &mut AsyncApp,
        query: &AppString,
    ) -> (DeferredToken, DeferredReceiver) {
        let res = self.blocking_search(query);
        let (_tx, rx) = watch::channel((0, res));
        (0, rx)
    }
    fn selected(&mut self, query_history: Vec<AppName>, opened_app: &App);
    /// If needed, update the search engine.
    fn update(&mut self);
}
