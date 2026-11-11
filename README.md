# connect_four_rust

A perfect Connect Four solver in Rust, with an opening book and a web frontend.
It is a rewrite of the C++ solver from Pascal Pons' [tutorial](http://blog.gamesolver.org/).

## Running the web server

```sh
cargo server            # or: cargo run --release
```

Then open <http://localhost:8080>.

The first build unzips the checked-in opening books (`openingBook.zip`) into
`unzipped_opening_books/`, and the server finds both those books and the frontend in `web/`
automatically. Pass options after `--`, e.g. `cargo server -- --port 9000`:

| Option | Default |
|---|---|
| `--port <port>` | `8080` |
| `--host <host>` | `127.0.0.1` (use `0.0.0.0` to allow other machines to connect) |
| `--web-dir <dir>` | `web/` |
| `--book-dir <dir>` | `unzipped_opening_books/` |

### API

Positions are identified by the 0-indexed columns played to reach them, so `moves=3324` means
columns 3, 3, 2, then 4. An empty `moves=` is the empty board.

- `GET /api/evaluation?moves=<moves>`: the position's score, encoded as the frontend expects.
- `GET /api/alignment?moves=<moves>`: comma separated indices of the winning squares
  (counting from the top left, moving right then down), or empty if nobody has won.

Positions in the opening book are answered instantly; deeper positions are solved live.

## Binaries

- `web-server`: the server described above.
- `solver`: reads lines of `<1-indexed columns> <expected score>` from stdin, solves each
  position, and prints `columns,score,nodes,time_us` as CSV. Exits with an error if a score
  differs from the expected one.
  ```sh
  cargo run --release --bin solver < test_positions.txt
  ```
- `book-maker`: builds the opening book for one depth (see below).

## The opening book

Each depth has its own book, `unzipped_opening_books/depth_NN.bin`, holding every position
reachable after `NN` moves with its exact score. Positions where the game is already won are
left out. A book is a sorted, flat list of little-endian `u64`s (`key << 8 | score`), looked up by
binary search. Both a position and its mirror image are stored, for faster lookups.

To build books for depths 0 through N, resuming from any books that already exist:

```sh
scripts/build_book.sh <N> [timings_csv]   # timings_csv: optionally record how long each depth took
scripts/zip_book.sh                       # repack every book into openingBook.zip, to commit
```

`build_book.sh` calls `book-maker` once per depth:

```sh
book-maker depth_00.bin                        # depth 0: just the empty board
book-maker depth_01.bin --prior depth_00.bin   # depth n + 1 from depth n
```

Set `RAYON_NUM_THREADS` to limit how many cores `book-maker` uses.
