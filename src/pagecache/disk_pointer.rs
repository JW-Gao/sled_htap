use std::num::NonZeroU64;

use super::{HeapId, LogOffset};
use crate::*;

/// A pointer to a location on disk or an off-log heap item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskPtr {
    /// Points to a value stored in the single-file log.
    Inline(LogOffset),
    /// Points to a value stored off-log in the heap.
    Heap(Option<NonZeroU64>, HeapId),
}

pub(crate) const STREAM_SELECTOR_MASK: u64 = 1 << 63;

impl DiskPtr {
    pub(crate) const fn new_inline(l: LogOffset) -> Self {
        DiskPtr::Inline(l)
    }

    pub(crate) const fn new_cold(l: LogOffset) -> Self {
        DiskPtr::Inline(l | STREAM_SELECTOR_MASK)
    }

    pub(crate) fn new_heap_item(lid: LogOffset, heap_id: HeapId) -> Self {
        DiskPtr::Heap(Some(NonZeroU64::new(lid).unwrap()), heap_id)
    }

    pub(crate) const fn is_inline(&self) -> bool {
        matches!(self, DiskPtr::Inline(_))
    }

    pub(crate) fn is_cold(&self) -> bool {
        match self {
            DiskPtr::Inline(lid) => lid & STREAM_SELECTOR_MASK != 0,
            DiskPtr::Heap(Some(lid), _) => lid.get() & STREAM_SELECTOR_MASK != 0,
            _ => false,
        }
    }

    pub(crate) fn to_raw_offset(&self) -> LogOffset {
        match self {
            DiskPtr::Inline(lid) => lid & !STREAM_SELECTOR_MASK,
            DiskPtr::Heap(Some(lid), _) => lid.get() & !STREAM_SELECTOR_MASK,
            _ => panic!("called to_raw_offset on invalid DiskPtr"),
        }
    }

    pub(crate) const fn is_heap_item(&self) -> bool {
        matches!(self, DiskPtr::Heap(_, _))
    }

    pub(crate) const fn heap_id(&self) -> Option<HeapId> {
        if let DiskPtr::Heap(_, heap_id) = self {
            Some(*heap_id)
        } else {
            None
        }
    }

    #[doc(hidden)]
    pub fn lid(&self) -> Option<LogOffset> {
        match self {
            DiskPtr::Inline(lid) => Some(*lid & !STREAM_SELECTOR_MASK),
            DiskPtr::Heap(Some(lid), _) => Some(lid.get() & !STREAM_SELECTOR_MASK),
            DiskPtr::Heap(None, _) => None,
        }
    }

    pub(crate) fn forget_heap_log_coordinates(&mut self) {
        match self {
            DiskPtr::Inline(_) => {}
            DiskPtr::Heap(ref mut opt, _) => *opt = None,
        }
    }

    pub(crate) const fn original_lsn(&self) -> Lsn {
        match self {
            DiskPtr::Heap(_, heap_id) => heap_id.original_lsn,
            DiskPtr::Inline(_) => panic!("called original_lsn on non-Heap"),
        }
    }

    pub(crate) const fn heap_pointer_merged_into_snapshot(&self) -> bool {
        matches!(self, DiskPtr::Heap(None, _))
    }
}

impl fmt::Display for DiskPtr {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> std::result::Result<(), fmt::Error> {
        write!(f, "{:?}", self)
    }
}
