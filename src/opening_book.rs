//! An opening book: every position reachable at a given depth, with its exact score.
//!
//! # File format
//! One file per depth, holding a flat list of little-endian `u64` entries, sorted ascending.
//! There is no header. Each entry packs a position key and its score:
//!
//! ```text
//!   bits 63..8: position key (only 49 bits are ever used)
//!   bits  7..0: score, as an i8 reinterpreted as a u8
//! ```
//!
//! Because the key occupies the high bits and keys are unique, sorting the raw entries
//! sorts them by key, which lets lookups binary search.
//!
//! Both a position and its mirror image are stored, trading file size for lookup speed.
//! Positions where the previous move won the game are never stored.

pub mod generation;

use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::board::Position;
use crate::board::key_methods::PositionKey;

const ENTRY_BYTES: usize = size_of::<u64>();
const SCORE_BITS: u32 = 8;

#[derive(Debug, Error)]
pub enum BookError {
    #[error("I/O error on opening book {path}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Opening book {path} has length {len}, which is not a multiple of {ENTRY_BYTES}")]
    BadLength { path: PathBuf, len: usize },
    #[error("Opening book {path} is not strictly sorted at entry {index}")]
    NotSorted { path: PathBuf, index: usize },
}

/// Path of the book file for a given depth (number of moves played) within a directory
pub fn book_path(book_dir: &Path, depth: u8) -> PathBuf {
    book_dir.join(format!("depth_{depth:02}.bin"))
}

fn encode_entry(key: PositionKey, score: i8) -> u64 {
    debug_assert!(key >> (u64::BITS - SCORE_BITS) == 0, "key too large to pack");
    key << SCORE_BITS | score as u8 as u64
}

fn decode_key(entry: u64) -> PositionKey {
    entry >> SCORE_BITS
}

fn decode_score(entry: u64) -> i8 {
    entry as u8 as i8
}

/// An opening book for a single depth, fully loaded into memory
pub struct OpeningBook {
    entries: Box<[u64]>,
}

impl OpeningBook {
    /// Builds a book from scored positions. Sorts the entries, but does not add mirrors.
    pub fn from_scored_keys(scored_keys: impl IntoIterator<Item = (PositionKey, i8)>) -> Self {
        let mut entries: Vec<u64> = scored_keys
            .into_iter()
            .map(|(key, score)| encode_entry(key, score))
            .collect();
        entries.sort_unstable();
        entries.dedup();
        Self {
            entries: entries.into_boxed_slice(),
        }
    }

    /// Takes entries which are already encoded and sorted
    fn from_sorted_entries(entries: Vec<u64>) -> Self {
        debug_assert!(entries.is_sorted());
        Self {
            entries: entries.into_boxed_slice(),
        }
    }

    pub fn load(path: &Path) -> Result<Self, BookError> {
        let bytes = fs::read(path).map_err(|source| BookError::Io {
            path: path.to_owned(),
            source,
        })?;

        if bytes.len() % ENTRY_BYTES != 0 {
            return Err(BookError::BadLength {
                path: path.to_owned(),
                len: bytes.len(),
            });
        }

        let entries: Box<[u64]> = bytes
            .chunks_exact(ENTRY_BYTES)
            .map(|chunk| u64::from_le_bytes(chunk.try_into().expect("chunk is 8 bytes")))
            .collect();

        // Binary search silently gives wrong answers on unsorted data, so check up front
        if let Some(index) = entries.windows(2).position(|pair| pair[0] >= pair[1]) {
            return Err(BookError::NotSorted {
                path: path.to_owned(),
                index: index + 1,
            });
        }

        Ok(Self { entries })
    }

    /// Writes to a temporary file, then renames it into place,
    /// so a partially written book is never mistaken for a finished one
    pub fn save(&self, path: &Path) -> Result<(), BookError> {
        let io_error = |source| BookError::Io {
            path: path.to_owned(),
            source,
        };

        let temporary_path = path.with_extension("bin.tmp");
        let file = File::create(&temporary_path).map_err(io_error)?;
        let mut writer = BufWriter::new(file);
        for entry in &self.entries {
            writer.write_all(&entry.to_le_bytes()).map_err(io_error)?;
        }
        let file = writer.into_inner().map_err(|e| io_error(e.into_error()))?;
        file.sync_all().map_err(io_error)?;
        fs::rename(&temporary_path, path).map_err(io_error)
    }

    /// The exact score of the position, from the current player's point of view,
    /// or `None` if the position is not in this book
    pub fn get(&self, position: Position) -> Option<i8> {
        self.get_by_key(position.get_key())
    }

    pub fn get_by_key(&self, key: PositionKey) -> Option<i8> {
        self.entries
            .binary_search_by_key(&key, |&entry| decode_key(entry))
            .ok()
            .map(|index| decode_score(self.entries[index]))
    }

    /// The number of moves played in every position of this book.
    /// `None` if the book is empty, or if its positions have different numbers of moves played.
    pub fn depth(&self) -> Option<u8> {
        let mut depths = self
            .iter()
            .map(|(key, _)| Position::from_key(key).moves_played);
        let first = depths.next()?;
        depths.all(|depth| depth == first).then_some(first)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterates over (key, score) pairs in ascending key order
    pub fn iter(&self) -> impl Iterator<Item = (PositionKey, i8)> + '_ {
        self.entries
            .iter()
            .map(|&entry| (decode_key(entry), decode_score(entry)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_round_trips_negative_and_positive_scores() {
        let key = (1 << 49) - 1;
        for score in [-21, -1, 0, 1, 21] {
            let entry = encode_entry(key, score);
            assert_eq!(decode_key(entry), key);
            assert_eq!(decode_score(entry), score);
        }
    }

    #[test]
    fn entries_sort_by_key_regardless_of_score() {
        // Lower key with a high score must still sort before a higher key with a low score
        assert!(encode_entry(5, -1) < encode_entry(6, -21));
    }

    #[test]
    fn get_finds_present_keys_and_misses_absent_ones() {
        let book = OpeningBook::from_scored_keys([(30, -3), (10, 1), (20, 0)]);
        assert_eq!(book.get_by_key(10), Some(1));
        assert_eq!(book.get_by_key(20), Some(0));
        assert_eq!(book.get_by_key(30), Some(-3));
        assert_eq!(book.get_by_key(15), None);
        assert_eq!(book.get_by_key(0), None);
        assert_eq!(book.get_by_key(31), None);
    }

    #[test]
    fn depth_is_inferred_from_keys() {
        let mut one_move = Position::new_blank_game();
        one_move.play_column(3).unwrap();
        let mut two_moves = one_move;
        two_moves.play_column(3).unwrap();

        let book = OpeningBook::from_scored_keys([(one_move.get_key(), 0)]);
        assert_eq!(book.depth(), Some(1));

        let mixed = OpeningBook::from_scored_keys([(one_move.get_key(), 0), (two_moves.get_key(), 0)]);
        assert_eq!(mixed.depth(), None);

        assert_eq!(OpeningBook::from_scored_keys([]).depth(), None);
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = std::env::temp_dir().join(format!("c4_book_test_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = book_path(&dir, 3);

        let book = OpeningBook::from_scored_keys([(30, -3), (10, 1), (20, 0)]);
        book.save(&path).unwrap();
        let loaded = OpeningBook::load(&path).unwrap();
        assert_eq!(loaded.iter().collect::<Vec<_>>(), book.iter().collect::<Vec<_>>());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn load_rejects_unsorted_files() {
        let dir = std::env::temp_dir().join(format!("c4_book_unsorted_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = book_path(&dir, 0);
        let bytes: Vec<u8> = [encode_entry(2, 0), encode_entry(1, 0)]
            .iter()
            .flat_map(|e| e.to_le_bytes())
            .collect();
        fs::write(&path, bytes).unwrap();

        assert!(matches!(OpeningBook::load(&path), Err(BookError::NotSorted { .. })));
        fs::remove_dir_all(&dir).unwrap();
    }
}
