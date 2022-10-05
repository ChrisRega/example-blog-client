use std::fmt::Debug;

pub mod value;
pub mod vec;

#[derive(Clone, PartialEq, Eq, Debug)]
/// Represents a processing state.
pub enum DataState {
    /// You should never receive this, as poll automatically updates
    Uninitialized,
    /// Data is complete
    UpToDate,
    /// Data is not (completely) ready, depending on your implementation, you may be able to get partial results
    Updating,
    /// Some error occurred
    Error(String),
}

#[derive(Debug)]
/// The message-type to send from the updater to the main thread. There's only two variants,
/// `NewData` which allows to send new data or `StateChange` which allows to signal readiness or error.
pub enum Message<T: Debug> {
    NewData(T),
    StateChange(DataState),
}

/// Maybe this should rather be called "LazyUpdating"?
/// Implementors can react to polling by queueing an update if needed.
/// Update should force an update.
pub trait Promise {
    fn poll_state(&mut self) -> &DataState;
    fn update(&mut self);
}

pub trait Sliceable<T> {
    fn as_slice(&self) -> &[T];
}

pub trait Value<T> {
    fn value(&self) -> Option<&T>;
}

/// Some type that implements lazy updating and provides a slice of the desired type
pub trait SlicePromise<T>: Promise + Sliceable<T> {}

/// Some type that implements lazy updating and provides a single value of the desired type
pub trait ValuePromise<T>: Promise + Value<T> {}

#[macro_export]
/// Error checking in async updater functions is tedious - this helps out by resolving results and sending errors on error. Result will be unwrapped if no error occurs.
macro_rules! unpack_result {
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
