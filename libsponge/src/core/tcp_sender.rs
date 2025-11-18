use crate::{
    Buffer, ByteStream, Milliseconds, SenderState, TCPConfig, TCPConnectionError, TCPSegment,
    WrappingU32,
};

use anyhow::{Error, Result};

use std::collections::VecDeque;

// #[derive(Default)]
pub struct TCPSender {
    isn: WrappingU32,
    pub segments_out: VecDeque<TCPSegment>,
    initial_retx_timeout: Milliseconds,
    stream_in: ByteStream,
    next_seqno: u64,
    segments_outstanding: VecDeque<TCPSegment>,
    bytes_in_flight: usize,
    receiver_window_size: u16,
    receiver_free_space: u16,
    timer: Milliseconds,
    timer_running: bool,
    retx_timeout: Milliseconds,
    consq_retxs: usize,
    state: Result<SenderState>,
}

impl Default for TCPSender {
    fn default() -> Self {
        Self {
            isn: WrappingU32::default(),
            segments_out: VecDeque::new(),
            initial_retx_timeout: Milliseconds::default(),
            stream_in: ByteStream::default(),
            next_seqno: 0,
            segments_outstanding: VecDeque::new(),
            bytes_in_flight: 0,
            receiver_window_size: 0,
            receiver_free_space: 0,
            timer: Milliseconds::default(),
            timer_running: false,
            retx_timeout: Milliseconds::default(),
            consq_retxs: 0,
            state: Ok(SenderState::Closed),
        }
    }
}

impl TCPSender {
    fn ack_is_valid(&self, abs_ackno: usize) -> bool {
        abs_ackno <= self.next_seqno as usize
            && match self.segments_outstanding.front() {
                Some(seg) => {
                    abs_ackno
                        >= WrappingU32::unwrap(&seg.header().seq_no, &self.isn, self.next_seqno)
                            as _
                }
                None => true,
            }
    }

    fn send_segment(&mut self, mut seg: TCPSegment) {
        seg.header_mut().seq_no = WrappingU32::wrap(self.next_seqno, &self.isn);
        self.next_seqno += seg.length_in_sequence_space() as u64;
        self.bytes_in_flight += seg.length_in_sequence_space() as usize;
        match self.state {
            Ok(SenderState::SynSent) | Ok(SenderState::SynAcked) => {
                self.receiver_free_space -= seg.length_in_sequence_space() as u16
            }
            _ => {}
        }
        self.segments_outstanding.push_back(seg.clone());
        self.segments_out.push_back(seg);
        if !self.timer_running {
            self.timer_running = true;
            self.timer = 0.into();
        }
    }

    pub fn with_config(cfg: &TCPConfig) -> Self {
        let isn = cfg
            .fixed_isn
            .clone()
            .unwrap_or_else(|| WrappingU32::random());
        let timeout = (cfg.timeout_default as u64).into();
        Self {
            isn,
            initial_retx_timeout: timeout,
            retx_timeout: timeout,
            stream_in: ByteStream::new(cfg.send_capacity),
            ..Default::default()
        }
    }

    pub fn tick(&mut self, ms_since_last_tick: Milliseconds) {
        if !self.timer_running {
            return;
        }
        self.timer += ms_since_last_tick;
        if self.timer >= self.retx_timeout {
            self.segments_out
                .push_back(self.segments_outstanding.front().unwrap().clone());
            if self.receiver_window_size > 0
                || self.segments_outstanding.front().unwrap().header().syn
            {
                self.consq_retxs += 1;
                self.retx_timeout <<= 1;
            }
        }
    }

    pub fn stream_in(&self) -> &ByteStream {
        &self.stream_in
    }

    pub fn stream_in_mut(&mut self) -> &mut ByteStream {
        &mut self.stream_in
    }

    #[inline(always)]
    pub fn state(&self) -> &Result<SenderState> {
        &self.state
    }

    pub fn set_state(&mut self, state: Result<SenderState>) {
        match (&state, &self.state) {
            (Ok(targ), Ok(curr)) if targ != curr => self.state = state,
            (Err(_), _) => self.state = state,
            _ => {}
        }
    }

    pub fn renew_state(&mut self) -> &Result<SenderState> {
        match (
            self.stream_in().error(),
            self.next_seqno,
            self.stream_in().eof(),
            self.bytes_in_flight as u64,
            self.stream_in().bytes_written() as u64,
        ) {
            (true, _, _, _, _) => self.set_state(Err(Error::from(TCPConnectionError::SenderError))),
            (_, 0, _, _, _) => self.set_state(Ok(SenderState::Closed)),
            (_, i, _, j, _) if i == j => self.set_state(Ok(SenderState::SynSent)),
            (_, _, false, _, _) => self.set_state(Ok(SenderState::SynAcked)),
            (_, _, _, i, j) if i < j + 2 => self.set_state(Ok(SenderState::SynAcked)),
            (_, _, _, i, _) if i != 0 => self.set_state(Ok(SenderState::FinSent)),
            _ => self.set_state(Ok(SenderState::FinAcked)),
        }
        &self.state
    }

    pub fn ack_received(&mut self, ackno: &WrappingU32, window_size: u16) {
        let abs_ackno = WrappingU32::unwrap(ackno, &self.isn, self.next_seqno as _);
        if !self.ack_is_valid(abs_ackno as _) {
            return;
        }

        self.receiver_window_size = window_size;
        self.receiver_free_space = window_size;

        while let Some(seg) = self.segments_outstanding.front() {
            if WrappingU32::unwrap(&seg.header().seq_no, &self.isn, self.next_seqno)
                + seg.length_in_sequence_space() as u64
                <= abs_ackno
            {
                self.bytes_in_flight -= seg.length_in_sequence_space();
                self.segments_outstanding.pop_front();
                self.timer = 0.into();
                self.retx_timeout = self.initial_retx_timeout;
                self.consq_retxs = 0;
            } else {
                break;
            }
        }

        if let Some(seg) = self.segments_outstanding.front() {
            self.receiver_free_space = ((abs_ackno + window_size as u64)
                - WrappingU32::unwrap(&seg.header().seq_no, &self.isn, self.next_seqno)
                - self.bytes_in_flight as u64) as _
        }
        if self.bytes_in_flight == 0 {
            self.timer_running = false;
        }
        self.fill_window();
    }

    pub fn send_empty_segment(&mut self) {
        let mut seg = TCPSegment::default();
        seg.header_mut().seq_no = WrappingU32::wrap(self.next_seqno, &self.isn);
        self.segments_out_mut().push_back(seg);
    }

    pub fn fill_window(&mut self) {
        match (
            self.state(),
            self.segments_outstanding.front(),
            self.stream_in(),
        ) {
            (Ok(SenderState::Closed), _, _) => {
                self.set_state(Ok(SenderState::SynSent));
                let mut seg = TCPSegment::default();
                seg.header_mut().syn = true;
                self.segments_out_mut().push_back(seg);
            }
            (_, Some(seg), _) if seg.header().syn => {
                self.set_state(Ok(SenderState::SynSent));
                return;
            }
            (_, _, s) if s.buffer_empty() && !s.eof() => return,
            (Ok(SenderState::FinSent), _, _) | (Ok(SenderState::FinAcked), _, _) => return,
            _ => {}
        }

        match (self.receiver_window_size, self.receiver_free_space) {
            (i, _) if i > 0 => {
                while self.receiver_free_space > 0 {
                    let mut seg = TCPSegment::default();
                    let payload_len = self
                        .stream_in
                        .buffer_size()
                        .min(self.receiver_free_space as _)
                        .min(TCPConfig::MAX_PAYLOAD_SIZE);
                    *seg.payload_mut() = Buffer::from(self.stream_in_mut().read(payload_len));
                    if self.stream_in.eof() && self.receiver_free_space as usize > payload_len {
                        seg.header_mut().fin = true;
                        self.set_state(Ok(SenderState::FinSent));
                    }
                    self.send_segment(seg);
                    if self.stream_in().buffer_empty() {
                        break;
                    }
                }
            }
            (_, 0) => {
                let mut seg = TCPSegment::default();
                match (self.stream_in.eof(), self.stream_in.buffer_empty()) {
                    (true, _) => {
                        seg.header_mut().fin = true;
                        self.set_state(Ok(SenderState::FinSent));
                        self.send_segment(seg);
                    }
                    (_, false) => {
                        *seg.payload_mut() = Buffer::from(self.stream_in.read(1));
                        self.send_segment(seg);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    pub fn bytes_in_flight(&self) -> usize {
        self.bytes_in_flight
    }

    pub fn consq_retxs(&self) -> usize {
        self.consq_retxs
    }

    pub fn segments_out(&self) -> &VecDeque<TCPSegment> {
        &self.segments_out
    }

    pub fn segments_out_mut(&mut self) -> &mut VecDeque<TCPSegment> {
        &mut self.segments_out
    }

    pub fn next_seqno_abs(&self) -> u32 {
        self.next_seqno as _
    }

    pub fn next_seqno(&self) -> WrappingU32 {
        WrappingU32::wrap(self.next_seqno, &self.isn)
    }
}
