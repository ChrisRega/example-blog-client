use lazy_async_promise::unpack_result;
use lazy_async_promise::LazyValuePromise;
use lazy_async_promise::LazyVecPromise;
use lazy_async_promise::{DataState, Message, SlicePromise, ValuePromise};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::fmt::Debug;
use std::time::Duration;
use tokio::sync::mpsc::Sender;

#[derive(Deserialize, Debug)]
pub struct Post {
    pub user: usize,
    pub post: String,
    pub outline: Option<String>,
    pub title: String,
    pub tags: Vec<usize>,
    pub timestamp: u128,
    pub idx: i64,
}

const POSTS_URL: &str = "https://actix.vdop.org/posts";
const TAG_URL: &str = "https://actix.vdop.org/tags";

#[derive(Deserialize, Debug)]
pub struct Tag {
    pub name: String,
    pub idx: usize,
}

pub fn timestamp_to_string(timestamp_millis: u128) -> String {
    let naive = chrono::NaiveDateTime::from_timestamp((timestamp_millis / 1000) as i64, 0);
    let datetime: chrono::DateTime<chrono::Utc> = chrono::DateTime::from_utc(naive, chrono::Utc);
    format!("{}", datetime.format("%Y-%m-%d %H:%M:%S"))
}

pub fn resolve_tag(tag_idx: usize, tags: &[Tag]) -> Option<&str> {
    tags.iter()
        .find(|t| t.idx == tag_idx)
        .map(|t| t.name.as_str())
}

pub fn resolve_tags<'a>(tag_idx: &[usize], tags: &'a [Tag]) -> Vec<&'a str> {
    tag_idx
        .iter()
        .filter_map(|t| resolve_tag(*t, tags))
        .collect()
}

fn make_request_buffer_slice<T: DeserializeOwned + Debug + Send + 'static>(
    url: &'static str,
) -> Box<dyn SlicePromise<T>> {
    let updater = move |tx: Sender<Message<T>>| async move {
        let response = unpack_result!(reqwest::get(url).await, tx);
        let entries: Vec<T> = unpack_result!(response.json().await, tx);
        for entry in entries {
            tx.send(Message::NewData(entry)).await.unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        tx.send(Message::StateChange(DataState::UpToDate))
            .await
            .unwrap();
    };
    let boxed: Box<dyn SlicePromise<T>> = Box::new(LazyVecPromise::new(updater, 6));
    boxed
}

pub fn make_posts_buffer() -> Box<dyn SlicePromise<Post>> {
    make_request_buffer_slice(POSTS_URL)
}

pub fn make_tags_buffer() -> Box<dyn SlicePromise<Tag>> {
    make_request_buffer_slice(TAG_URL)
}

pub fn make_single_post_request(post_num: i64) -> Box<dyn ValuePromise<Post>> {
    let updater = move |tx: Sender<Message<Post>>| async move {
        let response = unpack_result!(
            reqwest::get(format!("{}/{}", POSTS_URL, post_num)).await,
            tx
        );
        let post: Post = unpack_result!(response.json().await, tx);
        tx.send(Message::NewData(post)).await.unwrap();
        tx.send(Message::StateChange(DataState::UpToDate))
            .await
            .unwrap();
    };
    let boxed: Box<dyn ValuePromise<Post>> = Box::new(LazyValuePromise::new(updater, 6));
    boxed
}
