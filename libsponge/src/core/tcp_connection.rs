use crate::{
    ByteStream, Milliseconds, TCPConfig, TCPReceiver, TCPSegment, TCPSender, tcp_state::*,
};

use std::{collections::VecDeque, marker::PhantomData};

pub struct TCPConnection<T: TCPState, S: SenderState, R: ReceiverState> {
    _tcp_state: PhantomData<T>,
    _sender_state: PhantomData<S>,
    _receiver_state: PhantomData<R>,

    cfg: TCPConfig, //TODO maybe not need to be carried around...
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
    fn push_segments_out(&mut self, send_syn: Option<bool>) -> bool {
        let send_syn = send_syn.unwrap_or(false);
        let in_syn_recv = self.in_syn_recv();
        self.sender.fill_window(Some(send_syn || in_syn_recv));
        while let Some(mut seg) = self.sender.segments_out.pop_front() {
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
        return true;
    }

    fn clean_shutdown(&mut self) -> bool {
        if self.receiver.stream_out().input_ended() && !self.sender.stream_in().eof() {
            self.linger = false;
        }

        if self.sender.stream_in().eof()
            && self.sender.bytes_in_flight() == 0
            && self.receiver.stream_out().input_ended()
        {
            let ms: u64 = self.ms_since_last_seg_recv.into();
            if !self.linger || ms >= 10 * self.cfg.rt_timeout as u64 {
                self.active = false;
            }
        }
        return !self.active;
    }

    fn unclean_shutdown(&mut self, send_rst: bool) {
        self.receiver.stream_out_mut().set_error();
        self.sender.stream_in_mut().set_error();
        self.active = false;
        if send_rst {
            self.need_send_rst = true;
            if self.sender.segments_out_mut().is_empty() {
                self.sender.send_empty_segment(None);
            }
            self.push_segments_out(None);
        }
    }

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

impl<T: TCPState, S: SenderState, R: ReceiverState> TCPConnection<T, S, R> {
    pub fn with_config(cfg: &TCPConfig) -> Self {
        Self {
            active: true,
            ms_since_last_seg_recv: 0.into(),
            sender: TCPSender::with_config(cfg),
            receiver: TCPReceiver::with_config(cfg),
            segments_out: VecDeque::new(),
            cfg: cfg.clone(),
            linger: true,
            need_send_rst: false,
            ack_for_fin_sent: false,

            _tcp_state: PhantomData,
            _receiver_state: PhantomData,
            _sender_state: PhantomData,
        }
    }

    pub fn connect(&mut self) {
        self.push_segments_out(Some(true));
    }

    pub fn write(&mut self, data: &[u8]) -> usize {
        let ret = self.sender.stream_in_mut().write(data);
        self.push_segments_out(None);
        ret
    }

    #[inline(always)]
    pub fn remaining_outbound_capacity(&self) -> usize {
        self.sender.stream_in().remaining_capacity()
    }

    pub fn end_input_stream(&mut self) {
        self.sender.stream_in_mut().end_input();
        self.push_segments_out(None);
    }

    #[inline(always)]
    pub fn inbound_stream_mut(&mut self) -> &mut ByteStream {
        self.receiver.stream_out_mut()
    }

    #[inline(always)]
    pub fn bytes_in_flight(&self) -> usize {
        self.sender.bytes_in_flight()
    }

    #[inline(always)]
    pub fn unassembled_bytes(&self) -> usize {
        self.receiver.unassembled_bytes()
    }

    #[inline(always)]
    pub fn ms_since_last_seg_recv(&self) -> Milliseconds {
        self.ms_since_last_seg_recv
    }
    // TCPState state() const { return {_sender, _receiver, active(), _linger_after_streams_finish}; };

    // TODO: rewrite this function based on PSM.
    pub fn segment_received(&mut self, seg: &TCPSegment) {
        if !self.active {
            return;
        }

        self.ms_since_last_seg_recv = 0.into();

        if self.in_syn_sent() && seg.header().ack && !seg.payload().is_empty() {
            return;
        }

        let mut send_empty = false;
        if self.sender.next_seqno_abs() > 0
            && seg.header().ack
            && !self
                .sender
                .ack_received(&seg.header().ack_no, seg.header().win)
        {
            send_empty = true;
        }

        if !self.receiver.segment_received(seg) {
            send_empty = true;
        }

        if seg.header().syn && self.sender.next_seqno_abs() == 0 {
            self.connect();
        }

        if seg.header().rst {
            if self.in_syn_sent() && !seg.header().ack {
                return;
            }
            self.unclean_shutdown(false);
            return;
        }

        if seg.length_in_sequence_space() > 0 {
            send_empty = true;
        }

        if send_empty && self.receiver.ackno().is_some() && self.sender.segments_out().is_empty() {
            self.sender.send_empty_segment(None);
        }

        self.push_segments_out(None);
    }

    pub fn tick(&mut self, ms_since_last_tick: Milliseconds) {
        if !self.active {
            return;
        }

        self.ms_since_last_seg_recv += ms_since_last_tick;
        self.sender.tick(ms_since_last_tick);

        if self.sender.consq_retxs() > TCPConfig::MAX_RETX_ATTEMPTS as usize {
            self.unclean_shutdown(true);
        }
        self.push_segments_out(None);
    }

    pub fn segments_out_mut(&mut self) -> &mut VecDeque<TCPSegment> {
        &mut self.segments_out
    }

    #[inline(always)]
    pub fn active(&self) -> bool {
        self.active
    }
}

impl<T: TCPState, S: SenderState, R: ReceiverState> Drop for TCPConnection<T, S, R> {
    fn drop(&mut self) {
        if self.active {
            self.unclean_shutdown(true);
        }
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
