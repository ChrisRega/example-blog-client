# Testing egui and tokio
Just my demo project on how one can use tokio to make a fluid egui user interface when processing data from the internet.
The primitives are now located in the "lazy_async_promise" crate, which can be found on [crates.io](https://crates.io/crates/lazy_async_promise) 
and [github](https://github.com/ChrisRega/lazy_async_promise).
Maybe you will find them useful, too :)

For this example we artificially emit the posts slowly to show a progress-bar :)
We get both the tags and the posts slowed down like with the code from `blog_api.rs`
```rust
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
```

For getting a single post we use the `ImmediateValuePromise`:
```rust
pub fn make_immediate_post_request(
    post_num: i64,
    update_callback: impl Fn() + Send + 'static,
) -> ImmediateValuePromise<Post> {
    ImmediateValuePromise::new(async move {
        // we can finally use the ? operator! YAY! No macros!
        let response = reqwest::get(format!("{}/{}", POSTS_URL, post_num)).await?;
        let post: Post = response.json().await?;
        // notify egui we need a redraw in case the programmer (me) forgot the spinner
        update_callback();
        Ok(post)
    })
}
```
The specialty for egui is, that we don't actually get drawn / updated if nothing is going own.
Imagine downloading the post without a spinner - nothing changes, no update call therefore no polling.
But without polling we will never get notified about the finish. This will not happen in this use case since we would usually 
use a spinner for indicating that we are fetching main page content. But imagine a select!-macro
like future that waits for a message where a spinner would be a bad pattern (cpu usage, useless redraws,...).
We would like to get the update callback here :)

My solution to this problem was to store the context of egui in the app state.
```rust
struct BlogClient {
    update_callback_ctx: Option<egui::Context>,
    // and a lot more....
}
impl BlogClient {
    fn update_callback(&self) -> impl Fn() {
        let ctx = self.update_callback_ctx.clone().unwrap();
        move || {  ctx.request_repaint(); }
    }
}
```
Since as of egui 0.21 Context is:

```rust
pub struct Context(Arc<RwLock<ContextImpl>>);
```
we can safely do this and request repaint is safe to be called from any thread as per docs.rs.

The rest of the code is just me playing around with egui :)