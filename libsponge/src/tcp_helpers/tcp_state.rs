use thiserror::Error;

#[derive(Error, Debug)]
pub enum TCPConnectionError {
    #[error("Receiver error. (connection reset by peer)")]
    ReceiverError,
    #[error("Sender error. (connection reset by peer)")]
    SenderError,
    #[error("Unknown connection state")]
    UnknownConnectionState,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum TCPState {
    #[default]
    Listen,
    SynSent,
    SynRcvd,
    Established,
    FinWait1,
    FinWait2,
    Closing,
    CloseWait,
    LastAck,
    TimeWait,
    Closed,
    Reset,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum SenderState {
    #[default]
    Closed,
    SynSent,
    SynAcked,
    FinSent,
    FinAcked,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum ReceiverState {
    #[default]
    Listen,
    SynRcvd,
    FinRcvd,
}

// trait State {}
// impl State for TCPState {}
// impl State for SenderState {}
// impl State for ReceiverState {}

// impl<S> PartialEq<S> for Result<S>
// where
//     S: State + PartialEq,
// {
//     fn eq(&self, other: &Result<S>) -> bool {
//         match (self, other) {
//             (Ok(i), Ok(j)) => i == j,
//             _ => false,
//         }
//     }
// }
