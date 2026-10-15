use std::{mem::MaybeUninit, usize};

use crate::board::{GAME_WIDTH, Position};

// Odd number example [3, 2, 4, 1, 5, 0, 6]
// Even number example [4, 3, 5, 2, 6, 1, 7, 0]
pub(crate) const INSIDE_OUT_COLUMN_ORDER: [u8; GAME_WIDTH as usize] = {
    let mut default_column_order = [0; GAME_WIDTH as usize];
    let middle_column = GAME_WIDTH / 2;
    let mut offset_from_middle = 0;

    let mut i = 0;
    while i < GAME_WIDTH {
        if i % 2 == 1 {
            offset_from_middle += 1;
        }

        let column = if i % 2 == 0 {
            middle_column + offset_from_middle
        } else {
            middle_column - offset_from_middle
        };
        default_column_order[i as usize] = column;

        i += 1;
    }
    default_column_order
};

// Odd number example [6, 0, 5, 1, 4, 2, 3]
// Even number example [0, 7, 1, 6, 2, 5, 3, 4]

// Meant to be iterated over when adding to the SortedChildPositionIterator
// Iterate over this, because SortedChildPositionIterator behaves like a stack in the case of ties
// Therefore, if everything is tied, the column order will be INSIDE_OUT_COLUMN_ORDER
pub(crate) const OUTSIDE_IN_COLUMN_ORDER: [u8; GAME_WIDTH as usize] = {
    let mut reversed_default_column_order = [0; GAME_WIDTH as usize];
    let mut i = 0;
    while i < GAME_WIDTH as usize {
        reversed_default_column_order[GAME_WIDTH as usize - i - 1] = INSIDE_OUT_COLUMN_ORDER[i];
        i += 1;
    }
    reversed_default_column_order
};

#[derive(Clone, Copy)]
struct ScoredEntry {
    pub child_position: Position,
    pub score: u8,
}

pub(crate) struct ScoredPositionIterator {
    entries: [MaybeUninit<ScoredEntry>; GAME_WIDTH as usize],
    start_inclusive: u8,
    end_exclusive: u8,
}

impl ScoredPositionIterator {
    pub(crate) fn new() -> Self {
        Self {
            entries: [MaybeUninit::uninit(); GAME_WIDTH as usize],
            start_inclusive: 0,
            end_exclusive: 0,
        }
    }

    pub(crate) fn add(&mut self, child_position: Position, score: u8) {
        // The index we are looking at (initially, the first empty index)
        let mut vacant_index = self.end_exclusive as usize;

        // As long as the previous entry is strictly bigger, shift that right once
        while vacant_index > self.start_inclusive as usize
            && unsafe { self.entries[vacant_index - 1].assume_init().score } > score
        {
            self.entries[vacant_index] = self.entries[vacant_index - 1];
            vacant_index -= 1;
        }

        // Place our entry at the now empty spot
        self.entries[vacant_index] = MaybeUninit::new(ScoredEntry {
            child_position,
            score,
        });
        self.end_exclusive += 1;
    }
}

impl Iterator for ScoredPositionIterator {
    type Item = Position;

    /// Gets the next best child
    fn next(&mut self) -> Option<Self::Item> {
        if self.end_exclusive == self.start_inclusive {
            None
        } else {
            self.end_exclusive -= 1;
            Some(unsafe { self.entries[self.end_exclusive as usize].assume_init() }.child_position)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let exact_size = (self.end_exclusive - self.start_inclusive) as usize;
        (exact_size, Some(exact_size))
    }
}

impl DoubleEndedIterator for ScoredPositionIterator {
    /// Gets the next worst child
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start_inclusive == self.end_exclusive {
            None
        } else {
            let next = Some(
                unsafe { self.entries[self.start_inclusive as usize].assume_init() }.child_position,
            );
            self.start_inclusive += 1;
            return next;
        }
    }
}

impl ExactSizeIterator for ScoredPositionIterator {
    // len() automatically grabs the lower bound from size_hint(), which is the exact size
}

#[cfg(test)]
mod tests {
    use crate::board::testing_constants::BLANK_POSITION;

    use super::*;

    fn get_single_square_positions() -> Vec<Position> {
        (0..GAME_WIDTH)
            .map(|col| {
                let mut single_square_position = BLANK_POSITION;
                single_square_position.play_column(col);
                single_square_position
            })
            .collect()
    }

    // Tests that the iterator returns results then repeatedly returns None
    #[test]
    fn test_sorter_with_zero_elements() {
        let mut scored_position_iterator = ScoredPositionIterator::new();
        assert_eq!(None, scored_position_iterator.next());
        assert_eq!(None, scored_position_iterator.next());
    }

    #[test]
    fn test_sorter_with_one_elements() {
        let input_positions = get_single_square_positions();
        let mut scored_position_iterator = ScoredPositionIterator::new();

        scored_position_iterator.add(input_positions[0], 0);

        assert_eq!(Some(input_positions[0]), scored_position_iterator.next());
        assert_eq!(None, scored_position_iterator.next());
        assert_eq!(None, scored_position_iterator.next());
    }

    #[test]
    fn test_sorter_with_two_elements() {
        let input_positions = get_single_square_positions();
        let mut scored_position_iterator = ScoredPositionIterator::new();

        scored_position_iterator.add(input_positions[0], 0);
        scored_position_iterator.add(input_positions[1], 0);

        assert_eq!(Some(input_positions[1]), scored_position_iterator.next());
        assert_eq!(Some(input_positions[0]), scored_position_iterator.next());
        assert_eq!(None, scored_position_iterator.next());
        assert_eq!(None, scored_position_iterator.next());
    }

    // Tests the fact that if we put in all ties, it will function as a stack
    #[test]
    fn test_stable_sort() {
        let input_positions = get_single_square_positions();
        let mut scored_position_iterator = ScoredPositionIterator::new();

        for &position in &input_positions {
            scored_position_iterator.add(position, 0);
        }

        let mut iterator_result: Vec<Position> = scored_position_iterator.collect();
        iterator_result.reverse();
        assert_eq!(input_positions, iterator_result)
    }

    #[test]
    fn test_normal_sort() {
        let input_positions = get_single_square_positions();
        let mut scored_position_iterator = ScoredPositionIterator::new();

        let scores = [3, 2, 6, 7, 5, 4, 1];
        let expected_order = [3, 2, 4, 5, 0, 1, 6];

        let mut expected_output_positions = Vec::new();

        for i in 0..scores.len() {
            expected_output_positions.push(input_positions[expected_order[i]]);
            scored_position_iterator.add(input_positions[i], scores[i]);
        }

        let actual_positions: Vec<Position> = scored_position_iterator.collect();
        assert_eq!(expected_output_positions, actual_positions);
    }

    // Some ties, some different scores
    // Expect to sort as much as possible, then treat the ties like a stack
    #[test]
    fn test_hybrid_sort() {
        let input_positions = get_single_square_positions();
        let mut scored_position_iterator = ScoredPositionIterator::new();

        let scores = [3, 3, 6, 7, 5, 5, 1];
        let expected_order = [3, 2, 5, 4, 1, 0, 6];

        let mut expected_output_positions = Vec::new();

        for i in 0..scores.len() {
            expected_output_positions.push(input_positions[expected_order[i]]);
            scored_position_iterator.add(input_positions[i], scores[i]);
        }

        let actual_positions: Vec<Position> = scored_position_iterator.collect();
        assert_eq!(expected_output_positions, actual_positions);
    }

    // Tests the DoubleEndedIterator functionality - next_back returns worst scores first
    #[test]
    fn test_next_back() {
        let input_positions = get_single_square_positions();
        let mut scored_position_iterator = ScoredPositionIterator::new();

        let scores = [3, 2, 6, 7, 5, 4, 1];
        // Sorted descending: 7, 6, 5, 4, 3, 2, 1
        // next_back() should return: 1, 2, 3, 4, 5, 6, 7 (worst to best)

        for i in 0..scores.len() {
            scored_position_iterator.add(input_positions[i], scores[i]);
        }

        assert_eq!(
            Some(input_positions[6]),
            scored_position_iterator.next_back()
        ); // score 1
        assert_eq!(
            Some(input_positions[1]),
            scored_position_iterator.next_back()
        ); // score 2
        assert_eq!(
            Some(input_positions[0]),
            scored_position_iterator.next_back()
        ); // score 3
        assert_eq!(
            Some(input_positions[5]),
            scored_position_iterator.next_back()
        ); // score 4
        assert_eq!(
            Some(input_positions[4]),
            scored_position_iterator.next_back()
        ); // score 5
        assert_eq!(
            Some(input_positions[2]),
            scored_position_iterator.next_back()
        ); // score 6
        assert_eq!(
            Some(input_positions[3]),
            scored_position_iterator.next_back()
        ); // score 7
        assert_eq!(None, scored_position_iterator.next_back());
    }

    // Tests ExactSizeIterator::len() functionality
    #[test]
    fn test_len() {
        let input_positions = get_single_square_positions();
        let mut scored_position_iterator = ScoredPositionIterator::new();

        assert_eq!(0, scored_position_iterator.len());

        scored_position_iterator.add(input_positions[0], 5);
        assert_eq!(1, scored_position_iterator.len());

        scored_position_iterator.add(input_positions[1], 3);
        assert_eq!(2, scored_position_iterator.len());

        scored_position_iterator.add(input_positions[2], 7);
        assert_eq!(3, scored_position_iterator.len());

        // Consume one from front
        scored_position_iterator.next_back();
        assert_eq!(2, scored_position_iterator.len());

        // Consume one from back
        scored_position_iterator.next();
        assert_eq!(1, scored_position_iterator.len());

        scored_position_iterator.next();
        assert_eq!(0, scored_position_iterator.len());
    }

    // Tests size_hint returns exact bounds
    #[test]
    fn test_size_hint() {
        let input_positions = get_single_square_positions();
        let mut scored_position_iterator = ScoredPositionIterator::new();

        let (lower, upper) = scored_position_iterator.size_hint();
        assert_eq!((0, Some(0)), (lower, upper));

        scored_position_iterator.add(input_positions[0], 5);
        let (lower, upper) = scored_position_iterator.size_hint();
        assert_eq!((1, Some(1)), (lower, upper));

        scored_position_iterator.add(input_positions[1], 3);
        let (lower, upper) = scored_position_iterator.size_hint();
        assert_eq!((2, Some(2)), (lower, upper));

        // After consuming from back
        scored_position_iterator.next();
        let (lower, upper) = scored_position_iterator.size_hint();
        assert_eq!((1, Some(1)), (lower, upper));

        // After consuming from front
        scored_position_iterator.next_back();
        let (lower, upper) = scored_position_iterator.size_hint();
        assert_eq!((0, Some(0)), (lower, upper));
    }

    // Tests mixed next() and next_back() iteration
    #[test]
    fn test_mixed_iteration() {
        let input_positions = get_single_square_positions();
        let mut scored_position_iterator = ScoredPositionIterator::new();

        let scores = [3, 2, 6, 7, 5, 4, 1];
        // Sorted descending: 7(idx3), 6(idx2), 5(idx4), 4(idx5), 3(idx0), 2(idx1), 1(idx6)

        for i in 0..scores.len() {
            scored_position_iterator.add(input_positions[i], scores[i]);
        }

        // Alternate: best, worst, best, worst...
        assert_eq!(Some(input_positions[3]), scored_position_iterator.next()); // best: 7
        assert_eq!(
            Some(input_positions[6]),
            scored_position_iterator.next_back()
        ); // worst: 1
        assert_eq!(Some(input_positions[2]), scored_position_iterator.next()); // best: 6
        assert_eq!(
            Some(input_positions[1]),
            scored_position_iterator.next_back()
        ); // worst: 2
        assert_eq!(Some(input_positions[4]), scored_position_iterator.next()); // best: 5
        assert_eq!(
            Some(input_positions[0]),
            scored_position_iterator.next_back()
        ); // worst: 3
        assert_eq!(Some(input_positions[5]), scored_position_iterator.next()); // last: 4
        assert_eq!(None, scored_position_iterator.next());
        assert_eq!(None, scored_position_iterator.next_back());
    }

    // Tests that ties are handled correctly with next_back (stack behavior for ties)
    #[test]
    fn test_next_back_with_ties() {
        let input_positions = get_single_square_positions();
        let mut scored_position_iterator = ScoredPositionIterator::new();

        // All same score - should act as stack for both next() and next_back()
        // Added in order: 0, 1, 2, 3, 4, 5, 6
        // next() returns: 6, 5, 4, 3, 2, 1, 0 (LIFO - stack)
        // next_back() returns: 0, 1, 2, 3, 4, 5, 6 (FIFO - queue from front)

        for i in 0..GAME_WIDTH as usize {
            scored_position_iterator.add(input_positions[i], 5);
        }

        // Test next_back returns in insertion order (FIFO from front)
        for i in 0..GAME_WIDTH as usize {
            assert_eq!(
                Some(input_positions[i]),
                scored_position_iterator.next_back()
            );
        }
        assert_eq!(None, scored_position_iterator.next_back());
    }
}
