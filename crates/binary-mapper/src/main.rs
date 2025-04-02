use std::{fs::File, path::PathBuf};

use clap::{Parser, ValueEnum};
use memmap::MmapOptions;
use pelite::{pattern, PeFile};
use rayon::prelude::*;
use serde::Deserialize;

#[derive(ValueEnum, Clone)]
enum OutputFormat {
    Print,
    Rust,
}

/// Run a mapper profile against a binary to produce
#[derive(Parser)]
struct Args {
    #[arg(long, env("MAPPER_PROFILE"))]
    profile: PathBuf,

    #[arg(long, env("MAPPER_GAME_EXE"))]
    exe: PathBuf,

    #[arg(long, env("MAPPER_OUTPUT_FORMAT"))]
    output: OutputFormat,
}

fn main() {
    let args = Args::parse();

    let exe_file = File::open(&args.exe).expect("Could not open game binary");
    let exe_mmap =
        unsafe { MmapOptions::new().map(&exe_file) }.expect("Could not mmap game binary");
    let program =
        PeFile::from_bytes(&exe_mmap[0..]).expect("Could not create PE view for game binary");

    let contents = std::fs::read_to_string(args.profile).expect("Could not read profile file");
    let profile: MapperProfile = toml::from_str(&contents).expect("Could not parse profile TOML");

    let result = profile
        .patterns
        .into_par_iter()
        .flat_map(|entry| {
            let scanner_pattern = pattern::parse(&entry.pattern).unwrap_or_else(|_| {
                panic!("Could not parse provided pattern \"{}\"", &entry.pattern)
            });

            let captures = entry
                .captures
                .iter()
                .enumerate()
                .filter(|(_, e)| !e.is_empty());

            let mut matches = vec![0u32; entry.captures.len()];
            if !program
                .scanner()
                .matches_code(&scanner_pattern)
                .next(&mut matches)
            {
                captures
                    .map(|(_, e)| MapperEntryResult {
                        name: e.clone(),
                        found: false,
                        rva: 0x0,
                    })
                    .collect::<Vec<_>>()
            } else {
                captures
                    .map(|(i, e)| MapperEntryResult {
                        name: e.clone(),
                        found: true,
                        rva: matches[i],
                    })
                    .collect::<Vec<_>>()
            }
        })
        .collect::<Vec<_>>();

    match args.output {
        OutputFormat::Print => println!("Results: {result:#x?}"),
        OutputFormat::Rust => {
            let lines = result
                .iter()
                .map(|r| format!("pub const RVA_{}: u32 = {:#x};", r.name, r.rva))
                .collect::<Vec<_>>();
            println!("{}", lines.join("\n"));
        }
    }
}

/// Profile describing what offsets to extract from a game binary.
#[derive(Debug, Deserialize)]
pub struct MapperProfile {
    pub patterns: Vec<MapperProfilePattern>,
}

/// Profile describing what offsets to extract from a game binary.
#[derive(Debug, Deserialize)]
pub struct MapperProfilePattern {
    /// Pattern used for matching. Under the hood this uses pelite's parser.
    /// As such, the same pattern syntax is used.
    /// More: https://docs.rs/pelite/latest/pelite/pattern/fn.parse.html
    pub pattern: String,
    /// Names for the captures. These names can be referenced from the generated
    /// definition file.
    pub captures: Vec<String>,
}

/// Result of one of the entry items.
#[derive(Debug, Deserialize)]
pub struct MapperEntryResult {
    pub name: String,
    pub found: bool,
    pub rva: u32,
}
