#!/usr/bin/env bash
# Builds the opening book for depths 0..=MAX_DEPTH into unzipped_opening_books/, one book-maker call per depth.
# Depths whose book already exists are skipped, so an interrupted run resumes where it stopped.
# (`cargo build` first unzips any books checked in via openingBook.zip, so those are skipped too.)
#
# Usage: scripts/build_book.sh <max_depth> [timings_csv]
#   timings_csv: if given, appends `depth,total_runtime_seconds,cores_used,positions` per depth,
#                writing the header first if the file does not exist yet.
# Set RAYON_NUM_THREADS to limit the number of cores used.
set -euo pipefail

MAX_DEPTH=${1:?Usage: $0 <max_depth> [timings_csv]}
CSV=${2:-}
REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BOOK_DIR="$REPO_DIR/unzipped_opening_books"
CORES=${RAYON_NUM_THREADS:-$(getconf _NPROCESSORS_ONLN)}

now() { perl -MTime::HiRes=time -e 'printf "%.3f", time'; }
book() { printf '%s/depth_%02d.bin' "$BOOK_DIR" "$1"; }

if [[ -n "$CSV" && ! -e "$CSV" ]]; then
    echo "depth,total_runtime_seconds,cores_used,positions" > "$CSV"
fi

cargo build --release --manifest-path "$REPO_DIR/Cargo.toml" --bin book-maker
BOOK_MAKER="$REPO_DIR/target/release/book-maker"
mkdir -p "$BOOK_DIR"

for depth in $(seq 0 "$MAX_DEPTH"); do
    output=$(book "$depth")
    if [[ -e "$output" ]]; then
        echo "Depth $depth: $output already exists, skipping"
        continue
    fi

    prior_args=()
    if (( depth > 0 )); then
        prior_args=(--prior "$(book $((depth - 1)))")
    fi

    start=$(now)
    "$BOOK_MAKER" "$output" "${prior_args[@]+"${prior_args[@]}"}"
    end=$(now)

    if [[ -n "$CSV" ]]; then
        # Each entry is 8 bytes; this counts every position stored, mirrors included
        positions=$(( $(wc -c < "$output") / 8 ))
        runtime=$(perl -e "printf '%.1f', $end - $start")
        echo "$depth,$runtime,$CORES,$positions" >> "$CSV"
    fi
done
