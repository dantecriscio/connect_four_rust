//! Serves the web frontend, plus an API for evaluating positions.
//!
//! Usage: `web-server [--port <port>] [--host <host>] [--web-dir <dir>] [--book-dir <dir>]`
//!
//! Positions are identified by the 0-indexed columns played to reach them, e.g. `?moves=3324`.
//! - `GET /api/evaluation?moves=<moves>`: the position's score, encoded for the frontend.
//! - `GET /api/alignment?moves=<moves>`: comma separated indices of the winning squares,
//!   or an empty string if the player who just moved has not won.
//! - Anything else is served as a static file from the web directory.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use axum::Router;
use axum::extract::{Query, State};
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::get;
use connect_four_rust::board::{EndState, GAME_HEIGHT, GAME_WIDTH, GameState, Position};
use connect_four_rust::live_evaluation::Solver;
use connect_four_rust::opening_book::{OpeningBook, book_path};
use serde::Deserialize;
use tower_http::services::ServeDir;

const USAGE: &str =
    "Usage: web-server [--port <port>] [--host <host>] [--web-dir <dir>] [--book-dir <dir>]";
const DEFAULT_PORT: u16 = 8080;
// Only reachable from this machine by default. Pass `--host 0.0.0.0` to expose it on the network.
const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_WEB_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/web");
const DEFAULT_BOOK_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/unzipped_opening_books");

// A position's answer never changes, so browsers may cache API responses forever
const CACHE_FOREVER: (header::HeaderName, &str) =
    (header::CACHE_CONTROL, "public, max-age=31536000, immutable");

const MAX_MOVES: i8 = (GAME_WIDTH * GAME_HEIGHT) as i8;
// Same values as the Java version's Position.MAX_SCORE and Position.MIN_SCORE
const MAX_SCORE: i8 = (MAX_MOVES + 1) / 2 - 3;
const MIN_SCORE: i8 = -MAX_MOVES / 2 + 3;

struct Args {
    address: SocketAddr,
    web_dir: PathBuf,
    book_dir: PathBuf,
}

fn parse_args() -> Result<Args> {
    let mut port = DEFAULT_PORT;
    let mut host = DEFAULT_HOST.to_owned();
    let mut web_dir = PathBuf::from(DEFAULT_WEB_DIR);
    let mut book_dir = PathBuf::from(DEFAULT_BOOK_DIR);

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().context(USAGE);
        match arg.as_str() {
            "--port" => port = value()?.parse().context("Invalid --port")?,
            "--host" => host = value()?,
            "--web-dir" => web_dir = value()?.into(),
            "--book-dir" => book_dir = value()?.into(),
            _ => bail!("Unexpected argument `{arg}`\n{USAGE}"),
        }
    }

    let address = format!("{host}:{port}")
        .parse()
        .with_context(|| format!("Invalid host `{host}`"))?;
    Ok(Args {
        address,
        web_dir,
        book_dir,
    })
}

struct AppState {
    // Shared by every request. Its transposition table is thread safe, and keeps
    // what it learned from earlier requests.
    solver: Solver,
    // books[depth] holds every position with `depth` moves played
    books: Vec<OpeningBook>,
}

/// Loads depth_00.bin, depth_01.bin, ... stopping at the first depth that is missing
fn load_books(book_dir: &std::path::Path) -> Result<Vec<OpeningBook>> {
    let mut books = Vec::new();
    for depth in 0..MAX_MOVES as u8 {
        let path = book_path(book_dir, depth);
        if !path.exists() {
            break;
        }
        books.push(OpeningBook::load(&path)?);
    }
    Ok(books)
}

#[derive(Deserialize)]
struct MovesQuery {
    moves: String,
}

type ApiError = (StatusCode, String);

fn parse_moves(moves: &str) -> Result<Position, ApiError> {
    let mut position = Position::new_blank_game();
    for (index, character) in moves.chars().enumerate() {
        let bad_request = |reason: String| {
            (StatusCode::BAD_REQUEST, format!("Move {} `{character}`: {reason}", index + 1))
        };
        let column = character
            .to_digit(10)
            .ok_or_else(|| bad_request("not a column digit".to_owned()))?;
        position
            .play_column(column as u8)
            .map_err(|error| bad_request(error.to_string()))?;
    }
    Ok(position)
}

/// Exact score from the current player's point of view, as returned by `Solver::solve`
fn current_player_score(state: &AppState, position: Position) -> i8 {
    if let GameState::Ended(EndState::PriorPlayerWon) = position.get_game_state() {
        // The current player has already lost
        return (position.moves_played as i8 - MAX_MOVES - 2) / 2;
    }

    state
        .books
        .get(position.moves_played as usize)
        .and_then(|book| book.get(position))
        .unwrap_or_else(|| state.solver.solve(position))
}

/// Converts a score into the encoding the frontend expects (same as the Java version):
/// from player one's point of view, then a win becomes `MAX_SCORE + 4 - score`
/// and a loss becomes `MIN_SCORE - 4 - score`. A draw stays 0.
fn frontend_score(position: Position, current_player_score: i8) -> i8 {
    let player_one_score = if position.moves_played % 2 == 1 {
        -current_player_score
    } else {
        current_player_score
    };

    match player_one_score {
        score if score > 0 => MAX_SCORE + 4 - score,
        score if score < 0 => MIN_SCORE - 4 - score,
        _ => 0,
    }
}

async fn evaluation(
    State(state): State<Arc<AppState>>,
    Query(query): Query<MovesQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let position = parse_moves(&query.moves)?;

    let start = Instant::now();
    // Solving can take seconds, so run it on Tokio's blocking thread pool
    // rather than tying up one of the threads that handles requests
    let score = tokio::task::spawn_blocking(move || current_player_score(&state, position))
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let encoded = frontend_score(position, score);

    println!(
        "evaluation moves={:?} score={encoded} in {:.1?}",
        query.moves,
        start.elapsed()
    );
    Ok(([CACHE_FOREVER], encoded.to_string()))
}

async fn alignment(Query(query): Query<MovesQuery>) -> Result<impl IntoResponse, ApiError> {
    let position = parse_moves(&query.moves)?;
    let indices: Vec<String> = position
        .prior_player_connect_four_indices()
        .iter()
        .map(u8::to_string)
        .collect();

    println!("alignment moves={:?} indices=[{}]", query.moves, indices.join(","));
    Ok(([CACHE_FOREVER], indices.join(",")))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args()?;

    if !args.web_dir.join("index.html").exists() {
        bail!("No index.html found in web directory {}", args.web_dir.display());
    }
    let books = load_books(&args.book_dir)?;
    match books.len() {
        0 => println!("No opening books found in {}", args.book_dir.display()),
        count => println!("Loaded opening books for depths 0 through {}", count - 1),
    }

    let state = Arc::new(AppState {
        solver: Solver::new(),
        books,
    });

    let app = Router::new()
        .route("/api/evaluation", get(evaluation))
        .route("/api/alignment", get(alignment))
        .with_state(state)
        // ServeDir also serves index.html for `/`
        .fallback_service(ServeDir::new(&args.web_dir));

    let listener = tokio::net::TcpListener::bind(args.address)
        .await
        .with_context(|| format!("Could not listen on {}", args.address))?;
    println!("Serving on http://{}", args.address);
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_moves_rejects_bad_input() {
        assert!(parse_moves("").is_ok());
        assert!(parse_moves("3324").is_ok());
        assert_eq!(parse_moves("37").unwrap_err().0, StatusCode::BAD_REQUEST);
        assert_eq!(parse_moves("3x").unwrap_err().0, StatusCode::BAD_REQUEST);
        assert_eq!(parse_moves("3333333").unwrap_err().0, StatusCode::BAD_REQUEST); // full column
        assert_eq!(parse_moves("01010101").unwrap_err().0, StatusCode::BAD_REQUEST); // after a win
    }

    #[test]
    fn frontend_score_matches_java_encoding() {
        let blank = Position::new_blank_game();
        // Empty board: player one wins with their last stone, a score of 1
        assert_eq!(frontend_score(blank, 1), MAX_SCORE + 3);
        assert_eq!(frontend_score(blank, 0), 0);
        assert_eq!(frontend_score(blank, -2), MIN_SCORE - 2);

        // After one move it is player two's turn, so the sign flips
        let one_move = parse_moves("3").unwrap();
        assert_eq!(frontend_score(one_move, -1), MAX_SCORE + 3);
    }

    #[test]
    fn frontend_score_after_a_win_says_the_game_is_over() {
        // Player one wins vertically in column 0 on move 7
        let position = parse_moves("0101010").unwrap();
        let state = AppState {
            solver: Solver::new(),
            books: Vec::new(),
        };
        let encoded = frontend_score(position, current_player_score(&state, position));
        // The frontend decides a game is won when this equals the move count
        let winning_move_number = if encoded > 0 { encoded * 2 - 1 } else { encoded * -2 };
        assert_eq!(winning_move_number, 7);
    }
}
