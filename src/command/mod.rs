use std::borrow::Cow;

use rootcause::{Report, option_ext::OptionExt};
use trie_rs::map::{Trie, TrieBuilder};

use crate::apps::url::Url;

pub struct CommandTrie {
    inner: Trie<u8, Url>,
}

impl Default for CommandTrie {
    fn default() -> Self {
        let mut builder = TrieBuilder::new();

        builder.push("hn", Url::Https(Cow::Borrowed("news.ycombinator.com")));
        builder.push("gh", Url::Https(Cow::Borrowed("github.com")));

        Self {
            inner: builder.build(),
        }
    }
}

impl CommandTrie {
    pub fn execute(&self, command: &str) -> Result<(), Report> {
        self.inner
            .exact_match(command)
            .and_then(|res| res.open().ok())
            .ok_or_report()?;

        Ok(())
    }
}
