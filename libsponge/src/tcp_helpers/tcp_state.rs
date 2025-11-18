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
