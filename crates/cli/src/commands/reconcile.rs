use anyhow::Result;
use std::path::Path;
use yeokja_core::hash::content_hash;
use yeokja_core::project::ProjectContext;
use yeokja_core::reconcile::reconcile_accepting_context;
use yeokja_core::state::StateFile;
use yeokja_translate::orchestrator::{collect_files, scan_file};

/// Migrate saved state to a new parser's segment shape. Every file is
/// re-parsed and reconciled against its existing state: unchanged source
/// keeps its translation under the new segment id, `ContextChanged` is folded
/// in (its `context_hash` moves to the current value so `translate` will not
/// redo it), and segments with no counterpart are added (Pending) or dropped.
/// `GlossaryStale` segments are left untouched — a parser reshape does not
/// change the glossary. A file with no state yet is skipped: there is no
/// translation to migrate, and `translate` will create its state.
///
/// No model is called. Use this once, right after a parser change; do not use
/// it after pulling in updated source (a bumped submodule), since a
/// `ContextChanged` there may be a real context change, not a reshape.
pub fn run(path: &str, dry_run: bool) -> Result<()> {
    let ctx = ProjectContext::load()?;
    let parser_factory = super::parser_factory();

    let files = collect_files(Path::new(path), &ctx.config)?;

    let mut total_kept = 0usize;
    let mut total_added = 0usize;
    let mut total_dropped = 0usize;
    let mut total_context_accepted = 0usize;
    let mut total_glossary_stale = 0usize;
    let mut skipped = 0usize;

    for file_path in &files {
        let state_path = StateFile::state_file_path(file_path, ctx.config.state_dir());
        if !state_path.exists() {
            println!("{}: skipped (no state)", file_path.display());
            skipped += 1;
            continue;
        }
        let existing = StateFile::load(&state_path)?;
        let (document, _) = scan_file(file_path, &ctx.config, &ctx.glossary, &parser_factory)?;

        let report = reconcile_accepting_context(&document, &existing, &ctx.glossary);

        println!(
            "{}: kept {}, added {}, dropped {}, context_hash 갱신 {}, glossary stale 유지 {}",
            file_path.display(),
            report.kept,
            report.added.len(),
            report.dropped.len(),
            report.context_accepted,
            report.glossary_stale,
        );
        for source in &report.added {
            println!("  + {source}");
        }
        for source in &report.dropped {
            println!("  - {source}");
        }

        total_kept += report.kept;
        total_added += report.added.len();
        total_dropped += report.dropped.len();
        total_context_accepted += report.context_accepted;
        total_glossary_stale += report.glossary_stale;

        if !dry_run {
            let mut new_state = StateFile::new(content_hash(&document.source));
            new_state.segments = report.segments;
            new_state.save(&state_path)?;
        }
    }

    println!(
        "\n{} file(s): kept {}, added {}, dropped {}, context_hash 갱신 {}, glossary stale 유지 {}, skipped {} (no state){}",
        files.len(),
        total_kept,
        total_added,
        total_dropped,
        total_context_accepted,
        total_glossary_stale,
        skipped,
        if dry_run { " (dry run, state not written)" } else { "" },
    );

    Ok(())
}
