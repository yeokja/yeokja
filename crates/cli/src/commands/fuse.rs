use anyhow::Result;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use yeokja_core::project::ProjectContext;
use yeokja_core::state::StateFile;
use yeokja_translate::factory::{create_evaluator_provider, create_llm_provider, create_provider};
use yeokja_translate::fuse::EDITOR_SYSTEM_PROMPT;
use yeokja_translate::orchestrator::{CancelToken, Orchestrator, PreviousTranslations, TranslateOptions};

/// The translations `file`'s state held at git revision `rev`.
fn previous_at(rev: &str, file: &Path, state_dir: Option<&Path>) -> Option<PreviousTranslations> {
    let state_path = StateFile::state_file_path(file, state_dir);
    let spec = format!("{rev}:./{}", state_path.strip_prefix(".").unwrap_or(&state_path).display());
    let out = Command::new("git").args(["show", &spec]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let state: StateFile = serde_json::from_slice(&out.stdout).ok()?;
    Some(
        state
            .segments
            .into_iter()
            .filter_map(|seg| Some((seg.id.0, (seg.source_hash, seg.translation?))))
            .collect(),
    )
}

/// Rewrite the translations under `path` with the editor (the project's
/// provider), using the current and the `previous` revision's translations
/// as drafts.
pub async fn run(path: &str, previous: &str, skip_after: Option<&str>) -> Result<()> {
    let skip_after = skip_after
        .map(|t| chrono::DateTime::parse_from_rfc3339(t).map(|t| t.with_timezone(&chrono::Utc)))
        .transpose()?;
    let ctx = ProjectContext::load()?;
    let options = TranslateOptions::from_config(&ctx.config);
    let editor = create_llm_provider(&ctx.config.provider, EDITOR_SYSTEM_PROMPT)?;
    let eval_provider = if options.auto_evaluate && options.style_evaluate {
        create_evaluator_provider(&ctx.config.provider)?
    } else {
        None
    };
    let state_dir = ctx.config.state_dir().map(Path::to_path_buf);
    let orchestrator = Orchestrator {
        provider: create_provider(&ctx.config.provider)?,
        config: Arc::new(ctx.config),
        glossary: Arc::new(ctx.glossary),
        eval_provider,
        parser_factory: super::parser_factory(),
        options,
        cancel: CancelToken::default(),
    };
    let lookup = |file: &Path| previous_at(previous, file, state_dir.as_deref());
    let outcome = orchestrator.fuse_path(Path::new(path), editor, &lookup, skip_after).await?;
    tracing::info!(
        files = outcome.files_processed,
        changed = outcome.segments_translated,
        failed = outcome.files_failed,
        "Fusion done"
    );
    if outcome.files_failed > 0 {
        anyhow::bail!("{} file(s) failed to fuse", outcome.files_failed);
    }
    Ok(())
}
