
pub struct ColumnScanIter {
    tree: Tree,
    low_key: IVec,
    current_node: Option<Arc<crate::node::Inner>>,
    current_index: usize,
}

impl Iterator for ColumnScanIter {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(inner) = &self.current_node {
                if inner.is_columnar {
                    // Columnar Fast Path
                    // Note: This relies on Inner having public fields or accessors
                    // inner.children is u32.
                    if self.current_index < inner.children() as usize {
                        // Manual offset calc: f_start + idx * 4
                        // Accessing inner buffer safely needs a method on Inner?
                        // Inner.iter_column_f() is available but complex to partially iterate.
                        // I added iter_column_f which returns an iterator.
                        // But I can't store that iterator easily.
                        // I will use `inner.iter_column_f().nth(current_index)`?
                        // That's O(N) per step! Bad.
                        
                        // I should reuse the logic I added to Inner in a way that allows random access?
                        // Or just duplicate the pointer math here if fields are pub(crate).
                        // Inner::ptr is private-ish.
                        // Inner has `iter_column_f`.
                        // Maybe I should store the `iter_column_f` iterator in the struct?
                        // But it borrows `Inner`. Self-referential.
                        
                        // Let's use `iter_column_f` collectively?
                        // If `next()` returns one item, I can't.
                        
                        // Compromise:
                        // I will add `get_f_at(index)` to Inner.
                        // It's stateless and fast (O(1)).
                        
                        // Waiting... I can't modify node.rs again easily without context switch.
                        // Let's see if I can direct access.
                        // `inner.ptr` is not pub.
                        // `iter_column_f` is the only window.
                        
                        // What if `ColumnScanIter` does NOT assume per-item iteration?
                        // What if `scan_column_f` returns an iterator of *Chunks* (floats)?
                        // No, user wants `sum(f)`.
                        
                        // Wait, I modified `node.rs` to expose `iter_column_f`.
                        // Can I just add `pub(crate) fn get_column_f(&self, idx: usize) -> Option<f32>` to `Inner`?
                        // Yes, that would be cleanest.
                        
                        // Let's modify `src/node.rs` one more time to add `get_column_f`.
                        // Then `ColumnScanIter` becomes trivial.
                        
                        return None; // Placeholder
                    }
                } else {
                     // Row Fallback...
                }
                
                // Move to next node
                let next = inner.hi();
                 if let Some(h) = next {
                    self.low_key = IVec::from(h);
                    self.current_node = None;
                    self.current_index = 0;
                    continue;
                } else {
                    return None;
                }
            }
            
            // Fetch next
            let guard = pin();
            if let Ok(view) = self.tree.view_for_key(&self.low_key, &guard) {
                 self.current_node = Some(view.node.inner.clone());
                 self.current_index = 0;
            } else {
                 return None;
            }
        }
    }
}
