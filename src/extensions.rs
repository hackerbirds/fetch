use serde::{Deserialize, Serialize};
use tokio::sync::watch::{self, Receiver, Sender};

pub mod deterministic_search;

use crate::apps::{ExecutableApp, app_string::AppString};

pub type DeferredToken = usize;
pub type DeferredMessage = (DeferredToken, Vec<SearchResult>);
pub type DeferredSender = Sender<DeferredMessage>;
pub type DeferredReceiver = Receiver<DeferredMessage>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum SearchResult {
    Executable(ExecutableApp),
}

pub trait SearchEngine: Send + Sync + 'static {
    fn blocking_search(&self, query: AppString) -> Vec<SearchResult>;
    fn deferred_search(&self, query: AppString) -> (DeferredToken, DeferredReceiver) {
        let res = self.blocking_search(query);
        let (_tx, rx) = watch::channel((0, res));
        (0, rx)
    }

    /// This function is called after a search: either the user cancelled the search
    /// by pressing Esc, or they succeded a search by selecting an app.
    fn after_search(&self, selected_app: Option<SearchResult>);
}
