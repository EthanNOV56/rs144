use crate::{
    ByteStream, Milliseconds, TCPConfig, TCPReceiver, TCPSegment, TCPSender, tcp_state::*,
};

use anyhow::{Error, Result};

use std::collections::VecDeque;

pub struct TCPConnection {
    cfg: TCPConfig, //TODO maybe not need to be carried around...
    sender: TCPSender,
    receiver: TCPReceiver,
    segments_out: VecDeque<TCPSegment>,
    linger: bool,
    ms_since_last_seg_recv: Milliseconds,
    active: bool,
    state: Result<TCPState>,
}

impl TCPConnection {
    fn set_rst(&mut self) {
        self.sender
            .set_state(Err(Error::from(TCPConnectionError::SenderError)));
        self.receiver
            .set_state(Err(Error::from(TCPConnectionError::ReceiverError)));
        self.active = false;
    }

    fn send_rst(&mut self) {
        self.sender.send_empty_segment();
        let mut rst_seg = self.sender.segments_out_mut().pop_front().unwrap();
        self.set_ack_and_winsize(&mut rst_seg);
        rst_seg.header_mut().rst = true;
        self.segments_out.push_back(rst_seg);
    }

    fn real_send(&mut self) -> bool {
        let mut sent = false;
        while let Some(mut seg) = self.sender.segments_out_mut().pop_front() {
            self.set_ack_and_winsize(&mut seg);
            self.segments_out.push_back(seg);
            sent = true;
        }
        sent
    }

    fn set_ack_and_winsize(&self, seg: &mut TCPSegment) {
        if let Some(ackno) = self.receiver.ackno() {
            seg.header_mut().ack = true;
            seg.header_mut().ack_no = ackno;
        }
        seg.header_mut().win = self.receiver.win_size() as _;
    }

    fn inbound_ended(&self) -> bool {
        self.receiver.unassembled_bytes() == 0 && self.receiver.stream_out().input_ended()
    }

    fn outbound_ended(&self) -> bool {
        self.sender.stream_in().eof()
            && self.sender.next_seqno_abs() as usize == self.sender.stream_in().bytes_written() + 2
            && self.sender.bytes_in_flight() == 0
    }

    fn linger_mut(&mut self) -> bool {
        if self.inbound_ended() && !self.sender.stream_in().eof() {
            self.linger = false;
        }
        self.linger
    }

    fn active_mut(&mut self) -> bool {
        if self.inbound_ended() && self.outbound_ended() {
            let cfg_to: Milliseconds = (self.cfg.rt_timeout as u64).into();
            if !self.linger || self.ms_since_last_seg_recv >= cfg_to * 10 {
                self.active = false;
            }
        }
        self.active
    }
}

impl TCPConnection {
    pub fn with_config(cfg: &TCPConfig) -> Self {
        Self {
            active: true,
            ms_since_last_seg_recv: 0.into(),
            sender: TCPSender::with_config(cfg),
            receiver: TCPReceiver::with_config(cfg),
            segments_out: VecDeque::new(),
            cfg: cfg.clone(),
            linger: true,
            state: Ok(TCPState::default()),
        }
    }

    pub fn connect(&mut self) {
        self.sender.fill_window();
        self.real_send();
    }

    pub fn write(&mut self, data: &[u8]) -> usize {
        if data.len() == 0 {
            return 0;
        }
        let ret = self.sender.stream_in_mut().write(data);
        self.sender.fill_window();
        self.real_send();
        ret
    }

    #[inline(always)]
    pub fn remaining_outbound_capacity(&self) -> usize {
        self.sender.stream_in().remaining_capacity()
    }

    pub fn end_input_stream(&mut self) {
        self.sender.stream_in_mut().end_input();
        self.sender.fill_window();
        self.real_send();
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

    pub fn renew_state(&mut self) -> &Result<TCPState> {
        self.active_mut();
        self.linger_mut();
        self.state = match (self.sender.renew_state(), self.receiver.renew_state()) {
            (Err(_), Err(_)) if !self.linger && !self.active => Ok(TCPState::Reset),
            (Ok(SenderState::Closed), Ok(ReceiverState::Listen)) => Ok(TCPState::Listen),
            (Ok(SenderState::SynSent), Ok(ReceiverState::Listen)) => Ok(TCPState::SynSent),
            (Ok(SenderState::SynSent), Ok(ReceiverState::SynRcvd)) => Ok(TCPState::SynRcvd),
            (Ok(SenderState::SynAcked), Ok(ReceiverState::SynRcvd)) => Ok(TCPState::Established),
            (Ok(SenderState::SynAcked), Ok(ReceiverState::FinRcvd)) if !self.linger => {
                Ok(TCPState::CloseWait)
            }
            (Ok(SenderState::FinSent), Ok(ReceiverState::FinRcvd)) if !self.linger => {
                Ok(TCPState::LastAck)
            }
            (Ok(SenderState::FinSent), Ok(ReceiverState::FinRcvd)) => Ok(TCPState::Closing),
            (Ok(SenderState::FinAcked), Ok(ReceiverState::FinRcvd))
                if !self.linger && !self.active =>
            {
                Ok(TCPState::Closed)
            }
            (Ok(SenderState::FinSent), Ok(ReceiverState::SynRcvd)) => Ok(TCPState::FinWait1),
            (Ok(SenderState::FinAcked), Ok(ReceiverState::SynRcvd)) => Ok(TCPState::FinWait2),
            (Ok(SenderState::FinAcked), Ok(ReceiverState::FinRcvd)) => Ok(TCPState::TimeWait),
            _ => Err(Error::from(TCPConnectionError::UnknownConnectionState)),
        };
        &self.state
    }

    // TODO: rewrite this function based on PSM?
    pub fn segment_received(&mut self, seg: &TCPSegment) {
        self.ms_since_last_seg_recv = 0.into();
        if seg.header().rst {
            self.set_rst();
            return;
        }

        self.receiver.segment_received(seg);

        self.linger_mut();

        if seg.header().ack {
            self.sender
                .ack_received(&seg.header().ack_no, seg.header().win);
            self.real_send();
        }

        if seg.length_in_sequence_space() > 0 {
            self.sender.fill_window();
            if !self.real_send() {
                self.sender.send_empty_segment();
                let mut ack_seg = self.sender.segments_out_mut().pop_front().unwrap();
                self.set_ack_and_winsize(&mut ack_seg);
                self.segments_out.push_back(ack_seg);
            }
        }
    }

    pub fn tick(&mut self, ms_since_last_tick: Milliseconds) {
        if !self.active {
            return;
        }

        self.ms_since_last_seg_recv += ms_since_last_tick;
        self.sender.tick(ms_since_last_tick);

        if let Some(mut retx_seg) = self.sender.segments_out_mut().pop_front() {
            self.set_ack_and_winsize(&mut retx_seg);
            if self.sender.consq_retxs() > self.cfg.max_retx_attempts as _ {
                self.set_rst();
                retx_seg.header_mut().rst = true;
            }
            self.segments_out_mut().push_back(retx_seg);
        }

        self.linger_mut();
        self.active_mut();
    }

    pub fn segments_out_mut(&mut self) -> &mut VecDeque<TCPSegment> {
        &mut self.segments_out
    }

    #[inline(always)]
    pub fn active(&self) -> bool {
        self.active
    }
}

impl Drop for TCPConnection {
    fn drop(&mut self) {
        if self.active {
            self.set_rst();
            self.send_rst();
        }
    }
}
