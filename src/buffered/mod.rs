use std::fmt::Debug;

pub mod value;
pub mod vec;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DataState {
    Uninitialized,
    UpToDate,
    Updating,
    Error(String),
}

#[derive(Debug)]
pub enum Message<T: Debug> {
    NewData(T),
    StateChange(DataState),
}

pub trait Buffer {
    fn poll_state(&mut self) -> &DataState;
    fn update(&mut self);
}

pub trait Sliceable<T> {
    fn as_slice(&self) -> &[T];
}

pub trait Value<T> {
    fn value(&self) -> Option<&T>;
}

pub trait BufferedSlice<T>: Buffer + Sliceable<T> {}
pub trait BufferedValue<T>: Buffer + Value<T> {}

#[macro_export]
macro_rules! check {
    ( $result: expr, $sender: expr ) => {
        match $result {
            Ok(val) => val,
            Err(e) => {
                $sender
                    .send(Message::StateChange(DataState::Error(format!("{}", e))))
                    .await
                    .unwrap();
                return;
            }
        }
    };
}
