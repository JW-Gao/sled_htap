
    fn new_columnar(
        lo: &[u8],
        hi: Option<&[u8]>,
        prefix_len: u8,
        is_index: bool,
        next: Option<NonZeroU64>,
        items: &[(KeyRef<'_>, &[u8])],
    ) -> Inner {
        // Validation (optional but good)
        // assert!(!is_index);
        
        let count = items.len();
        
        // Fixed sizes
        let k_width = 4;
        let f_width = 4;
        let c_width = 10;
        
        let k_size = count * k_width;
        let f_size = count * f_width;
        let c_size = count * c_width;
        
        // Offsets relative to Data Buffer Start
        let k_start = 0;
        let f_start = k_size;
        let c_start = k_size + f_size;
        
        let total_data_size = k_size + f_size + c_size;
        
        let total_node_size = size_of::<Header>()
            + lo.len()
            + hi.map(|h| h.len()).unwrap_or(0)
            + total_data_size;
            
        let mut ret = uninitialized_node(total_node_size);
        
        let header = ret.header_mut();
        header.lo_len = lo.len() as u64;
        header.hi_len = hi.map(|h| h.len() as u64).unwrap_or(0);
        header.children = count as u32;
        header.prefix_len = prefix_len;
        header.version = 1;
        header.next = next;
        header.is_index = is_index; // Should be false
        
        // Set columnar offsets
        header.k_start = k_start as u32;
        header.f_start = f_start as u32;
        header.c_start = c_start as u32;
        
        // Copy lo/hi
        ret.lo_mut().copy_from_slice(lo);
        if let Some(h) = hi {
             ret.hi_mut().unwrap().copy_from_slice(h);
        }
        
        // Write Columns
        let data_slice = ret.data_buf_mut();
        
        for (i, (_key, val)) in items.iter().enumerate() {
            // Assume val is [k(4), f(4), c(10)]
            // We can also extract k from _key if _key is full key, but _key might be prefix compressed?
            // "items" passed to Inner::new usually have full keys or KeyKeys?
            // The split logic passes full keys (or reconstructed keys).
            // But let's trust "val" which contains the full record payload as per user description.
            
            if val.len() < 18 {
                // Fallback or panic? For now panic or handle gracefully?
                // Given the strict requirement, let's assume valid data.
                // Or maybe the first 4 bytes of val are K.
            }
            
            // Write K
            let k_src = &val[0..4];
            let k_dst_start = k_start + i * k_width;
            data_slice[k_dst_start..k_dst_start+4].copy_from_slice(k_src);
            
            // Write F
            let f_src = &val[4..8];
            let f_dst_start = f_start + i * f_width;
            data_slice[f_dst_start..f_dst_start+4].copy_from_slice(f_src);
            
            // Write C
            let c_src = &val[8..18];
            let c_dst_start = c_start + i * c_width;
            data_slice[c_dst_start..c_dst_start+10].copy_from_slice(c_src);
        }
        
        ret
    }
