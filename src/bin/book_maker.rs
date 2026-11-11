//! Builds the opening book for a single depth.
//!
//! Usage: `book-maker <output_book> [--prior <prior_book>]`
//!
//! - Without `--prior`, builds the depth 0 book (just the empty board).
//! - With `--prior`, reads the book for depth `n`, and builds the book for depth `n + 1`
//!   by solving every child of every position in it.
//!
//! To build books for depths 0 through N, call this N + 1 times, e.g.
//! ```text
//! book-maker depth_00.bin
//! book-maker depth_01.bin --prior depth_00.bin
//! book-maker depth_02.bin --prior depth_01.bin
//! ```

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use connect_four_rust::board::{GAME_HEIGHT, GAME_WIDTH, Position};
use connect_four_rust::live_evaluation::Solver;
use connect_four_rust::opening_book::{OpeningBook, generation};

const PROGRESS_INTERVAL: Duration = Duration::from_secs(10);
const USAGE: &str = "Usage: book-maker <output_book> [--prior <prior_book>]";

struct Args {
    output_book: PathBuf,
    prior_book: Option<PathBuf>,
}

fn parse_args() -> Result<Args> {
    let mut output_book = None;
    let mut prior_book = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--prior" => prior_book = Some(args.next().context(USAGE)?.into()),
            _ if output_book.is_none() => output_book = Some(arg.into()),
            _ => bail!("Unexpected argument `{arg}`\n{USAGE}"),
        }
    }

    Ok(Args {
        output_book: output_book.context(USAGE)?,
        prior_book,
    })
}

fn main() -> Result<()> {
    let args = parse_args()?;
    if args.output_book.exists() {
        bail!("{} already exists, refusing to overwrite it", args.output_book.display());
    }

    let start = Instant::now();
    let positions = match &args.prior_book {
        None => vec![Position::new_blank_game()],
        Some(prior_path) => {
            let prior_book = OpeningBook::load(prior_path)?;
            let prior_depth = prior_book.depth().with_context(|| {
                format!("{} is empty or mixes depths", prior_path.display())
            })?;
            if prior_depth + 1 >= GAME_WIDTH * GAME_HEIGHT {
                bail!("{} is already at the final depth", prior_path.display());
            }
            generation::positions_after(&prior_book)
        }
    };
    let depth = positions[0].moves_played;
    eprintln!(
        "Depth {depth}: generated {} positions to solve (excluding mirrors) in {:.1?}, using {} threads",
        positions.len(),
        start.elapsed(),
        rayon::current_num_threads()
    );

    let solver = Solver::new();
    let solve_start = Instant::now();
    let last_report = Mutex::new(Instant::now());
    let total = positions.len();
    let book = generation::solve_positions(&solver, &positions, |solved| {
        // try_lock: never make a worker wait just to print progress
        let Ok(mut last) = last_report.try_lock() else {
            return;
        };
        if last.elapsed() < PROGRESS_INTERVAL {
            return;
        }
        *last = Instant::now();
        let elapsed = solve_start.elapsed();
        let remaining = elapsed.mul_f64((total - solved) as f64 / solved as f64);
        eprintln!(
            "  {solved}/{total} ({:.1}%), elapsed {elapsed:.0?}, estimated remaining {remaining:.0?}",
            100.0 * solved as f64 / total as f64,
        );
    });

    book.save(&args.output_book)?;
    eprintln!(
        "Depth {depth}: wrote {} entries to {} in {:.1?}",
        book.len(),
        args.output_book.display(),
        start.elapsed()
    );
    Ok(())
}
