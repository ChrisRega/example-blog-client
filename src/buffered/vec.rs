use crate::buffered::{Buffer, BufferedSlice, DataState, Message, Sliceable};
use std::fmt::Debug;
use std::future::Future;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub struct BufVec<
    T: Debug,
    U: Fn(Sender<Message<T>>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
> {
    data: Vec<T>,
    state: DataState,
    rx: Receiver<Message<T>>,
    tx: Sender<Message<T>>,
    updater: U,
}

impl<T: Debug, U: Fn(Sender<Message<T>>) -> Fut, Fut: Future<Output = ()> + Send + 'static>
    BufVec<T, U, Fut>
{
    pub fn new(updater: U, buffer_size: usize) -> Self {
        let (tx, rx) = channel::<Message<T>>(buffer_size);

        Self {
            data: vec![],
            state: DataState::Uninitialized,
            rx,
            tx,
            updater,
        }
    }

    pub fn to_vec(&self) -> &Vec<T> {
        &self.data
    }
}

impl<T: Debug, U: Fn(Sender<Message<T>>) -> Fut, Fut: Future<Output = ()> + Send + 'static>
    Sliceable<T> for BufVec<T, U, Fut>
{
    fn as_slice(&self) -> &[T] {
        self.data.as_slice()
    }
}

impl<T: Debug, U: Fn(Sender<Message<T>>) -> Fut, Fut: Future<Output = ()> + Send + 'static>
    BufferedSlice<T> for BufVec<T, U, Fut>
{
}

impl<T: Debug, U: Fn(Sender<Message<T>>) -> Fut, Fut: Future<Output = ()> + Send + 'static> Buffer
    for BufVec<T, U, Fut>
{
    fn poll_state(&mut self) -> &DataState {
        if self.state == DataState::Uninitialized {
            self.update();
        }

        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Message::NewData(data) => {
                    self.data.push(data);
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

        self.state = DataState::Updating;
        self.data.clear();
        let future = (self.updater)(self.tx.clone());
        tokio::spawn(future);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::buffered::vec::BufVec;
    use std::time::Duration;
    use tokio::runtime::Runtime;

    #[test]
    fn test_func() {
        let string_maker = |tx: Sender<Message<String>>| async move {
            for i in 0..5 {
                tx.send(Message::NewData(i.to_string())).await.unwrap();
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            tx.send(Message::StateChange(DataState::UpToDate))
                .await
                .unwrap();
        };

        Runtime::new().unwrap().block_on(async {
            let mut delayed_vec = BufVec::new(string_maker, 6);
            assert_eq!(delayed_vec.poll_state(), DataState::Updating);
            assert!(delayed_vec.to_vec().is_empty());
            std::thread::sleep(Duration::from_millis(150));
            assert_eq!(delayed_vec.poll_state(), DataState::UpToDate);
            assert_eq!(delayed_vec.to_vec().len(), 5);
        });
    }
}
