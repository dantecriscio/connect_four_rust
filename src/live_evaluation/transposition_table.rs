use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::usize;

use crate::board::bitmap::{Bitmap, BITMAP_COLUMN_HEIGHT, BITMAP_ROW_WIDTH};
use crate::board::key_methods::PositionKey;
use crate::live_evaluation::mtd::{MAXIMUM_SCORE, NEGATED_MINIMUM_SCORE};

// Must be called with n > 2
const fn is_prime(n: u32) -> bool {
    let mut divisor = 2;
    while divisor * divisor <= n {
        if n % divisor == 0 {
            return false;
        }
        divisor += 1;
    }
    true
}

// The Table Size should be a prime number to ensure a uniform usage of the table
const MINIMUM_LOG_OF_TABLE_ENTRIES: u8 = 23;
const TABLE_SIZE_U32: u32 = {
    let mut table_size = 1 << MINIMUM_LOG_OF_TABLE_ENTRIES;
    while !is_prime(table_size) {
        table_size += 1;
    }
    table_size
};

// Needed because this file casts u32 to usize
const _: () = {
    // size_of returns bytes. 4 bytes = 32 bits.
    if core::mem::size_of::<usize>() < 4 {
        panic!("Compilation failed: This project requires a 32-bit or 64-bit target architecture.");
    }
};
const TABLE_SIZE: usize = TABLE_SIZE_U32 as usize;


struct TableEntry(AtomicU64);
impl TableEntry {
    fn new_blank() -> Self {
        Self(AtomicU64::new(0))
    }

    fn store(&self, key: Bitmap, val: u8, ordering: Ordering) {
        self.0.store((key << 8) | (val as u64), ordering);
    }

    fn load(&self, ordering: Ordering) -> (Bitmap, u8) {
        let entry = self.0.load(ordering);
        (entry >> 8, entry as u8)
    }
}


type TruncatedKey = u32;
const _: () = {
    let full_key_bits = BITMAP_ROW_WIDTH * BITMAP_COLUMN_HEIGHT;
    assert!(
        MINIMUM_LOG_OF_TABLE_ENTRIES + 32 >= full_key_bits,
        "There is a possibility of collisions in hash table"
    );
};

const STORED_UPPER_BOUND_OFFSET: u8 = 1 + NEGATED_MINIMUM_SCORE;
const MAX_STORED_UPPER_BOUND: u8 = MAXIMUM_SCORE + STORED_UPPER_BOUND_OFFSET;
const STORED_LOWER_BOUND_OFFSET: u8 = MAX_STORED_UPPER_BOUND + 1 + NEGATED_MINIMUM_SCORE;
const MAX_STORED_LOWER_BOUND: u8 = MAXIMUM_SCORE + STORED_LOWER_BOUND_OFFSET;

#[derive(Clone, Copy)]
pub struct TableKey {
    table_index: u32,
    full_position_key: Bitmap,
}

impl From<PositionKey> for TableKey {
    fn from(position_key: PositionKey) -> Self {
        Self {
            // This is safe casting
            // IMO, a u64 modulo u32 should just return a u32
            // Because modulo operations can never return anything greater than the modulus
            table_index: (position_key % TABLE_SIZE_U32 as u64) as u32,
            full_position_key: position_key,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TableResult {
    Miss,
    LowerBound(i8),
    UpperBound(i8),
}

pub struct TranspositionTable(Box<[TableEntry]>);

impl TranspositionTable {
    pub fn new() -> Self {
        Self((0..TABLE_SIZE)
            .map(|_| TableEntry::new_blank())
            .collect::<Vec<TableEntry>>()
            .into_boxed_slice())
    }

    pub fn put_upper_bound(&self, table_key: TableKey, upper_bound: i8) {
        let saved_value = (upper_bound + STORED_UPPER_BOUND_OFFSET as i8) as u8;
        self.0[table_key.table_index as usize].store(table_key.full_position_key, saved_value, Ordering::Relaxed);
    }

    pub fn put_lower_bound(&self, table_key: TableKey, lower_bound: i8) {
        let saved_value = (lower_bound + STORED_LOWER_BOUND_OFFSET as i8) as u8;
        self.0[table_key.table_index as usize].store(table_key.full_position_key, saved_value, Ordering::Relaxed);
    }

    pub fn get(&self, table_key: TableKey) -> TableResult {
        let (saved_key, saved_value) = self.0[table_key.table_index as usize].load(Ordering::Relaxed);
        if saved_key != table_key.full_position_key {
            return TableResult::Miss;
        }
        match saved_value {
            0 => TableResult::Miss,
            1..=MAX_STORED_UPPER_BOUND => {
                TableResult::UpperBound(saved_value as i8 - STORED_UPPER_BOUND_OFFSET as i8)
            }
            _ => {
                debug_assert!(saved_value <= MAX_STORED_LOWER_BOUND);
                TableResult::LowerBound(saved_value as i8 - STORED_LOWER_BOUND_OFFSET as i8)
            }
        }
    }

    pub fn reset(&self) {
        for entry in &self.0 {
            entry.store(0, 0, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_is_initially_empty() {
        let table = TranspositionTable::new();
        let dummy_table_key = TableKey::from(0);
        assert_eq!(TableResult::Miss, table.get(dummy_table_key));
    }

    #[test]
    fn test_put_and_get_lower_bound() {
        let table = TranspositionTable::new();
        let dummy_table_key = TableKey::from(0);
        let dummy_bound = 1;

        table.put_lower_bound(dummy_table_key, dummy_bound);
        assert_eq!(
            TableResult::LowerBound(dummy_bound),
            table.get(dummy_table_key)
        );
    }

    #[test]
    fn test_put_and_get_upper_bound() {
        let table = TranspositionTable::new();
        let dummy_table_key = TableKey::from(0);
        let dummy_bound = 1;

        table.put_upper_bound(dummy_table_key, dummy_bound);
        assert_eq!(
            TableResult::UpperBound(dummy_bound),
            table.get(dummy_table_key)
        );
    }

    #[test]
    fn test_collision_picks_later() {
        let table = TranspositionTable::new();
        let dummy_table_key = TableKey::from(0);
        let dummy_bound = 1;

        table.put_lower_bound(dummy_table_key, dummy_bound);
        table.put_upper_bound(dummy_table_key, dummy_bound);
        assert_eq!(
            TableResult::UpperBound(dummy_bound),
            table.get(dummy_table_key)
        );

        table.put_lower_bound(dummy_table_key, dummy_bound);
        assert_eq!(
            TableResult::LowerBound(dummy_bound),
            table.get(dummy_table_key)
        );
    }

    #[test]
    fn test_reset_causes_miss() {
        let table = TranspositionTable::new();
        let dummy_table_key = TableKey::from(0);
        let dummy_bound = 1;

        table.put_lower_bound(dummy_table_key, dummy_bound);
        assert_eq!(
            TableResult::LowerBound(dummy_bound),
            table.get(dummy_table_key)
        );

        table.reset();
        assert_eq!(TableResult::Miss, table.get(dummy_table_key));
    }
}
