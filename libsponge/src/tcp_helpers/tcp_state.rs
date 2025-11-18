pub enum TCPState {
    Closed,
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
}

pub enum SenderState {
    Closed,
    SynSent,
    SynAcked,
    FinSent,
    FinAcked,
}

pub enum ReceiverState {
    Listen,
    SynRcvd,
    FinRcvd,
}
