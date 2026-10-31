//! Read-only process inspection utility used by the world-query research notes.
mod process;
mod scanner;

use process::{ReadOnlyProcess, find_game_image};
use std::process::ExitCode;

#[cfg(not(windows))]
compile_error!("trace-xref-probe is Windows-only");

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err((code, message)) => {
            eprintln!("{message}");
            ExitCode::from(code)
        }
    }
}

fn run(args: Vec<String>) -> Result<(), (u8, String)> {
    let pid = parse_pid(&args)?;
    let process = ReadOnlyProcess::open(pid).map_err(|error| (1, error))?;
    match args.get(1).map(String::as_str) {
        Some("--peek") | Some("--peek-any") => peek(&process, pid, &args),
        Some("--image-range") => scan_explicit_range(&process, pid, &args),
        _ => scan_game_image(&process, pid),
    }
}

fn parse_pid(args: &[String]) -> Result<u32, (u8, String)> {
    args.first()
        .and_then(|arg| arg.parse::<u32>().ok())
        .ok_or_else(|| (2, usage()))
}

fn peek(process: &ReadOnlyProcess, pid: u32, args: &[String]) -> Result<(), (u8, String)> {
    let address = parse_hex(args.get(2)).ok_or_else(|| (2, usage()))?;
    let length = parse_length(args.get(3));
    let arbitrary = args.get(1).is_some_and(|mode| mode == "--peek-any");
    let range = if arbitrary {
        None
    } else {
        Some(find_game_image(pid).ok_or_else(|| (1, "Could not find MadMax.exe.".into()))?)
    };
    validate_peek_range(address, length, range)?;
    let bytes = process.read(address, length).map_err(|error| (1, error))?;
    print_bytes(address, &bytes);
    Ok(())
}

fn scan_explicit_range(
    process: &ReadOnlyProcess,
    pid: u32,
    args: &[String],
) -> Result<(), (u8, String)> {
    let range = parse_hex(args.get(2))
        .zip(parse_hex(args.get(3)))
        .and_then(|(base, size)| base.checked_add(size).map(|end| (base, end)))
        .ok_or_else(|| (2, usage()))?;
    if range.0 >= range.1 {
        return Err((2, usage()));
    }
    scan_range(process, pid, range)
}

fn scan_game_image(process: &ReadOnlyProcess, pid: u32) -> Result<(), (u8, String)> {
    let range = find_game_image(pid)
        .ok_or_else(|| (1, "Could not determine MadMax.exe image range.".into()))?;
    scan_range(process, pid, range)
}

fn scan_range(
    process: &ReadOnlyProcess,
    pid: u32,
    (start, end): (usize, usize),
) -> Result<(), (u8, String)> {
    println!("Read-only scan: PID {pid}, image {start:#x}..{end:#x}");
    let hits = scanner::scan_image(process, start, end);
    println!("Finished. Candidate references found: {hits}");
    Ok(())
}

fn parse_hex(value: Option<&String>) -> Option<usize> {
    value.and_then(|text| usize::from_str_radix(text.trim_start_matches("0x"), 16).ok())
}

fn parse_length(value: Option<&String>) -> usize {
    value
        .and_then(|text| text.parse::<usize>().ok())
        .unwrap_or(64)
        .clamp(1, 256)
}

fn validate_peek_range(
    address: usize,
    length: usize,
    image: Option<(usize, usize)>,
) -> Result<(), (u8, String)> {
    if let Some((start, end)) = image
        && (address < start
            || address
                .checked_add(length)
                .is_none_or(|peek_end| peek_end > end))
    {
        return Err((2, "Peek range is outside the MadMax.exe image.".into()));
    }
    Ok(())
}

fn print_bytes(address: usize, bytes: &[u8]) {
    let output = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    println!("{address:#x}: {output}");
}

fn usage() -> String {
    "Usage: trace_xref_probe <PID> [--image-range <base-hex> <size-hex> | --peek <address> [length] | --peek-any <address> [length]]".into()
}
