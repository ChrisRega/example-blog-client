use crate::buffered::{DataState, Message, Promise, Value, ValuePromise};
use std::fmt::Debug;
use std::future::Future;
use tokio::sync::mpsc::{channel, Receiver, Sender};

/// # A single lazy-async updated value
/// Create one with the `new` method and supply an updater.
/// It's Updated only on first try to poll it making it scale nicely on more complex UIs.

pub struct LazyValuePromise<
    T: Debug,
    U: Fn(Sender<Message<T>>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
> {
    cache: Option<T>,
    updater: U,
    state: DataState,
    rx: Receiver<Message<T>>,
    tx: Sender<Message<T>>,
}
impl<T: Debug, U: Fn(Sender<Message<T>>) -> Fut, Fut: Future<Output = ()> + Send + 'static>
    LazyValuePromise<T, U, Fut>
{
    pub fn new(updater: U, buffer_size: usize) -> Self {
        let (tx, rx) = channel::<Message<T>>(buffer_size);

        Self {
            cache: None,
            state: DataState::Uninitialized,
            rx,
            tx,
            updater,
        }
    }
}

impl<T: Debug, U: Fn(Sender<Message<T>>) -> Fut, Fut: Future<Output = ()> + Send + 'static> Value<T>
    for LazyValuePromise<T, U, Fut>
{
    fn value(&self) -> Option<&T> {
        self.cache.as_ref()
    }
}

impl<T: Debug, U: Fn(Sender<Message<T>>) -> Fut, Fut: Future<Output = ()> + Send + 'static>
    ValuePromise<T> for LazyValuePromise<T, U, Fut>
{
}

impl<T: Debug, U: Fn(Sender<Message<T>>) -> Fut, Fut: Future<Output = ()> + Send + 'static> Promise
    for LazyValuePromise<T, U, Fut>
{
    fn poll_state(&mut self) -> &DataState {
        if self.state == DataState::Uninitialized {
            self.update();
        }

        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Message::NewData(data) => {
                    self.cache = Some(data);
                }
                Message::StateChange(new_state) => {
                    self.state = new_state;
                }
            }
        }

        &self.state
    }

    fn update(&mut self) {
        if self.state == DataState::Updating {
            return;
        }
        self.cache = None;

        self.state = DataState::Updating;
        let future = (self.updater)(self.tx.clone());
        tokio::spawn(future);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::Duration;
    use tokio::runtime::Runtime;

    #[test]
    fn test_func() {
        let string_maker = |tx: Sender<Message<String>>| async move {
            for i in 0..2 {
                tx.send(Message::NewData(i.to_string())).await.unwrap();
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            tx.send(Message::StateChange(DataState::UpToDate))
                .await
                .unwrap();
        };

        Runtime::new().unwrap().block_on(async {
            let mut delayed_value = LazyValuePromise::new(string_maker, 6);
            assert_eq!(*delayed_value.poll_state(), DataState::Updating);
            assert!(delayed_value.value().is_none());
            std::thread::sleep(Duration::from_millis(150));
            assert_eq!(*delayed_value.poll_state(), DataState::UpToDate);
            assert_eq!(delayed_value.value().unwrap(), "1");
        });
    }
}
