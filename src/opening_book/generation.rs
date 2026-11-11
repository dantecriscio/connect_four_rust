//! Building the book for one depth from the book of the previous depth

use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;

use crate::board::key_methods::PositionKey;
use crate::board::{EndState, GAME_WIDTH, GameState, Position};
use crate::live_evaluation::Solver;
use crate::opening_book::{OpeningBook, encode_entry};

/// The smaller of a position's key and its mirror's key.
/// A position and its mirror always share a score, so only this one needs solving.
fn canonical_key(position: Position) -> PositionKey {
    position.get_key().min(position.get_mirror_key())
}

/// Every position one move after `parent`, excluding positions where that move won the game
fn children(parent: Position) -> impl Iterator<Item = Position> {
    (0..GAME_WIDTH).filter_map(move |column| {
        let mut child = parent;
        child.play_column(column).ok()?;
        match child.get_game_state() {
            GameState::Ended(EndState::PriorPlayerWon) => None,
            _ => Some(child),
        }
    })
}

/// All distinct positions one move after the positions in `previous_book`, one per mirror pair.
/// Returned in ascending canonical key order.
pub fn positions_after(previous_book: &OpeningBook) -> Vec<Position> {
    let mut canonical_keys: Vec<PositionKey> = previous_book
        .entries
        .par_iter()
        .map(|&entry| Position::from_key(super::decode_key(entry)))
        // The previous book holds both mirrors, but the canonical one alone reaches every child
        .filter(|&parent| parent.get_key() == canonical_key(parent))
        .flat_map_iter(|parent| children(parent).map(canonical_key))
        .collect();

    canonical_keys.par_sort_unstable();
    canonical_keys.dedup();

    canonical_keys.into_par_iter().map(Position::from_key).collect()
}

/// Solves every position in parallel, returning a book that contains each position and its mirror.
/// `on_progress(solved_so_far)` is called after each position is solved, from worker threads.
pub fn solve_positions(
    solver: &Solver,
    positions: &[Position],
    on_progress: impl Fn(usize) + Sync,
) -> OpeningBook {
    let solved = AtomicUsize::new(0);

    let mut entries: Vec<u64> = positions
        .par_iter()
        .flat_map_iter(|&position| {
            let score = solver.solve(position);
            on_progress(solved.fetch_add(1, Ordering::Relaxed) + 1);

            let key = position.get_key();
            let mirror_key = position.get_mirror_key();
            let mirror_entry = (mirror_key != key).then(|| encode_entry(mirror_key, score));
            std::iter::once(encode_entry(key, score)).chain(mirror_entry)
        })
        .collect();

    entries.par_sort_unstable();
    OpeningBook::from_sorted_entries(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book_with_scores_of_zero(positions: &[Position]) -> OpeningBook {
        OpeningBook::from_scored_keys(positions.iter().flat_map(|p| {
            [(p.get_key(), 0), (p.get_mirror_key(), 0)]
        }))
    }

    #[test]
    fn depth_one_has_four_positions_up_to_mirroring() {
        let blank = Position::new_blank_game();
        let book = book_with_scores_of_zero(&[blank]);
        assert_eq!(positions_after(&book).len(), 4);
    }

    #[test]
    fn position_counts_match_known_values() {
        // Counts of distinct positions (including mirrors) after n moves: OEIS A212693
        let known_counts = [1, 7, 49, 238, 1120, 4263, 16422];

        let mut positions = vec![Position::new_blank_game()];
        for depth in 0..known_counts.len() - 1 {
            let book = book_with_scores_of_zero(&positions);
            assert_eq!(book.len(), known_counts[depth], "depth {depth}");
            positions = positions_after(&book);
        }
    }

    #[test]
    fn winning_moves_are_excluded() {
        // Player one has three stacked in column 0 and can win by playing it again
        let mut parent = Position::new_blank_game();
        for column in [0, 1, 0, 1, 0, 1] {
            parent.play_column(column).unwrap();
        }
        let child_count = children(parent).count();
        assert_eq!(child_count, GAME_WIDTH as usize - 1);
    }
}
