use crate::{
    ByteStream, ReceiverState, StreamReassembler, TCPConfig, TCPConnectionError, TCPSegment,
    WrappingU32,
};

use anyhow::{Error, Result};

pub struct TCPReceiver {
    reassembler: StreamReassembler,
    isn: WrappingU32,
    capacity: usize,
    state: Result<ReceiverState>,
}

impl TCPReceiver {
    pub fn new(capacity: usize) -> Self {
        TCPReceiver {
            reassembler: StreamReassembler::new(capacity),
            isn: WrappingU32::new(0),
            capacity,
            state: Ok(ReceiverState::default()),
        }
    }

    pub fn with_config(cfg: &TCPConfig) -> Self {
        let capa = cfg.recv_capacity;
        TCPReceiver {
            reassembler: StreamReassembler::new(capa),
            isn: WrappingU32::new(0),
            capacity: capa,
            state: Ok(ReceiverState::default()),
        }
    }

    pub fn ackno(&self) -> Option<WrappingU32> {
        let idx = self.reassembler.head_index() as u64;
        match (&self.state, self.reassembler.is_empty()) {
            (Ok(ReceiverState::Listen), _) => None,
            (Ok(ReceiverState::FinRcvd), true) => Some(WrappingU32::wrap(idx + 2, &self.isn)),
            _ => Some(WrappingU32::wrap(idx + 1, &self.isn)),
        }
    }

    #[inline(always)]
    pub fn win_size(&self) -> usize {
        self.capacity - self.reassembler.stream_out().buffer_size()
    }

    #[inline(always)]
    pub fn unassembled_bytes(&self) -> usize {
        self.reassembler.unassemble_bytes()
    }

    pub fn segment_received(&mut self, seg: &TCPSegment) {
        let header = seg.header();
        match (header.syn, header.fin, &self.state) {
            (true, fin, Ok(ReceiverState::Listen)) => {
                self.set_state(Ok(ReceiverState::SynRcvd));
                self.isn = header.seq_no.clone();
                if fin {
                    self.set_state(Ok(ReceiverState::FinRcvd));
                }
                self.reassembler
                    .push_substring(seg.payload().as_ref(), 0, fin);
                return;
            }
            (_, true, Ok(ReceiverState::SynRcvd)) => {
                self.set_state(Ok(ReceiverState::FinRcvd));
            }
            _ => {}
        }
        let check_point = self.reassembler.head_index();
        let mut abs_seqno = WrappingU32::unwrap(&header.seq_no, &self.isn, check_point as _);
        match self.state {
            Ok(ReceiverState::SynRcvd) => abs_seqno -= 1,
            _ => {}
        }
        self.reassembler
            .push_substring(seg.payload().as_ref(), abs_seqno as _, header.fin);
    }

    #[inline(always)]
    pub fn stream_out(&self) -> &ByteStream {
        self.reassembler.stream_out()
    }

    #[inline(always)]
    pub fn stream_out_mut(&mut self) -> &mut ByteStream {
        self.reassembler.stream_out_mut()
    }

    pub fn renew_state(&mut self) -> &Result<ReceiverState> {
        match (
            self.stream_out().error(),
            self.ackno(),
            self.stream_out().input_ended(),
        ) {
            (true, _, _) => self.set_state(Err(Error::from(TCPConnectionError::ReceiverError))),
            (_, None, _) => self.set_state(Ok(ReceiverState::Listen)),
            (_, _, true) => self.set_state(Ok(ReceiverState::FinRcvd)),
            _ => self.set_state(Ok(ReceiverState::SynRcvd)),
        }
        &self.state
    }

    #[inline(always)]
    pub fn state(&self) -> &Result<ReceiverState> {
        &self.state
    }

    pub fn set_state(&mut self, state: Result<ReceiverState>) {
        match (&state, &self.state) {
            (Ok(targ), Ok(curr)) if targ != curr => self.state = state,
            (Err(_), _) => self.state = state,
            _ => {}
        }
    }
}
