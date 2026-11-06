//! Gambiit terminal prototype.
//!
//! This is the Phase 0 risk spike: prove the rules, Stockfish candidates and
//! OpenCode prompt transport before building the Tauri UI. The final game uses
//! buttons and automatic agent turns; here the human types moves in a terminal.

use anyhow::{Context, Result, anyhow, bail};
use clap::{Args, Parser, Subcommand};
use gambiit_agent::{AgentConfig, AgentSession, AgentTurn, OpenCodeClient, OpenCodeOutput};
use gambiit_core::{Game, MoveInfo, color_name};
use gambiit_engine::Stockfish;
use shakmaty::{Color, Role, Square};
use std::{
    io::{self, Write},
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Parser)]
#[command(name = "gambiit-cli", version, about = "Gambiit terminal prototype")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Play a game against an OpenCode model, a mock opponent or Stockfish.
    Play(PlayArgs),
    /// Check OpenCode, model and Stockfish availability.
    Probe(ProbeArgs),
    /// Count legal move sequences (perft) from the starting position.
    Perft { depth: u32 },
}

#[derive(Debug, Args)]
struct PlayArgs {
    /// OpenCode model in provider/model format.
    #[arg(long, default_value = "opencode-go/deepseek-v4.1-flash")]
    model: String,
    /// Your colour: w/white or b/black.
    #[arg(long, default_value = "w")]
    color: String,
    /// Agent persona.
    #[arg(long, default_value = "Theater Master")]
    persona: String,
    /// Agent language: en or et.
    #[arg(long, default_value = "en")]
    language: String,
    /// OpenCode request timeout in seconds.
    #[arg(long, default_value_t = 180)]
    timeout: u64,
    /// Retries after an invalid agent response.
    #[arg(long, default_value_t = 1)]
    retries: usize,
    /// Path to the Stockfish binary. Auto-detected when omitted.
    #[arg(long)]
    stockfish: Option<PathBuf>,
    /// Stockfish search depth for candidate moves.
    #[arg(long, default_value_t = 12)]
    depth: u32,
    /// Number of Stockfish candidate moves.
    #[arg(long, default_value_t = 3)]
    multipv: usize,
    /// Disable Stockfish entirely.
    #[arg(long)]
    no_stockfish: bool,
    /// Opponent mode: opencode, mock or stockfish.
    #[arg(long, default_value = "opencode")]
    agent: String,
    /// Stop after this many plies (0 = unlimited).
    #[arg(long, default_value_t = 0)]
    max_plies: u32,
    /// Resume a specific OpenCode session.
    #[arg(long)]
    session: Option<String>,
}

#[derive(Debug, Args)]
struct ProbeArgs {
    /// OpenCode model to test with --prompt.
    #[arg(long, default_value = "opencode-go/deepseek-v4.1-flash")]
    model: String,
    /// Path to the Stockfish binary. Auto-detected when omitted.
    #[arg(long)]
    stockfish: Option<PathBuf>,
    /// Send a tiny prompt through OpenCode and print the response.
    #[arg(long)]
    prompt: bool,
    /// Prompt timeout in seconds.
    #[arg(long, default_value_t = 120)]
    timeout: u64,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Play(args) => play(args),
        Command::Probe(args) => probe(args),
        Command::Perft { depth } => run_perft(depth),
    }
}

fn play(args: PlayArgs) -> Result<()> {
    let human_color = parse_color(&args.color)?;
    let mut game = Game::new();
    game.set_human_color(human_color);

    let mut stockfish = open_stockfish(&args);
    let mut session = if args.agent == "opencode" {
        let config = AgentConfig {
            model: args.model.clone(),
            persona: args.persona.clone(),
            language: args.language.clone(),
            language_name: if args.language.eq_ignore_ascii_case("et") {
                "Estonian".to_string()
            } else {
                "English".to_string()
            },
            timeout_secs: args.timeout,
            retries: args.retries,
            env: Vec::new(),
            style_instruction: String::new(),
            persona_prompt: String::new(),
        };
        let mut session = AgentSession::new(config)?;
        session.set_session_id(args.session.clone());
        Some(session)
    } else {
        None
    };

    println!("\x1b[1mGambiit\x1b[0m — terminal prototype");
    println!(
        "Model: {} | You: {} | Agent: {}",
        args.model,
        color_name(human_color),
        color_name(!human_color)
    );
    if let Some(engine) = &stockfish {
        println!(
            "Engine: {} (depth {}, MultiPV {})",
            engine.id(),
            args.depth,
            args.multipv
        );
    } else {
        println!("Engine: unavailable — playing without Stockfish");
    }
    println!("Type 'help' for commands.\n");

    print_board(&game, human_color);

    loop {
        if args.max_plies > 0 && game.history().len() >= args.max_plies as usize {
            println!("\nStopped after {} plies.", game.history().len());
            break;
        }

        let status = game.status();
        if status.is_over() {
            println!("\n\x1b[1m{}\x1b[0m", game.status_text());
            break;
        }

        if game.turn() == human_color {
            print_legal_moves(&game);
            let Some(input) = read_line("\nYour move › ")? else {
                break;
            };
            let input = input.trim();
            if input.is_empty() {
                continue;
            }
            match input {
                "q" | "quit" | "exit" => break,
                "h" | "help" | "?" => {
                    print_help();
                    continue;
                }
                "m" | "moves" => {
                    print_legal_moves(&game);
                    continue;
                }
                "fen" => {
                    println!("{}", game.fen());
                    continue;
                }
                "pgn" => {
                    println!("{}", game.pgn_moves());
                    continue;
                }
                "resign" => {
                    game.resign(!human_color);
                    println!("{}", game.status_text());
                    break;
                }
                _ => {}
            }

            match game.apply_uci(input).or_else(|_| game.apply_san(input)) {
                Ok(info) => {
                    println!("You play {} ({})", info.san, info.uci);
                    print_board(&game, human_color);
                }
                Err(error) => {
                    eprintln!("Illegal move: {error}");
                    eprintln!("Use 'moves' to see legal moves.");
                }
            }
        } else {
            let opponent_label = match args.agent.as_str() {
                "mock" => "Mock opponent".to_string(),
                "stockfish" => "Stockfish".to_string(),
                _ => format!("Agent ({})", args.model),
            };
            println!("\n\x1b[2m{opponent_label} is thinking...\x1b[0m");
            match request_agent_turn(&game, &args.agent, &mut session, &mut stockfish, &args) {
                Ok((turn, output, elapsed)) => {
                    let applied = game
                        .apply_uci(&turn.mv)
                        .or_else(|_| game.apply_san(&turn.mv));
                    match applied {
                        Ok(info) => {
                            game.annotate_last(Some(turn.comment.clone()), turn.eval.clone());
                            print_agent_turn(&turn, &info, elapsed, output.as_ref());
                            print_board(&game, human_color);
                        }
                        Err(error) => {
                            eprintln!("Agent returned an illegal move: {error}");
                            let next = read_line("Enter = retry, q = quit › ")?;
                            if next.as_deref() == Some("q") {
                                break;
                            }
                        }
                    }
                }
                Err(error) => {
                    eprintln!("Agent error: {error:#}");
                    let next = read_line("Enter = retry, s = Stockfish move, q = quit › ")?;
                    match next.as_deref() {
                        Some("q") => break,
                        Some("s") => {
                            if let Some(engine) = stockfish.as_mut() {
                                match engine.best_move(&game.fen(), args.depth) {
                                    Ok(best) => match game.apply_uci(&best) {
                                        Ok(info) => {
                                            game.annotate_last(
                                                Some(
                                                    "Stockfish fallback: the agent did not answer."
                                                        .into(),
                                                ),
                                                Some("engine".into()),
                                            );
                                            println!(
                                                "Stockfish fallback plays {} ({})",
                                                info.san, info.uci
                                            );
                                            print_board(&game, human_color);
                                        }
                                        Err(error) => {
                                            eprintln!(
                                                "Stockfish returned an illegal move: {error}"
                                            );
                                        }
                                    },
                                    Err(error) => {
                                        eprintln!("Stockfish failed: {error}");
                                    }
                                }
                            } else {
                                eprintln!("Stockfish is not available.");
                            }
                        }
                        _ => continue,
                    }
                }
            }
        }
    }

    Ok(())
}

fn request_agent_turn(
    game: &Game,
    mode: &str,
    session: &mut Option<AgentSession>,
    stockfish: &mut Option<Stockfish>,
    args: &PlayArgs,
) -> Result<(AgentTurn, Option<OpenCodeOutput>, Duration)> {
    let mut candidates = Vec::new();
    if let Some(engine) = stockfish.as_mut() {
        match engine.candidates(&game.fen(), args.multipv, args.depth) {
            Ok(found) => candidates = found,
            Err(error) => eprintln!("Stockfish warning: {error}"),
        }
    }

    let started = Instant::now();
    match mode {
        "opencode" => {
            let session = session
                .as_mut()
                .context("OpenCode session is not initialised")?;
            let snapshot = game.clone();
            let legal_hint = game.legal_uci().join(", ");
            let validate = move |turn: &AgentTurn| {
                let mut probe = snapshot.clone();
                if probe.apply_uci(&turn.mv).is_ok() {
                    return Ok(());
                }
                if probe.apply_san(&turn.mv).is_ok() {
                    return Ok(());
                }
                Err(format!(
                    "'{}' is not a legal move. Legal UCI moves: {}",
                    turn.mv, legal_hint
                ))
            };
            let (turn, output) = session.request_move(game, &candidates, validate)?;
            Ok((turn, Some(output), started.elapsed()))
        }
        "mock" => {
            let mv = candidates
                .first()
                .map(|candidate| candidate.uci.clone())
                .unwrap_or_else(|| {
                    let legal = game.legal_moves();
                    let nanos = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos() as usize;
                    legal[nanos % legal.len()].uci.clone()
                });
            let (comment, eval) = candidates.first().map_or_else(
                || ("Mock agent picks a random legal move.".to_string(), None),
                |candidate| {
                    (
                        format!("Mock agent follows Stockfish: {}.", candidate.score_text()),
                        Some(candidate.score_text()),
                    )
                },
            );
            Ok((
                AgentTurn {
                    mv,
                    comment,
                    eval,
                    candidates: Vec::new(),
                    plan: None,
                },
                None,
                started.elapsed(),
            ))
        }
        "stockfish" => {
            let candidate = candidates.first().ok_or_else(|| {
                anyhow!("Stockfish is unavailable; install it or use --agent mock")
            })?;
            Ok((
                AgentTurn {
                    mv: candidate.uci.clone(),
                    comment: format!("Stockfish plays {}.", candidate.score_text()),
                    eval: Some(candidate.score_text()),
                    candidates: Vec::new(),
                    plan: None,
                },
                None,
                started.elapsed(),
            ))
        }
        other => bail!("unknown agent mode '{other}' (expected opencode, mock or stockfish)"),
    }
}

fn open_stockfish(args: &PlayArgs) -> Option<Stockfish> {
    if args.no_stockfish {
        println!("Stockfish: disabled (--no-stockfish)");
        return None;
    }
    let path = args.stockfish.clone().or_else(Stockfish::discover);
    let Some(path) = path else {
        println!(
            "Stockfish: not found. Install with `sudo pacman -S --needed stockfish`; continuing without engine."
        );
        return None;
    };
    match Stockfish::new(&path) {
        Ok(engine) => {
            println!("Stockfish: {} ({})", engine.id(), path.display());
            Some(engine)
        }
        Err(error) => {
            eprintln!("Stockfish: failed to start {}: {error}", path.display());
            None
        }
    }
}

fn probe(args: ProbeArgs) -> Result<()> {
    println!("\x1b[1mGambiit probe\x1b[0m");
    let client = OpenCodeClient::new(args.timeout.max(10)).ok();
    match client.as_ref() {
        Some(client) => {
            let path = client.binary();
            let version = command_output_path(path, &["--version"])
                .unwrap_or_else(|| "version unknown".to_string());
            println!("OpenCode: {} ({})", version, path.display());
            match command_output_path(path, &["models"]) {
                Some(models) => println!(
                    "Models: {} available",
                    models
                        .lines()
                        .filter(|line| !line.trim().is_empty())
                        .count()
                ),
                None => println!("Models: could not list (check `opencode auth`)"),
            }
        }
        None => println!("OpenCode: not found (set OPENCODE_BIN or install OpenCode)"),
    }

    let stockfish_path = args.stockfish.clone().or_else(Stockfish::discover);
    match stockfish_path {
        Some(path) => match Stockfish::new(&path) {
            Ok(engine) => println!("Stockfish: {} ({})", engine.id(), path.display()),
            Err(error) => println!("Stockfish: failed to start {}: {error}", path.display()),
        },
        None => println!("Stockfish: not found. Install with `sudo pacman -S --needed stockfish`."),
    }

    if args.prompt {
        println!("\nPrompt test with {}...", args.model);
        let client = client.context("OpenCode was not found")?;
        let config = AgentConfig {
            model: args.model.clone(),
            ..Default::default()
        };
        let output = client.run(
            &config,
            "Return exactly this JSON and nothing else: {\"ok\":true,\"move\":\"e2e4\"}",
            None,
        )?;
        println!("Response: {}", output.text.trim());
        if let Some(session) = output.session_id {
            println!("Session: {session}");
        }
        if let Some(tokens) = output.tokens {
            println!("Tokens: {tokens}");
        }
        if let Some(cost) = output.cost {
            println!("Cost: ${cost:.4}");
        }
    }

    Ok(())
}

fn run_perft(depth: u32) -> Result<()> {
    if depth > 6 {
        bail!("perft depth above 6 is too slow for the prototype");
    }
    let game = Game::new();
    for current in 1..=depth {
        let started = Instant::now();
        let nodes = game.perft(current);
        println!(
            "perft({current}) = {nodes} ({:.3}s)",
            started.elapsed().as_secs_f64()
        );
    }
    Ok(())
}

fn parse_color(input: &str) -> Result<Color> {
    match input.to_ascii_lowercase().as_str() {
        "w" | "white" => Ok(Color::White),
        "b" | "black" => Ok(Color::Black),
        _ => bail!("colour must be w/white or b/black"),
    }
}

fn print_help() {
    println!(
        "\nCommands:\n  <SAN/UCI>   make a move, e.g. e4 or e2e4\n  m, moves    list legal moves\n  fen         show the current FEN\n  pgn         show the move history\n  resign      resign the game\n  q, quit     quit\n"
    );
}

fn print_legal_moves(game: &Game) {
    let legal = game.legal_moves();
    println!("Legal moves ({}):", legal.len());
    for chunk in legal.chunks(6) {
        let line = chunk
            .iter()
            .map(|mv| format!("{} [{}]", mv.san, mv.uci))
            .collect::<Vec<_>>()
            .join("  ");
        println!("  {line}");
    }
}

fn print_board(game: &Game, human_color: Color) {
    let flip = human_color == Color::Black;
    let ranks: Vec<u32> = if flip {
        (0..8).collect()
    } else {
        (0..8).rev().collect()
    };
    let files: Vec<u32> = if flip {
        (0..8).rev().collect()
    } else {
        (0..8).collect()
    };
    let last = game
        .last_move()
        .map(|record| (record.from.as_str(), record.to.as_str()));

    println!();
    for rank in ranks {
        print!("\x1b[2;37m{}\x1b[0m ", rank + 1);
        for &file in &files {
            let square = Square::new(rank * 8 + file);
            let light = (file + rank) % 2 == 1;
            let mut background = if light {
                "\x1b[48;5;180m"
            } else {
                "\x1b[48;5;94m"
            };
            let square_name = square.to_string();
            if let Some((from, to)) = last
                && (square_name == from || square_name == to)
            {
                background = "\x1b[48;5;28m";
            }

            let piece = game.piece_at(square);
            if game.is_check()
                && let Some(piece) = piece
                && piece.color == game.turn()
                && piece.role == Role::King
            {
                background = "\x1b[48;5;88m";
            }

            let glyph = piece
                .map(|piece| piece.unicode_char().to_string())
                .unwrap_or_else(|| " ".to_string());
            let foreground = match piece {
                Some(piece) if piece.color == Color::White => "\x1b[1;97m",
                Some(_) => "\x1b[1;94m",
                None => "\x1b[0m",
            };
            print!("{background}{foreground} {glyph} \x1b[0m");
        }
        println!();
    }

    print!("   ");
    for &file in &files {
        print!("\x1b[2;37m {} \x1b[0m", (b'a' + file as u8) as char);
    }
    println!("\n\n\x1b[1m{}\x1b[0m", game.status_text());
}

fn print_agent_turn(
    turn: &AgentTurn,
    info: &MoveInfo,
    elapsed: Duration,
    output: Option<&OpenCodeOutput>,
) {
    let eval = turn
        .eval
        .as_deref()
        .map(|eval| format!("  eval {eval}"))
        .unwrap_or_default();
    println!("\n┌─ Agent plays {} ({}){}", info.san, info.uci, eval);

    let mut meta = format!("{:.1}s", elapsed.as_secs_f64());
    if let Some(output) = output {
        if let Some(tokens) = output.tokens {
            meta.push_str(&format!(", {tokens} tokens"));
        }
        if let Some(cost) = output.cost {
            meta.push_str(&format!(", ${cost:.4}"));
        }
    }
    println!("│ {meta}");

    if !turn.comment.trim().is_empty() {
        for line in wrap_text(turn.comment.trim(), 78) {
            println!("│ {line}");
        }
    }
    if !turn.candidates.is_empty() {
        println!("│ Candidates:");
        for candidate in &turn.candidates {
            let mut parts = vec![candidate.mv.clone()];
            if let Some(why) = &candidate.why {
                parts.push(why.clone());
            }
            if let Some(why_rejected) = &candidate.why_rejected {
                parts.push(format!("rejected: {why_rejected}"));
            }
            println!("│   • {}", parts.join(" — "));
        }
    }
    if let Some(plan) = &turn.plan
        && !plan.trim().is_empty()
    {
        println!("│ Plan: {plan}");
    }
    println!("└");
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.len() + 1 + word.len() > width {
            lines.push(current);
            current = String::new();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn read_line(prompt: &str) -> Result<Option<String>> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut line = String::new();
    if io::stdin().read_line(&mut line)? == 0 {
        println!();
        return Ok(None);
    }
    Ok(Some(line.trim().to_string()))
}

fn command_output_path(binary: &std::path::Path, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(binary)
        .args(args)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}
