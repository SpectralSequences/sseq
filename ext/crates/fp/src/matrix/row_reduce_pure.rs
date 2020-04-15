pub fn row_reduce_pure(&mut self) {
    // println!("rows : {}, cols : {}", self.rows(), self.columns());
    // println!("{}\n\n\n", self);
    let mut column_to_pivot_row = self.take_pivots();
    let p = self.p;
    let rows = self.rows();
    for x in column_to_pivot_row.iter_mut() {
        *x = -1;
    }
    if rows == 0 {
        self.set_pivots(column_to_pivot_row);
        return; 
    }
    let mask = vector::bitmask(p);
    let bit_length = vector::bit_length(p);
    let entries_per_64_bits = vector::entries_per_64_bits(p);
    let usable_bits_per_limb = bit_length * entries_per_64_bits;
    let min_limb = self[0].min_limb();
    let max_limb = self[0].max_limb();
    let mut cur_limb_cache = vec![0; self.rows()];
    let mut next_limb_cache = vec![0; self.rows()];
    for i in 0..rows {
        cur_limb_cache[i] = self[i].limbs()[min_limb];
    }
    let mut pivot : usize = 0;
    let mut pivot_column = 0;
    let mut cached_next_limb;
    'outer : for limb_idx in min_limb .. max_limb {
        cached_next_limb = !(limb_idx + 1 < max_limb);
        let next_limb = if limb_idx + 1 < max_limb { limb_idx + 1 } else { limb_idx };
        for bit in (0..usable_bits_per_limb).step_by(bit_length) {
            if pivot_column >= self.columns() {
                break 'outer;
            }
            // Search down column for a nonzero entry.
            let mut pivot_row = rows;
            for i in pivot..rows {
                if !cached_next_limb {
                    next_limb_cache[i] = self[i].limbs()[next_limb];
                }
                // 20% of L1 cache misses happen here.
                // println!("bit : {}, pivot_column : {}", bit, pivot_column);
                // println!("self[i].limbs()[limb_idx] : {}", self[i].limbs()[limb_idx]);
                // println!("cur_limb_cache[i] : {}", cur_limb_cache[i]);
                let entry = ((cur_limb_cache[i] >> bit) & mask) as u32;
                debug_assert_eq!(entry, self[i].entry(pivot_column));
                if entry != 0 { // 
                    pivot_row = i;
                    break;
                }
            }
            if pivot_row == rows {
                // No pivot in this column.
                // Ensure that we cache next column
                if !cached_next_limb && bit + bit_length >= usable_bits_per_limb {
                    // If a whole limb has no pivots, better just read whole next column now.
                    for i in 0..rows {
                        next_limb_cache[i] = self[i].limbs()[limb_idx + 1];
                    }
                }
                pivot_column += 1;
                continue;
            }

            // Record position of pivot.
            column_to_pivot_row[pivot_column] = pivot as isize;

            // Pivot_row contains a row with a pivot in current column.
            // Swap pivot row up.
            self.swap_rows(pivot, pivot_row);
            cur_limb_cache.swap(pivot, pivot_row);
            next_limb_cache.swap(pivot, pivot_row);
            // println!("({}) <==> ({}): \n{}", pivot, pivot_row, self);
            // println!("({}) <==> ({})", pivot, pivot_row);

            // // Divide pivot row by pivot entry
            let c = ((cur_limb_cache[pivot] >> bit) & mask) as u32;// self[pivot].entry(pivot_column);
            debug_assert_eq!(c, self[pivot].entry(pivot_column));
            let c_inv = prime::inverse(p, c);
            self[pivot].scale(c_inv);
            cur_limb_cache[pivot] = self[pivot].reduce_limb(cur_limb_cache[pivot] * c_inv as u64);
            next_limb_cache[pivot] = self[pivot].reduce_limb(next_limb_cache[pivot] * c_inv as u64);
            // println!("({}) <== {} * ({}): \n{}", pivot, c_inv, pivot, self);
            // println!("({}) <== {} * ({})", pivot, c_inv, pivot);
            // We would say:
            // for i in 0..rows { // but we want to skip a few rows so we can't use for.
            let mut i = 0;
            while i < rows {
                if i as usize == pivot {
                    // Between pivot and pivot_row, we already checked that the pivot column is 0, 
                    // so we skip ahead a bit.
                    i = pivot_row + 1;
                    // We already cached these entries when searching for the pivot
                    continue;
                }

                // if bit == 0 && 
                if !cached_next_limb { // Condition slows us down due to branch mispredict
                    // Everything else needs to be cached now
                    next_limb_cache[i] = self[i].limbs()[next_limb];
                }

                // 50% of L1 cache misses happen here
                let pivot_column_entry = ((cur_limb_cache[i] >> bit) & mask) as u32; //self[i].entry(pivot_column);
                debug_assert!(pivot_column_entry == self[i].entry(pivot_column), 
                    format!("\n\ni : {}, pivot_column : {}\n\n", i, pivot_column)
                );
                if pivot_column_entry == 0 {
                    i += 1; // loop control structure.
                    continue;
                }
                let row_op_coeff = *p - pivot_column_entry;
                if row_op_coeff == 0 {
                    continue;
                }
                // 12% of L1 cache misses here (90% of L1 write misses.)
                // self.row_op(i, pivot, row_op_coeff);
                // println!("({}) <== ({}) + {} * ({}): \n{}", i, i, c_inv, pivot, self);
                // println!("({}) <== ({}) + {} * ({})", i, i, c_inv, pivot);
                let mut target_limbs = self[i].take_limbs();
                for l in min_limb .. max_limb {
                    target_limbs[l] = self[i].reduce_limb(self[i].add_limb(target_limbs[l], self[pivot].limbs()[l], row_op_coeff));
                }
                cur_limb_cache[i] = self[i].reduce_limb(self[i].add_limb(cur_limb_cache[i], cur_limb_cache[pivot], row_op_coeff));
                // next_limb_cache[i] = self[i].limbs()[limb_idx + 1];
                next_limb_cache[i] = self[i].reduce_limb(self[i].add_limb(next_limb_cache[i], next_limb_cache[pivot], row_op_coeff));
                self[i].put_limbs(target_limbs);
                i += 1; // loop control structure.
            }
            cached_next_limb = true;
            pivot += 1;
            pivot_column += 1;
        }
        std::mem::swap(&mut next_limb_cache, &mut cur_limb_cache);
    }
    self.set_pivots(column_to_pivot_row);
}