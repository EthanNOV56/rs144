use crate::{ByteStream, Milliseconds, TCPConfig, TCPSegment, WrappingU32};

use std::collections::VecDeque;

#[derive(Debug, Default)]
pub struct TCPSender {
    isn: WrappingU32,
    pub segments_out: VecDeque<TCPSegment>,
    initial_retx_timeout: Milliseconds,
    stream_in: ByteStream,
    next_seqno: u64,
    segments_outstanding: VecDeque<TCPSegment>,
    bytes_in_flight: usize,
    recv_ackno: usize,
    syn_flag: bool,
    fin_flag: bool,
    window_size: u16,
    timer: Milliseconds,
    timer_running: bool,
    retx_timeout: Milliseconds,
    consq_retxs: usize,
}

impl TCPSender {
    fn send_segment(&mut self, mut seg: TCPSegment) {
        seg.header_mut().seq_no = WrappingU32::wrap(self.next_seqno, &self.isn);
        self.next_seqno += seg.length_in_sequence_space() as u64;
        self.bytes_in_flight += seg.length_in_sequence_space() as usize;
        self.segments_outstanding.push_back(seg.clone());
        self.segments_out.push_back(seg);
        if !self.timer_running {
            self.timer_running = true;
            self.timer = 0.into();
        }
    }

    pub fn new(cfg: TCPConfig) -> Self {
        let isn = cfg.fixed_isn.unwrap_or_else(|| WrappingU32::random());
        let timeout = (cfg.timeout_default as u64).into();
        Self {
            isn,
            initial_retx_timeout: timeout,
            retx_timeout: timeout,
            stream_in: ByteStream::new(cfg.capacity),
            ..Default::default()
        }
    }

    pub fn stream_in(&self) -> &ByteStream {
        &self.stream_in
    }

    pub fn stream_in_mut(&mut self) -> &mut ByteStream {
        &mut self.stream_in
    }

    pub fn ack_received(&mut self, ackno: WrappingU32, window_size: u16) -> bool {
        let abs_ackno = WrappingU32::unwrap(&ackno, &self.isn, self.recv_ackno as _);
        // out of window, invalid ackno
        if abs_ackno > self.next_seqno {
            return false;
        }

        // if ackno is legal, modify _window_size before return
        self.window_size = window_size;

        // ack has been received
        if abs_ackno as usize <= self.recv_ackno {
            return true;
        }
        self.recv_ackno = abs_ackno as _;

        // pop all elment before ackno
        while let Some(seg) = self.segments_outstanding.front() {
            if WrappingU32::unwrap(&seg.header().seq_no, &self.isn, self.next_seqno)
                + seg.length_in_sequence_space() as u64
                <= abs_ackno
            {
                self.bytes_in_flight -= seg.length_in_sequence_space();
                self.segments_outstanding.pop_front();
            } else {
                break;
            }
        }

        self.fill_window(None);

        self.retx_timeout = self.initial_retx_timeout;
        self.consq_retxs = 0;

        // if have other outstanding segment, restart timer
        if !self.segments_outstanding.is_empty() {
            self.timer_running = true;
            self.timer = 0.into();
        }
        return true;
    }

    pub fn send_empty_segment(&mut self, seqno: Option<WrappingU32>) {
        let mut seg = TCPSegment::default();
        seg.header_mut().seq_no = seqno.unwrap_or(WrappingU32::wrap(self.next_seqno, &self.isn));
        self.segments_out_mut().push_back(seg);
    }

    pub fn fill_window(&mut self, send_syn: Option<bool>) {
        // sent a SYN before sent other segment
        let send_syn = send_syn.unwrap_or(true);
        if !self.syn_flag {
            if send_syn {
                let mut seg = TCPSegment::default();
                seg.header_mut().syn = true;
                self.send_segment(seg);
                self.syn_flag = true;
            }
            return;
        }

        // take window_size as 1 when it equal 0
        let win = self.window_size.min(1);
        let mut remain; // window's free space
        // when window isn't full and never sent FIN
        while !self.fin_flag {
            remain = win as usize - (self.next_seqno as usize - self.recv_ackno);
            if remain == 0 {
                break;
            }
            let mut seg = TCPSegment::default();
            *seg.payload_mut() = self.stream_in_mut().read(remain).into();
            if seg.length_in_sequence_space() < win as _ && self.stream_in.eof() {
                seg.header_mut().fin = true;
                self.fin_flag = true;
            }
            // stream is empty
            if seg.length_in_sequence_space() == 0 {
                return;
            }
            self.send_segment(seg);
        }
    }

    pub fn tick(&mut self, ms_since_last_tick: Milliseconds) {
        self.timer += ms_since_last_tick;
        match self.segments_outstanding.front() {
            Some(seg) => {
                if self.timer >= self.retx_timeout {
                    self.segments_out.push_back(seg.clone());
                    self.consq_retxs += 1;
                    self.retx_timeout *= 2;
                    self.timer_running = true;
                    self.timer = 0.into();
                }
            }
            None => self.timer_running = false,
        }
    }

    pub fn bytes_in_flight(&self) -> usize {
        self.bytes_in_flight
    }

    pub fn consq_retxs(&self) -> usize {
        self.consq_retxs
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
