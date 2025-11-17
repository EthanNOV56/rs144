use crate::{ByteStream, TCPConfig, TCPSegment, WrappingU32, Milliseconds};

use std::collections::VecDeque;

#[derive(Debug, Default)]
pub struct TCPSender {
    isn: WrappingU32,
    segments_out: VecDeque<TCPSegment>,
    initial_retx_timeout: Milliseconds,
    stream_in: ByteStream,
    next_seqno: u64,
    segments_outstanding: VecDeque<TCPSegment>,
    bytes_in_flight: usize,
    recv_ackno: usize,
    syn_flag: bool,
    fin_flag: bool,
    window_size: usize,
    timer: usize,
    timer_running: bool,
    retx_timeout: Milliseconds,
    consq_retxs: usize,
}

impl TCPSender {
    fn send_segment(&self, : TCPSegment) {}

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

    pub fn get_stream_in(&self) -> &ByteStream {
        &self.stream_in
    }

    pub fn get_stream_in_mut(&mut self) -> &mut ByteStream {
        &mut self.stream_in
    }

    pub fn ack_received(&mut self, ackno: WrappingU32, window_size: u16) -> bool {
        true
    }

    pub fn send_empty_segment(&self, seqno: Option<WrappingU32>) {}

    pub fn fill_window(&mut self, send_syn: Option<bool>) {}

    pub fn tick(&mut self, ms_since_last_tick: usize) {}

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
