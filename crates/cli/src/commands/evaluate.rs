use anyhow::Result;
use std::path::Path;
use yeokja_core::change::SegmentStatus;
use yeokja_core::project::ProjectContext;
use yeokja_translate::evaluator::{EvaluationContext, IssueSeverity};
use yeokja_translate::factory::create_evaluator_provider;
use yeokja_translate::inline::audit::Audit;
use yeokja_translate::orchestrator::{audit_file, collect_files, evaluators_for, inline_tags_for, scan_file};

/// Audit the inline markup of stored translations (see `inline::audit`),
/// printing every finding; with `repair`, write the confirmed repairs to the
/// state files. `translated_at` is kept: a repair is not a translation.
pub fn audit(path: &str, repair: bool) -> Result<()> {
    let ctx = ProjectContext::load()?;
    let parser_factory = super::parser_factory();
    let files = collect_files(Path::new(path), &ctx.config)?;
    let (mut repaired, mut unrepairable) = (0usize, 0usize);
    for file_path in &files {
        let state_path = yeokja_core::state::StateFile::state_file_path(file_path, ctx.config.state_dir());
        if !state_path.exists() {
            continue;
        }
        let findings = audit_file(file_path, &ctx.config, &ctx.glossary, &parser_factory)?;
        let mut fixes = std::collections::HashMap::new();
        for finding in findings {
            match &finding.audit {
                Audit::Repaired { markdown, defects } => {
                    repaired += 1;
                    println!("{} {} REPAIRED ({})", file_path.display(), finding.segment, defects.join(", "));
                    println!("  - {}", finding.translation);
                    println!("  + {markdown}");
                    fixes.insert(finding.segment.clone(), markdown.clone());
                }
                Audit::Unrepairable { defects, reasons } => {
                    unrepairable += 1;
                    println!(
                        "{} {} UNREPAIRABLE ({}): {}",
                        file_path.display(),
                        finding.segment,
                        defects.join(", "),
                        reasons.join("; ")
                    );
                    println!("  = {}", finding.translation);
                }
                Audit::Sound => {}
            }
        }
        if repair && !fixes.is_empty() {
            let mut state = yeokja_core::state::StateFile::load(&state_path)?;
            for segment in &mut state.segments {
                if let Some(markdown) = fixes.get(&segment.id.0) {
                    segment.translation = Some(markdown.clone());
                }
            }
            state.save(&state_path)?;
        }
    }
    println!(
        "\nAudit complete: {repaired} repairable{}, {unrepairable} unrepairable",
        if repair { " (written)" } else { "" }
    );
    Ok(())
}

pub async fn run(path: &str, mechanical_only: bool) -> Result<()> {
    let ctx = ProjectContext::load()?;
    let parser_factory = super::parser_factory();

    let eval_provider = if mechanical_only {
        None
    } else {
        create_evaluator_provider(&ctx.config.provider)?
    };
    let files = collect_files(Path::new(path), &ctx.config)?;

    // A file whose source uses inline tags gets the tag-mode format check: a
    // split emphasis is no defect there.
    let evaluators = evaluators_for(eval_provider.clone(), &ctx.config.project.target_lang, false);
    let inline_evaluators = evaluators_for(eval_provider, &ctx.config.project.target_lang, true);

    let mut total_segments = 0usize;
    let mut total_issues = 0usize;

    tracing::info!(files = files.len(), "Evaluating translations");

    for file_path in &files {
        let state_path =
            yeokja_core::state::StateFile::state_file_path(file_path, ctx.config.state_dir());
        if !state_path.exists() {
            tracing::debug!(file = %file_path.display(), "No state file, skipping");
            continue;
        }

        let (_, reconciled) = scan_file(file_path, &ctx.config, &ctx.glossary, &parser_factory)?;
        let markup = parser_factory(file_path, &ctx.config).markup();
        let file_evaluators =
            if inline_tags_for(&ctx.config, file_path) { &inline_evaluators } else { &evaluators };

        let mut file_issues = 0usize;

        for rs in &reconciled {
            // Only evaluate translated segments
            if !matches!(rs.status, SegmentStatus::Translated) {
                continue;
            }

            let translation = match &rs.state.translation {
                Some(t) => t,
                None => continue,
            };

            total_segments += 1;

            let eval_ctx = EvaluationContext {
                source: rs.state.source.clone(),
                translation: translation.clone(),
                glossary: ctx.glossary.terms().clone(),
                source_lang: ctx.config.project.source_lang.clone(),
                target_lang: ctx.config.project.target_lang.clone(),
                markup,
            };

            for evaluator in file_evaluators {
                match evaluator.evaluate(&eval_ctx).await {
                    Ok(result) => {
                        for issue in &result.issues {
                            file_issues += 1;
                            let level = match issue.severity {
                                IssueSeverity::Error => "ERROR",
                                IssueSeverity::Warning => "WARN",
                            };
                            tracing::info!(
                                file = %file_path.display(),
                                segment = %rs.state.id,
                                evaluator = evaluator.name(),
                                level,
                                "{}", issue.message
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            evaluator = evaluator.name(),
                            error = %e,
                            "Evaluator failed"
                        );
                    }
                }
            }
        }

        // A slipped batch leaves translations that are each fine on their own,
        // so the per-segment evaluators above cannot see it. Compare every
        // stored translation with its neighbours' sources instead.
        let stored: Vec<(String, (String, String))> = reconciled
            .iter()
            .filter(|rs| matches!(rs.status, SegmentStatus::Translated))
            .filter_map(|rs| {
                let translation = rs.state.translation.clone()?;
                Some((rs.state.id.to_string(), (rs.state.source.clone(), translation)))
            })
            .collect();
        let pairs: Vec<(String, String)> = stored.iter().map(|(_, pair)| pair.clone()).collect();
        // Segment ids are `section:S/block:B/seg:N`; the paragraph is S/B.
        let paragraphs: Vec<String> = stored
            .iter()
            .map(|(id, _)| id.rsplit_once('/').map_or(id.as_str(), |(paragraph, _)| paragraph).to_string())
            .collect();
        for (position, from, anchors) in
            yeokja_translate::alignment::misaligned_in_sequence(&pairs, &paragraphs, markup, 3)
        {
            file_issues += 1;
            tracing::info!(
                file = %file_path.display(),
                segment = %stored[position].0,
                evaluator = "Alignment",
                level = "ERROR",
                "Translation carries {} from segment {}; the batch numbering probably slipped",
                anchors.join(", "),
                stored[from].0
            );
        }

        if file_issues > 0 {
            tracing::info!(file = %file_path.display(), issues = file_issues, "Issues found");
        } else {
            tracing::info!(file = %file_path.display(), "No issues");
        }

        total_issues += file_issues;
    }

    println!("\nEvaluation complete: {} segments checked, {} issues found", total_segments, total_issues);
    Ok(())
}
