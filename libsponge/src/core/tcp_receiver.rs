use crate::{ByteStream, StreamReassembler, TCPSegment, WrappingU32};

pub struct TCPReceiver {
    reassembler: StreamReassembler,
    syn_flag: bool,
    fin_flag: bool,
    base: usize,
    isn: usize,
    capacity: usize,
}

impl TCPReceiver {
    pub fn new(capacity: usize) -> Self {
        TCPReceiver {
            reassembler: StreamReassembler::new(capacity),
            syn_flag: false,
            fin_flag: false,
            base: 0,
            isn: 0,
            capacity,
        }
    }

    pub fn ackno(&self) -> Option<WrappingU32> {
        if self.base > 0 {
            Some(WrappingU32::wrap(
                self.base as _,
                &WrappingU32::new(self.isn as _),
            ))
        } else {
            None
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

    pub fn segment_received(&mut self, seg: &TCPSegment) -> bool {
        let mut ret = false;
        let mut abs_seq_no: usize = 0;
        let mut length;
        if seg.header().syn {
            if self.syn_flag {
                return false;
            }
            self.syn_flag = true;
            ret = true;
            self.isn = seg.header().seq_no.raw_val() as _;
            abs_seq_no = 1;
            self.base = 1;
            length = seg.length_in_sequence_space() - 1;
            if length == 0 {
                return true;
            }
        } else if !self.syn_flag {
            return false;
        } else {
            abs_seq_no = WrappingU32::unwrap(
                &WrappingU32::new(seg.header().seq_no.raw_val()),
                &WrappingU32::new(self.isn as _),
                abs_seq_no as _,
            ) as usize;
            length = seg.length_in_sequence_space();
        }

        if seg.header().fin {
            if self.fin_flag {
                return false;
            }
            self.fin_flag = true;
            ret = true;
        } else if seg.length_in_sequence_space() == 0 && abs_seq_no == self.base {
            return true;
        } else if abs_seq_no >= self.base + self.win_size() || abs_seq_no + length <= self.base {
            if !ret {
                return false;
            }
        }

        self.reassembler
            .push_substring(seg.payload().as_ref(), abs_seq_no - 1, seg.header().fin);
        self.base = self.reassembler.head_index() + 1;
        self.base += self.reassembler.input_ended() as usize;
        return true;
    }

    #[inline(always)]
    pub fn stream_out(&self) -> &ByteStream {
        self.reassembler.stream_out()
    }

    #[inline(always)]
    pub fn stream_out_mut(&mut self) -> &mut ByteStream {
        self.reassembler.stream_out_mut()
    }
}
