//! `yeokja-eval`: the Korean translation eval (see `evals/ko-translation/README.md`).

mod extract;
mod gate;
mod human;
mod item;
mod judge;
mod manifest;
mod report;
mod run;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "yeokja-eval", about = "Freeze, run and judge the translation eval set")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Freeze production requests from the translation projects into the set
    Extract {
        /// Repository root
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        /// Set directory (holds sources.toml and synthetic/)
        #[arg(long, default_value = "evals/ko-translation/v1")]
        out: PathBuf,
        #[arg(long, default_value_t = 20260923)]
        seed: u64,
        /// Real requests to aim for
        #[arg(long, default_value_t = 60)]
        target: usize,
        #[arg(long, default_value_t = 2)]
        per_project_min: usize,
        #[arg(long, default_value_t = 4)]
        per_project_max: usize,
    },
    /// Translate every item with a candidate model
    Run {
        /// Candidate file (label and [provider])
        candidate: PathBuf,
        #[arg(long, default_value = "evals/ko-translation/v1")]
        set: PathBuf,
        /// Output directory (evals/ko-translation/runs/<date>-<label>)
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value_t = 8)]
        concurrency: usize,
        /// Production default for every project
        #[arg(long, default_value_t = 3)]
        max_retries: u32,
        /// Translate each item this many times
        #[arg(long, default_value_t = 1)]
        repeat: u32,
        /// Only this many items, picked by a fixed hash
        #[arg(long)]
        subset: Option<usize>,
        /// Only items whose id contains this
        #[arg(long)]
        only: Option<String>,
        /// Fuse: the candidate edits these runs' translations into its own
        #[arg(long = "draft")]
        drafts: Vec<PathBuf>,
        /// With --draft: put each corrected-away translation in the first
        /// draft and run only the items that have one
        #[arg(long)]
        contaminate: bool,
    },
    /// Mechanical checks per block of each run (writes <run>/gates.jsonl)
    Gate {
        /// Run directories
        runs: Vec<PathBuf>,
        #[arg(long, default_value = "evals/ko-translation/v1")]
        set: PathBuf,
    },
    /// Blind pairwise judging of candidates against the baseline
    Judge {
        /// Baseline run directory
        #[arg(long)]
        baseline: PathBuf,
        /// Candidate run directories
        #[arg(long = "candidate", required = true)]
        candidates: Vec<PathBuf>,
        /// Judge files (label and [provider])
        #[arg(long = "judge", required = true)]
        judges: Vec<PathBuf>,
        /// Output directory (evals/ko-translation/judgments/<date>-<label>)
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "evals/ko-translation/v1")]
        set: PathBuf,
        #[arg(long, default_value_t = 8)]
        concurrency: usize,
        #[arg(long, default_value_t = 20260923)]
        seed: u64,
        /// Real blocks in the judging sample (used only when the sample is first frozen)
        #[arg(long, default_value_t = 150)]
        real: usize,
    },
    /// Write the Markdown report
    Report {
        #[arg(long)]
        baseline: PathBuf,
        #[arg(long = "candidate", required = true)]
        candidates: Vec<PathBuf>,
        /// Judgment directory (pairs.jsonl, human.jsonl)
        #[arg(long)]
        judgments: PathBuf,
        /// Report file (evals/ko-translation/reports/<date>-<label>.md)
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "evals/ko-translation/v1")]
        set: PathBuf,
    },
    /// Pick blind pairs for the person's judging page
    HumanPairs {
        #[arg(long)]
        baseline: PathBuf,
        #[arg(long = "candidate", required = true)]
        candidates: Vec<PathBuf>,
        /// Judgment directory
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value_t = 30)]
        count: usize,
        #[arg(long, default_value_t = 20260923)]
        seed: u64,
        #[arg(long, default_value = "evals/ko-translation/v1")]
        set: PathBuf,
    },
    /// Turn the page's verdicts (JSON by pair id) into human.jsonl
    ImportHuman {
        /// Judgment directory (holds blind-map.jsonl)
        #[arg(long)]
        out: PathBuf,
        verdicts: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")))
        .with_writer(std::io::stderr)
        .init();
    match Cli::parse().command {
        Commands::Extract { repo, out, seed, target, per_project_min, per_project_max } => {
            extract::run(&extract::Options { repo, out, seed, target, per_project_min, per_project_max })
        }
        Commands::Run { candidate, set, out, concurrency, max_retries, repeat, subset, only, drafts, contaminate } => {
            run::run(run::Options { set, drafts, contaminate, candidate, out, concurrency, max_retries, repeat, subset, only })
                .await
        }
        Commands::Gate { runs, set } => gate::run(set, runs).await,
        Commands::Judge { baseline, candidates, judges, out, set, concurrency, seed, real } => {
            judge::run(judge::Options { set, baseline, candidates, judges, out, concurrency, seed, real }).await
        }
        Commands::HumanPairs { baseline, candidates, out, count, seed, set } => {
            human::pairs(&set, &baseline, &candidates, &out, count, seed)
        }
        Commands::ImportHuman { out, verdicts } => human::import(&out, &verdicts),
        Commands::Report { baseline, candidates, judgments, out, set } => {
            report::run(report::Options { set, baseline, candidates, judgments, out })
        }
    }
}
