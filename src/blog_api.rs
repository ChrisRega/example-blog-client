use lazy_async_promise::{
    api_macros::*, DataState, ImmediateValuePromise, LazyValuePromise, LazyVecPromise, Message,
    Progress,
};
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Arc;
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

#[derive(Deserialize, Debug)]
pub struct PostUpload {
    pub post: String,
    pub title: String,
    pub outline: Option<String>,
    pub tags: Vec<usize>,
}

impl From<Post> for PostUpload {
    fn from(value: Post) -> Self {
        PostUpload {
            post: value.post,
            title: value.title,
            outline: value.outline,
            tags: value.tags,
        }
    }
}

const POSTS_URL: &str = "https://actix.vdop.org/posts";
const TAG_URL: &str = "https://actix.vdop.org/tags";
const LOGIN_URL: &str = "https://actix.vdop.org/login";

#[derive(Deserialize, Debug)]
pub struct Tag {
    pub name: String,
    pub idx: usize,
}

#[derive(Serialize, Debug, Default, Clone)]
pub struct Login {
    pub user: String,
    pub password: String,
}

impl Login {
    pub fn try_login(&self, client: Arc<reqwest::Client>) -> ImmediateValuePromise<StatusCode> {
        let credentials = self.clone();
        let updater = async move {
            let result = client.post(LOGIN_URL).json(&credentials).send().await?;
            Ok(result.status())
        };

        ImmediateValuePromise::new(updater)
    }
}

pub fn timestamp_to_string(timestamp_millis: u128) -> String {
    let naive = chrono::NaiveDateTime::from_timestamp_millis(timestamp_millis as i64)
        .expect("Could not convert timestamp to datetime");
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
) -> LazyVecPromise<T> {
    let updater = move |tx: Sender<Message<T>>| async move {
        let response = unpack_result!(reqwest::get(url).await, tx);
        let entries: Vec<T> = unpack_result!(response.json().await, tx);
        let total_entries = entries.len();
        for (num, entry) in entries.into_iter().enumerate() {
            send_data!(entry, tx);
            set_progress!(
                Progress::from_fraction(num as u32, total_entries as u32),
                tx
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        set_finished!(tx);
    };
    LazyVecPromise::new(updater, 6)
}

pub fn make_posts_buffer() -> LazyVecPromise<Post> {
    make_request_buffer_slice(POSTS_URL)
}

pub fn make_tags_buffer() -> LazyVecPromise<Tag> {
    make_request_buffer_slice(TAG_URL)
}

// not used currently in favor of immediate updating version below which is easier but:
// this allows an easy "update" button on the posts page...
pub fn _make_lazy_single_post_request(post_num: i64) -> LazyValuePromise<Post> {
    let updater = move |tx: Sender<Message<Post>>| async move {
        let response = unpack_result!(
            reqwest::get(format!("{}/{}", POSTS_URL, post_num)).await,
            tx
        );
        let post: Post = unpack_result!(response.json().await, tx);
        send_data!(post, tx);
        set_finished!(tx);
    };
    LazyValuePromise::new(updater, 6)
}

pub fn make_immediate_post_request(post_num: i64) -> ImmediateValuePromise<Post> {
    ImmediateValuePromise::new(async move {
        let response = reqwest::get(format!("{}/{}", POSTS_URL, post_num)).await?;
        let post: Post = response.json().await?;
        Ok(post)
    })
}
