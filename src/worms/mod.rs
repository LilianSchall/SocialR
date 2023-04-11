use std::fmt::Debug;
use crate::error::Error;

use async_trait::async_trait;

#[async_trait]
pub trait Worm : Send + Sync {
    type Item: Debug + Clone;

    fn name(&self) -> String;
    fn start_urls(&self) -> Vec<String>;
    async fn scrape(&self, url: &str) -> Result<(Vec<Self::Item>, Vec<String>), Error>;
}
