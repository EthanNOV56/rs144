use crate::{
    BufferError, Milliseconds, TCPConfig, TCPReceiver, TCPSegment, TCPSender, tcp_state::*,
};

use std::{collections::VecDeque, marker::PhantomData};

pub struct TCPConnection<T: TCPState, S: SenderState, R: ReceiverState> {
    _tcp_state: PhantomData<T>,
    _sender_state: PhantomData<S>,
    _receiver_state: PhantomData<R>,

    cfg: TCPConfig,
    sender: TCPSender,
    receiver: TCPReceiver,
    segments_out: VecDeque<TCPSegment>,
    linger: bool,
    ms_since_last_seg_recv: Milliseconds,
    active: bool,
    need_send_rst: bool,
    ack_for_fin_sent: bool,
}

impl<T: TCPState, S: SenderState, R: ReceiverState> TCPConnection<T, S, R> {
    fn push_segments_out(&mut self, send_syn: Option<bool>) -> Result<bool, BufferError> {
        let send_syn = send_syn.unwrap_or(false);
        self.sender
            .fill_window(Some(send_syn || self.in_syn_recv()));
        while let Some(seg) = self.sender.segments_out.pop_front() {
            if let Some(val) = self.receiver.ackno() {
                seg.header_mut().ack = true;
                seg.header_mut().ack_no = val;
                seg.header_mut().win = self.receiver.win_size() as u16;
            }
            if self.need_send_rst {
                self.need_send_rst = false;
                seg.header_mut().rst = true;
            }
            self.segments_out.push_back(seg);
        }
        self.clean_shutdown();
        return Ok(true);
    }

    fn clean_shutdown(&mut self) -> bool {
        if self.receiver.stream_out().input_ended() && !self.sender.stream_in().eof() {
            self.linger = false;
        }
        if self.sender.stream_in().eof()
            && self.sender.bytes_in_flight() == 0
            && self.receiver.stream_out().input_ended()
        {
            if !self.linger || self.time_since_last_segment_received() >= 10 * self.cfg.rt_timeout {
                self.active = false;
            }
        }
        return !self.active;
    }

    fn unclean_shutdown(&mut self, send_rst: bool) {}

    fn in_listen(&self) -> bool {
        self.receiver.ackno().is_none() && self.sender.next_seqno_abs() == 0
    }

    fn in_syn_recv(&mut self) -> bool {
        let res = self.receiver.ackno().is_some() && !self.receiver.stream_out().input_ended();
        if res {}
        res
    }

    fn in_syn_sent(&self) -> bool {
        self.sender.next_seqno_abs() > 0
            && self.sender.bytes_in_flight() == self.sender.next_seqno_abs() as _
    }
}

pub trait CanSendData {}
impl CanSendData for TCPConnection<Established, SenderEstablished, ReceiverEstablished> {}

pub trait CanReceiveData {}
impl CanReceiveData for TCPConnection<Established, SenderEstablished, ReceiverEstablished> {}
impl CanReceiveData for TCPConnection<FinWait1, SenderFinSent, ReceiverEstablished> {}
impl CanReceiveData for TCPConnection<FinWait2, SenderFinAcknowledged, ReceiverEstablished> {}

impl<T: TCPState, S: SenderState, R: ReceiverState> TCPConnection<T, S, R> where Self: CanSendData {}

impl<T: TCPState, S: SenderState, R: ReceiverState> TCPConnection<T, S, R> where Self: CanReceiveData
{}

// public:
// //! \name "Input" interface for the writer
// //!@{

// //! \brief Initiate a connection by sending a SYN segment
// void connect();

// //! \brief Write data to the outbound byte stream, and send it over TCP if possible
// //! \returns the number of bytes from `data` that were actually written.
// size_t write(const std::string &data);

// //! \returns the number of `bytes` that can be written right now.
// size_t remaining_outbound_capacity() const;

// //! \brief Shut down the outbound byte stream (still allows reading incoming data)
// void end_input_stream();
// //!@}

// //! \name "Output" interface for the reader
// //!@{

// //! \brief The inbound byte stream received from the peer
// ByteStream &inbound_stream() { return _receiver.stream_out(); }
// //!@}

// //! \name Accessors used for testing

// //!@{
// //! \brief number of bytes sent and not yet acknowledged, counting SYN/FIN each as one byte
// size_t bytes_in_flight() const;
// //! \brief number of bytes not yet reassembled
// size_t unassembled_bytes() const;
// //! \brief Number of milliseconds since the last segment was received
// size_t time_since_last_segment_received() const;
// //!< \brief summarize the state of the sender, receiver, and the connection
// TCPState state() const { return {_sender, _receiver, active(), _linger_after_streams_finish}; };
// //!@}

// //! \name Methods for the owner or operating system to call
// //!@{

// //! Called when a new segment has been received from the network
// void segment_received(const TCPSegment &seg);

// //! Called periodically when time elapses
// void tick(const size_t ms_since_last_tick);

// //! \brief TCPSegments that the TCPConnection has enqueued for transmission.
// //! \note The owner or operating system will dequeue these and
// //! put each one into the payload of a lower-layer datagram (usually Internet datagrams (IP),
// //! but could also be user datagrams (UDP) or any other kind).
// std::queue<TCPSegment> &segments_out() { return _segments_out; }

// //! \brief Is the connection still alive in any way?
// //! \returns `true` if either stream is still running or if the TCPConnection is lingering
// //! after both streams have finished (e.g. to ACK retransmissions from the peer)
// bool active() const;
// //!@}

// //! Construct a new connection from a configuration
// explicit TCPConnection(const TCPConfig &cfg) : _cfg{cfg} {}

// //! \name construction and destruction
// //! moving is allowed; copying is disallowed; default construction not possible

// //!@{
// ~TCPConnection();  //!< destructor sends a RST if the connection is still open
// TCPConnection() = delete;
// TCPConnection(TCPConnection &&other) = default;
// TCPConnection &operator=(TCPConnection &&other) = default;
// TCPConnection(const TCPConnection &other) = delete;
// TCPConnection &operator=(const TCPConnection &other) = delete;
// //!@}
// };

// #endif  // SPONGE_LIBSPONGE_TCP_FACTORED_HH
