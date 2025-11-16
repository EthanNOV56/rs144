use crate::ByteStream;

use thiserror::Error;

use std::collections::BTreeMap;

#[derive(Debug, Error)]
pub enum ReassemblyError {
    #[error("Uninterleaved blocks")]
    UnInterleavedBlocks,
}

#[derive(Debug, Default)]
struct BlockNode {
    begin: usize,
    data: Vec<u8>,
}

impl BlockNode {
    pub fn new(begin: usize, data: &[u8]) -> Self {
        BlockNode {
            begin,
            data: data.to_vec(),
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn can_merge(&self, other: &Self) -> bool {
        self.begin + self.len() >= other.begin || self.begin <= other.begin + other.len()
    }

    pub fn merge(self, other: Self) -> Self {
        if !self.can_merge(&other) {
            panic!(
                "Cannot merge uninterleaved blocks, ownership taken.\n
                Use BlockNode::can_merge as predicate."
            )
        } else {
            if self.begin < other.begin {
                self.merge_impl(other)
            } else {
                other.merge_impl(self)
            }
        }
    }

    fn merge_impl(mut self, other: Self) -> Self {
        let self_end = self.begin + self.len();
        if self_end < other.begin + other.len() {
            self.data.extend_from_slice(&other.data[self_end..])
        }
        self
    }
}

#[derive(Debug, Default)]
pub struct StreamReassembler {
    pending_blocks: BTreeMap<usize, BlockNode>,
    buffer: Vec<u8>,
    unassemble_bytes: usize,
    head_index: usize,
    eof_flag: bool,
    output: ByteStream,
    capacity: usize,
}

impl StreamReassembler {
    fn judge_eof(&mut self, eof: bool) {
        if eof {
            self.eof_flag = true;
        }
        if self.eof_flag && self.buffer.is_empty() {
            self.output.end_input();
        }
    }

    pub fn new(capacity: usize) -> Self {
        StreamReassembler {
            capacity,
            output: ByteStream::new(capacity),
            buffer: Vec::with_capacity(capacity),
            ..Default::default()
        }
    }

    fn try_get_next_merge(&self, new_node: &BlockNode) -> Option<usize> {
        for (idx, node) in self.pending_blocks.range(new_node.begin..) {
            if new_node.can_merge(node) {
                return Some(*idx);
            }
        }
        None
    }

    fn try_get_prev_merge(&self, new_node: &BlockNode) -> Option<usize> {
        for (idx, node) in self.pending_blocks.range(..=new_node.begin).rev() {
            if new_node.can_merge(node) {
                return Some(*idx);
            }
        }
        None
    }

    fn remove_pending(&mut self, index: usize) -> BlockNode {
        let node = self.pending_blocks.remove(&index).unwrap();
        self.unassemble_bytes -= node.len();
        node
    }

    fn insert_pending(&mut self, node: BlockNode) {
        self.unassemble_bytes += node.len();
        self.pending_blocks.insert(node.begin, node);
    }

    pub fn push_substring(&mut self, data: &[u8], index: usize, eof: bool) {
        if index >= self.head_index + self.capacity {
            return;
        }

        let mut new_node;
        if index + data.len() <= self.head_index {
            return self.judge_eof(eof);
        } else if index < self.head_index {
            new_node = BlockNode::new(self.head_index, &data[(self.head_index - index)..]);
        } else {
            new_node = BlockNode::new(index, data);
        }

        // merge next
        while let Some(idx) = self.try_get_next_merge(&new_node) {
            let node_to_merge = self.remove_pending(idx);
            new_node = new_node.merge(node_to_merge);
        }

        // merge prev
        while let Some(idx) = self.try_get_prev_merge(&new_node) {
            let node_to_merge = self.remove_pending(idx);
            new_node = new_node.merge(node_to_merge);
        }

        self.insert_pending(new_node);

        // write to ByteStream
        if let Some((&idx, head)) = self.pending_blocks.first_key_value() {
            if head.begin == self.head_index {
                let head = self.remove_pending(idx);
                self.head_index += self.output.write(&head.data);
            }
        }
    }

    #[inline(always)]
    pub fn stream_out(&self) -> &ByteStream {
        &self.output
    }

    #[inline(always)]
    pub fn stream_out_mut(&mut self) -> &mut ByteStream {
        &mut self.output
    }

    #[inline(always)]
    pub fn unassemble_bytes(&self) -> usize {
        self.unassemble_bytes
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.unassemble_bytes == 0
    }

    #[inline(always)]
    pub fn head_index(&self) -> usize {
        self.head_index
    }

    #[inline(always)]
    pub fn input_ended(&self) -> bool {
        false
    }
}
