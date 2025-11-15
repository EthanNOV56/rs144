use std::{cmp::Ordering, collections::VecDeque, io::IoSlice};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum BufferError {
    #[error("Index out of bounds.")]
    IndexOutOfBounds,
    #[error("Buffer is not contiguous, use BufferList::into<String>() instead.")]
    NotContiguous,
}

#[derive(Default, Clone, Debug)]
struct Buffer {
    storage: Vec<u8>, // TODO: Reimplement based on Arc.
    starting_offset: usize,
}

impl From<Vec<u8>> for Buffer {
    #[inline(always)]
    fn from(v: Vec<u8>) -> Self {
        Buffer {
            storage: v,
            starting_offset: 0,
        }
    }
}

impl AsRef<[u8]> for Buffer {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        &self.storage[self.starting_offset..]
    }
}

impl AsMut<[u8]> for Buffer {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.storage[self.starting_offset..]
    }
}

impl Buffer {
    #[inline(always)]
    pub fn try_at(&self, n: usize) -> Option<u8> {
        self.as_ref().get(n).copied()
    }

    #[inline(always)]
    pub fn at_mut(&mut self, n: usize) -> u8 {
        self.try_at_mut(n).expect("Buffer index out of bounds")
    }

    #[inline(always)]
    pub fn try_at_mut(&mut self, n: usize) -> Option<u8> {
        self.as_mut().get(n).copied()
    }

    #[inline(always)]
    pub fn at(&self, n: usize) -> u8 {
        self.try_at(n).expect("Buffer index out of bounds")
    }

    #[inline(always)]
    pub fn size(&self) -> usize {
        self.as_ref().len()
    }

    #[inline(always)]
    pub fn clone_storage(&self) -> Vec<u8> {
        self.storage.clone()
    }

    #[inline(always)]
    pub fn remove_prefix(&mut self, n: &mut usize) -> &[u8] {
        self.try_remove_prefix(n)
            .expect("This operation should not fail")
    }

    pub fn try_remove_prefix(&mut self, n: &mut usize) -> Result<&[u8], BufferError> {
        let sz = self.size();

        if *n > sz {
            return Err(BufferError::IndexOutOfBounds);
        }
        let slice = &self.storage[self.starting_offset..self.starting_offset + *n];
        self.starting_offset += *n;
        *n = 0;
        Ok(slice)
        // if (_storage and _starting_offset == _storage->size()) {
        //     _storage.reset();
        // }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.size() == 0
    }
}

#[derive(Default, Clone, Debug)]
pub struct BufferList {
    buffers: VecDeque<Buffer>,
}

impl From<Buffer> for BufferList {
    #[inline(always)]
    fn from(buffer: Buffer) -> Self {
        BufferList {
            buffers: VecDeque::from([buffer]),
        }
    }
}

impl From<Vec<u8>> for BufferList {
    #[inline(always)]
    fn from(v: Vec<u8>) -> Self {
        BufferList {
            buffers: VecDeque::from([Buffer::from(v)]),
        }
    }
}

impl AsRef<VecDeque<Buffer>> for BufferList {
    #[inline(always)]
    fn as_ref(&self) -> &VecDeque<Buffer> {
        &self.buffers
    }
}

impl AsMut<VecDeque<Buffer>> for BufferList {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut VecDeque<Buffer> {
        &mut self.buffers
    }
}

impl TryInto<Buffer> for &BufferList {
    type Error = BufferError;

    #[inline(always)]
    fn try_into(self) -> Result<Buffer, Self::Error> {
        match self.buffers.len() {
            0 => Ok(Buffer::default()),
            1 => Ok(self.buffers[0].clone()),
            _ => Err(BufferError::NotContiguous),
        }
    }
}

impl Into<Vec<u8>> for &BufferList {
    fn into(self) -> Vec<u8> {
        let size = self.size();
        self.iter().map(|buf| buf.as_ref().to_vec()).fold(
            Vec::with_capacity(size),
            |mut acc, buf| {
                acc.extend(buf);
                acc
            },
        )
    }
}

impl BufferList {
    #[inline(always)]
    pub fn size(&self) -> usize {
        self.iter().map(|buf| buf.len()).sum()
    }

    pub fn try_remove_prefix(&mut self, mut n: usize) -> Result<Vec<u8>, BufferError> {
        let mut vec = Vec::with_capacity(n);
        while let Some(buffer) = self.buffers.front_mut() {
            let mut sub = n.min(buffer.size());
            n -= sub;
            vec.extend(buffer.remove_prefix(&mut sub));
            if buffer.is_empty() {
                self.buffers.pop_front();
            }
            if n == 0 {
                return Ok(vec);
            }
        }

        Err(BufferError::IndexOutOfBounds)
    }

    #[inline(always)]
    pub fn append(&mut self, other: BufferList) {
        self.buffers.extend(other.buffers);
    }

    fn iter(&self) -> impl Iterator<Item = &[u8]> {
        self.buffers.iter().map(|buf| buf.as_ref())
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut [u8]> {
        self.buffers.iter_mut().map(|buf| buf.as_mut())
    }
}

#[derive(Debug)]
struct BufferViewList<'a> {
    views: VecDeque<&'a [u8]>,
}

impl<'a> From<&'a [u8]> for BufferViewList<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        BufferViewList {
            views: VecDeque::from([bytes]),
        }
    }
}

// impl<'a> From<CStr> for BufferViewList<'a> {
//     fn from(cstr: CStr) -> Self {
//         BufferViewList {
//             views: VecDeque::from([cstr.into()]),
//         }
//     }
// }

impl<'a> From<&'a BufferList> for BufferViewList<'a> {
    fn from(buffers: &'a BufferList) -> Self {
        BufferViewList {
            views: buffers.iter().map(|buf| buf.as_ref()).collect(),
        }
    }
}

impl<'a> From<&'a str> for BufferViewList<'a> {
    fn from(s: &'a str) -> Self {
        BufferViewList {
            views: VecDeque::from([s.as_bytes()]),
        }
    }
}

impl<'a> BufferViewList<'a> {
    pub fn try_remove_prefix(&mut self, mut n: usize) -> Result<(), BufferError> {
        while let Some(buffer) = self.views.front_mut() {
            let sz = buffer.len();
            let mut drop = false;
            match sz.cmp(&n) {
                Ordering::Less => {
                    n -= sz;
                    drop = true;
                }
                Ordering::Equal => {
                    n = 0;
                    drop = true;
                }
                Ordering::Greater => {
                    *buffer = &buffer[n..];
                    n = 0;
                }
            }
            if drop {
                self.views.pop_front();
            }
            if n == 0 {
                return Ok(());
            }
        }
        Err(BufferError::IndexOutOfBounds)
    }
    pub fn size(&self) -> usize {
        self.views.iter().map(|view| view.len()).sum()
    }
    pub fn as_iovecs(&self) -> Vec<IoSlice<'_>> {
        self.views.iter().map(|view| IoSlice::new(view)).collect()
    }
}

// size_t BufferViewList::size() const {
//     size_t ret = 0;
//     for (const auto &buf : _views) {
//         ret += buf.size();
//     }
//     return ret;
// }

// vector<iovec> BufferViewList::as_iovecs() const {
//     vector<iovec> ret;
//     ret.reserve(_views.size());
//     for (const auto &x : _views) {
//         ret.push_back({const_cast<char *>(x.data()), x.size()});
//     }
//     return ret;
// }
