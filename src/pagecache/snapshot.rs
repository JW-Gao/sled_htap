use crate::*;

use super::{
    arr_to_u32, pwrite_all, raw_segment_iter_from, u32_to_arr, u64_to_arr,
    BasedBuf, DiskPtr, DualLogIter, HeapId, LogIter, LogKind, LogOffset, Lsn,
    MessageKind,
};
use std::fs::File;

/// A snapshot of the state required to quickly restart
/// the `PageCache` and `SegmentAccountant`.
#[derive(PartialEq, Debug, Default)]
#[cfg_attr(test, derive(Clone))]
pub struct Snapshot {
    /// The version of the snapshot format
    pub version: u8,
    /// The last read message lsn
    pub stable_lsn: Option<Lsn>,
    /// The last read message lid
    pub active_segment: Option<LogOffset>,
    /// the mapping from pages to (lsn, lid)
    pub pt: Vec<PageState>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageState {
    /// Present signifies a page that has some data.
    ///
    /// It has two parts. The base and the fragments.
    /// `base` is separated to guarantee that it will
    /// always have at least one because it is
    /// correct by construction.
    /// The third element in each tuple is the on-log
    /// size for the corresponding write. If things
    /// are pretty large, they spill into the heaps
    /// directory, but still get a small pointer that
    /// gets written into the log. The sizes are used
    /// for the garbage collection statistics on
    /// segments. The lsn and the DiskPtr can be used
    /// for actually reading the item off the disk,
    /// and the size tells us how much storage it uses
    /// on the disk.
    Present {
        base: (Lsn, DiskPtr),
        frags: Vec<(Lsn, DiskPtr)>,
    },

    /// This is a free page.
    Free(Lsn, DiskPtr),
    Uninitialized,
}

impl PageState {
    fn push(&mut self, item: (Lsn, DiskPtr)) {
        match *self {
            PageState::Present { base, ref mut frags } => {
                if frags.last().map_or(base.0, |f| f.0) < item.0 {
                    frags.push(item)
                } else {
                    debug!(
                        "skipping merging item {:?} into \
                        existing PageState::Present({:?})",
                        item, frags
                    );
                }
            }
            _ => panic!("pushed frags to {:?}", self),
        }
    }

    pub(crate) const fn is_free(&self) -> bool {
        matches!(self, PageState::Free(_, _))
    }

    #[cfg(feature = "testing")]
    fn offsets(&self) -> Vec<Option<LogOffset>> {
        match *self {
            PageState::Present { base, ref frags } => {
                let mut offsets = vec![base.1.lid()];
                for (_, ptr) in frags {
                    offsets.push(ptr.lid());
                }
                offsets
            }
            PageState::Free(_, ptr) => vec![ptr.lid()],
            PageState::Uninitialized => {
                vec![]
            }
        }
    }

    pub(crate) fn heap_ids(&self) -> Vec<HeapId> {
        let mut ret = vec![];

        match *self {
            PageState::Present { base, ref frags } => {
                if let Some(heap_id) = base.1.heap_id() {
                    ret.push(heap_id);
                }
                for (_, ptr) in frags {
                    if let Some(heap_id) = ptr.heap_id() {
                        ret.push(heap_id);
                    }
                }
            }
            PageState::Free(_, ptr) => {
                if let Some(heap_id) = ptr.heap_id() {
                    ret.push(heap_id);
                }
            }
            PageState::Uninitialized => {
                // Return empty vector, which is `ret`
            }
        }

        ret
    }
}

impl Snapshot {
    pub fn recovered_coords(
        &self,
        segment_size: usize,
    ) -> (Option<LogOffset>, Option<Lsn>) {
        if self.stable_lsn.is_none() {
            return (None, None);
        }

        let stable_lsn = self.stable_lsn.unwrap();

        if let Some(base_offset) = self.active_segment {
            let progress = stable_lsn % segment_size as Lsn;
            let offset = base_offset + LogOffset::try_from(progress).unwrap();

            (Some(offset), Some(stable_lsn))
        } else {
            let lsn_idx = stable_lsn / segment_size as Lsn
                + if stable_lsn % segment_size as Lsn == 0 { 0 } else { 1 };
            let next_lsn = lsn_idx * segment_size as Lsn;
            (None, Some(next_lsn))
        }
    }

    fn apply(
        &mut self,
        log_kind: LogKind,
        pid: PageId,
        lsn: Lsn,
        disk_ptr: DiskPtr,
    ) -> Result<()> {
        trace!(
            "trying to deserialize buf for pid {} ptr {} lsn {}",
            pid,
            disk_ptr,
            lsn
        );
        #[cfg(feature = "metrics")]
        let _measure = Measure::new(&M.snapshot_apply);

        let pushed = if self.pt.len() <= usize::try_from(pid).unwrap() {
            self.pt.resize(
                usize::try_from(pid + 1).unwrap(),
                PageState::Uninitialized,
            );
            true
        } else {
            false
        };

        match log_kind {
            // 原来这是compact后的页面啊
            LogKind::Replace => {
                trace!(
                    "compact of pid {} at ptr {} lsn {}",
                    pid,
                    disk_ptr,
                    lsn,
                );

                let pid_usize = usize::try_from(pid).unwrap();

                self.pt[pid_usize] =
                    PageState::Present { base: (lsn, disk_ptr), frags: vec![] };
            }
            LogKind::Link => {
                // Because we rewrite pages over time, we may have relocated
                // a page's initial Compact to a later segment. We should skip
                // over pages here unless we've encountered a Compact for them.
                if let Some(lids @ PageState::Present { .. }) =
                    self.pt.get_mut(usize::try_from(pid).unwrap())
                {
                    trace!(
                        "append of pid {} at lid {} lsn {}",
                        pid,
                        disk_ptr,
                        lsn,
                    );

                    lids.push((lsn, disk_ptr));
                } else {
                    trace!(
                        "skipping dangling append of pid {} at lid {} lsn {}",
                        pid,
                        disk_ptr,
                        lsn,
                    );
                    if pushed {
                        let old = self.pt.pop().unwrap();
                        if old != PageState::Uninitialized {
                            error!(
                                "expected previous page state to be uninitialized"
                            );
                            return Err(Error::corruption(None));
                        }
                    }
                }
            }
            LogKind::Free => {
                trace!("free of pid {} at ptr {} lsn {}", pid, disk_ptr, lsn);
                self.pt[usize::try_from(pid).unwrap()] =
                    PageState::Free(lsn, disk_ptr);
            }
            LogKind::Corrupted | LogKind::Skip => {
                error!(
                    "unexpected messagekind in snapshot application for pid {}: {:?}",
                    pid, log_kind
                );
                return Err(Error::corruption(None));
            }
        }

        Ok(())
    }

    fn filter_inner_heap_ids(&mut self) {
        for page in &mut self.pt {
            match page {
                PageState::Free(_lsn, ref mut ptr) => {
                    ptr.forget_heap_log_coordinates()
                }
                PageState::Present { ref mut base, ref mut frags } => {
                    base.1.forget_heap_log_coordinates();
                    for (_, ref mut ptr) in frags {
                        ptr.forget_heap_log_coordinates();
                    }
                }
                PageState::Uninitialized => {
                    // This can happen if a snapshot is taken while the page table
                    // has been expanded but not yet filled. It is safe to ignore.
                }
            }
        }
    }
}

fn clean_tail_for_log(
    iter: &mut LogIter,
    snapshot_stable_lsn: Option<Lsn>,
    config: &RunningConfig,
    file: &File,
) -> Result<(Lsn, Option<LogOffset>)> {
    let no_recovery_progress = iter.cur_lsn.is_none()
        || iter.cur_lsn.unwrap() <= snapshot_stable_lsn.unwrap_or(0);
    
    // If no progress, we can't really claim a new stable tip for this log,
    // but the system as a whole might have advanced.
    // However, clean_tail returns the "tip" of this log.
    // If empty/no progress, we return previous state?
    // Actually, handling this logic inside advance_snapshot loop was messy.
    // Here we focus on zeroing the TEAR.

    if iter.cur_lsn.is_none() {
        // Nothing read from this log, or no progress.
        // Return 0, None? Or rely on caller to interpret?
        // If cur_lsn is None, it means we scanned nothing or everything was old.
        // We should just return early.
        return Ok((0, None));
    }

    let iterated_lsn = iter.cur_lsn.unwrap();

    let segment_progress: Lsn = iterated_lsn % (config.segment_size as Lsn);

    let monotonic = segment_progress >= SEG_HEADER_LEN as Lsn
        || (segment_progress == 0 && iter.segment_base.is_none());
    if !monotonic {
        error!(
            "expected segment progress {} to be above SEG_HEADER_LEN or == 0, cur_lsn: {}",
            segment_progress, iterated_lsn,
        );
        return Err(Error::corruption(None));
    }

    let (stable_lsn, active_segment) = if segment_progress
        + MAX_MSG_HEADER_LEN as Lsn
        >= config.segment_size as Lsn
    {
        let bumped =
            config.normalize(iterated_lsn) + config.segment_size as Lsn;
        trace!("bumping snapshot.stable_lsn to {}", bumped);
        (bumped, None)
    } else {
        if let Some(BasedBuf { offset, .. }) = iter.segment_base {
            let shred_len = config.segment_size
                - usize::try_from(segment_progress).unwrap()
                - 1;
            let shred_zone = vec![MessageKind::Corrupted.into(); shred_len];
            let shred_base =
                offset + LogOffset::try_from(segment_progress).unwrap();

            debug!(
                "zeroing the end of the recovered segment at lsn {} between lids {} and {}",
                config.normalize(iterated_lsn),
                shred_base,
                shred_base + shred_len as LogOffset
            );
            pwrite_all(file, &shred_zone, shred_base)?;
            config.file.sync_all()?;
        }
        (iterated_lsn, iter.segment_base.as_ref().map(|bb| bb.offset))
    };
    
    // Zero torn segments (the ones after the last valid message but before max_lsn)
    for (lsn, to_zero) in &iter.segments {
        debug!("zeroing torn segment at lsn {} lid {}", lsn, to_zero);
        io_fail!(config, "segment initial free zero");
        pwrite_all(
            file,
            &*vec![MessageKind::Corrupted.into(); config.segment_size],
            *to_zero,
        )?;
        if !config.temporary {
            config.file.sync_all()?;
        }
    }

    Ok((stable_lsn, active_segment))
}

fn advance_snapshot(
    mut iter: DualLogIter,
    mut snapshot: Snapshot,
    config: &RunningConfig,
) -> Result<Snapshot> {
    #[cfg(feature = "metrics")]
    let _measure = Measure::new(&M.advance_snapshot);

    trace!("building on top of old snapshot: {:?}", snapshot);

    let old_stable_lsn = snapshot.stable_lsn;

    for (log_kind, pid, lsn, ptr) in &mut iter {
        trace!(
            "in advance_snapshot looking at item with pid {} lsn {} ptr {}",
            pid,
            lsn,
            ptr
        );

        if lsn < snapshot.stable_lsn.unwrap_or(-1) {
            trace!(
                "continuing in advance_snapshot, lsn {} ptr {} stable_lsn {:?}",
                lsn,
                ptr,
                snapshot.stable_lsn
            );
            continue;
        }

        snapshot.apply(log_kind, pid, lsn, ptr)?;
    }

    // Clean up tails for Hot Log
    let (hot_stable, hot_active) = clean_tail_for_log(
        &mut iter.hot_iter, 
        snapshot.stable_lsn, 
        config, 
        &config.file
    )?;

    // Clean up tails for Cold Log (if valid)
    let (cold_stable, _cold_active) = if let Some(ref mut cold_iter) = iter.cold_iter {
        if let Some(ref cold_file) = config.cold_file {
             clean_tail_for_log(
                cold_iter, 
                snapshot.stable_lsn, 
                config, 
                cold_file
            )?
        } else {
            (0, None)
        }
    } else {
        (0, None)
    };
    
    // The new stable LSN is the max of the tips of both logs
    let new_stable_lsn = std::cmp::max(hot_stable, cold_stable);
    
    // If we made progress beyond old stable
    if new_stable_lsn > snapshot.stable_lsn.unwrap_or(0) {
        snapshot.stable_lsn = Some(new_stable_lsn);
        // Active segment depends on which log stream is "active" for new allocations?
        // Actually, SegmentAccountant tracks active segments for BOTH.
        // Snapshot struct only stores ONE `active_segment`?
        // `pub active_segment: Option<LogOffset>`
        // This seems to refer to the Hot Log active segment (since we start SA with it).
        // Since we split hot/cold, SA handles cold segments separately?
        // If we restore from snapshot, we need to know Hot Active Segment.
        // Cold Active Segment is recovered by `recover_tip`?
        // Yes, `Log::start` calls `recover_tip` for cold log.
        // So we only need to persist `hot_active` in snapshot.
        snapshot.active_segment = hot_active;
    }

    snapshot.filter_inner_heap_ids();

    trace!("generated snapshot: {:?}", snapshot);

    if snapshot.stable_lsn < old_stable_lsn {
        error!("unexpected corruption encountered in storage snapshot file");
        return Err(Error::corruption(None));
    }

    if snapshot.stable_lsn > old_stable_lsn {
        write_snapshot(config, &snapshot)?;
    }

    #[cfg(feature = "event_log")]
    config.event_log.recovered_lsn(snapshot.stable_lsn.unwrap_or(0));

    Ok(snapshot)
}

/// Read a `Snapshot` or generate a default, then advance it to
/// the tip of the data file, if present.
pub fn read_snapshot_or_default(config: &RunningConfig) -> Result<Snapshot> {
    // NB we want to error out if the read snapshot was corrupted.
    // We only use a default Snapshot when there is no snapshot found.
    let last_snap = read_snapshot(config)?.unwrap_or_default();

    let hot_iter = raw_segment_iter_from(
        last_snap.stable_lsn.unwrap_or(0), 
        config, 
        config.file.clone()
    )?;
    
    let cold_iter = if let Some(ref cold_file) = config.cold_file {
        Some(raw_segment_iter_from(
             last_snap.stable_lsn.unwrap_or(0), 
             config, 
             cold_file.clone()
        )?)
    } else {
        None
    };
    
    let dual_iter = DualLogIter::new(hot_iter, cold_iter);

    let res = advance_snapshot(dual_iter, last_snap, config)?;

    Ok(res)
}

/// Read a `Snapshot` from disk.
/// Returns an error if the read snapshot was corrupted.
/// Returns `Ok(Some(snapshot))` if there was nothing written.
fn read_snapshot(config: &RunningConfig) -> Result<Option<Snapshot>> {
    let mut candidates = config.get_snapshot_files()?;
    if candidates.is_empty() {
        debug!("no previous snapshot found");
        return Ok(None);
    }

    candidates.sort();
    let path = candidates.pop().unwrap();

    let mut f = std::fs::OpenOptions::new().read(true).open(&path)?;

    let mut buf = vec![];
    let _read = f.read_to_end(&mut buf)?;
    let len = buf.len();
    if len <= 12 {
        warn!("empty/corrupt snapshot file found at path: {:?}", path);
        return Err(Error::corruption(None));
    }

    let mut len_expected_bytes = [0; 8];
    len_expected_bytes.copy_from_slice(&buf[len - 12..len - 4]);

    let mut crc_expected_bytes = [0; 4];
    crc_expected_bytes.copy_from_slice(&buf[len - 4..]);

    let _ = buf.split_off(len - 12);
    let crc_expected: u32 = arr_to_u32(&crc_expected_bytes);

    let crc_actual = crc32(&buf);

    if crc_expected != crc_actual {
        warn!(
            "corrupt snapshot file found, crc does not match expected. \
            path: {:?}",
            path
        );
        return Err(Error::corruption(None));
    }

    Snapshot::deserialize(&mut buf.as_slice()).map(Some)
}

pub(in crate::pagecache) fn write_snapshot(
    config: &RunningConfig,
    snapshot: &Snapshot,
) -> Result<()> {
    trace!("writing snapshot {:?}", snapshot);

    let bytes = snapshot.serialize();

    let crc32: [u8; 4] = u32_to_arr(crc32(&bytes));
    let len_bytes: [u8; 8] = u64_to_arr(bytes.len() as u64);

    let path_1_suffix =
        format!("snap.{:016X}.generating", snapshot.stable_lsn.unwrap_or(0));

    let mut path_1 = config.get_path();
    path_1.push(path_1_suffix);

    let path_2_suffix =
        format!("snap.{:016X}", snapshot.stable_lsn.unwrap_or(0));

    let mut path_2 = config.get_path();
    path_2.push(path_2_suffix);

    let parent = path_1.parent().unwrap();
    std::fs::create_dir_all(parent)?;
    let mut f =
        std::fs::OpenOptions::new().write(true).create(true).open(&path_1)?;

    // write the snapshot bytes, followed by a crc64 checksum at the end
    io_fail!(config, "snap write");
    f.write_all(&*bytes)?;
    io_fail!(config, "snap write len");
    f.write_all(&len_bytes)?;
    io_fail!(config, "snap write crc");
    f.write_all(&crc32)?;
    io_fail!(config, "snap write post");
    f.sync_all()?;

    trace!("wrote snapshot to {}", path_1.to_string_lossy());

    io_fail!(config, "snap write mv");
    std::fs::rename(&path_1, &path_2)?;
    io_fail!(config, "snap write dir fsync");
    maybe_fsync_directory(config.get_path())?;
    io_fail!(config, "snap write mv post");

    trace!("renamed snapshot to {}", path_2.to_string_lossy());

    // clean up any old snapshots
    let candidates = config.get_snapshot_files()?;
    for path in candidates {
        let path_str = path.file_name().unwrap().to_str().unwrap();
        if !path_2.to_string_lossy().ends_with(path_str) {
            debug!("removing old snapshot file {:?}", path);

            io_fail!(config, "snap write rm old");

            if let Err(e) = std::fs::remove_file(&path) {
                // TODO should this just be a try return?
                warn!(
                    "failed to remove old snapshot file, maybe snapshot race? {}",
                    e
                );
            }
        }
    }
    Ok(())
}
