use anyhow::Result;
use std::path::Path;
use yeokja_core::glossary::{Glossary, TermsFile};
use yeokja_core::project::ProjectContext;
use yeokja_translate::factory::create_llm_provider;
use yeokja_translate::terms::{
    AdoptReport, DecideOptions, Known, TERMS_SYSTEM_PROMPT, adopt_path, collect_corpus, decide, mine,
};

/// Print what adoption did and which entries still leave segments to translate.
pub fn print_adopt_report(report: &AdoptReport) {
    println!(
        "Accepted {} existing translation(s) in {} file(s); {} segment(s) still need translating for the glossary.",
        report.adopted, report.files_written, report.stale
    );
    let mut by_count: Vec<(&String, &usize)> = report.stale_by_term.iter().collect();
    by_count.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (term, count) in by_count.into_iter().take(30) {
        println!("  {count:>6}  {term}");
    }
}

/// Mine the project for recurring terms and segments, have the model screen
/// and decide them, record the decisions in the term base, and accept the
/// translations that already follow them.
pub async fn build(dry_run: bool, min_count: usize, limit: usize) -> Result<()> {
    let ctx = ProjectContext::load()?;
    let factory = super::parser_factory();
    let terms_path = Path::new(&ctx.config.project.terms);
    let mut file = if terms_path.exists() { TermsFile::load(terms_path)? } else { TermsFile::default() };

    let corpus = collect_corpus(&ctx.config, &ctx.glossary, &factory)?;
    let (mut known_terms, mut known_whole) = ctx.glossary.known_keys();
    let (decided_terms, decided_whole) = file.decided_keys();
    known_terms.extend(decided_terms);
    known_whole.extend(decided_whole);
    let candidates = mine(&corpus, &Known { terms: known_terms, whole: known_whole }, min_count, limit);
    println!(
        "{} file(s), {} segment(s): {} term candidate(s), {} recurring segment(s) translated inconsistently.",
        corpus.files.len(),
        corpus.segments.len(),
        candidates.terms.len(),
        candidates.whole.len()
    );

    if dry_run {
        for term in &candidates.terms {
            println!("  term   {:>6} {:>4}f  {}", term.count, term.files, term.term);
        }
        for whole in &candidates.whole {
            let variants: Vec<String> = whole.variants.iter().map(|(t, n)| format!("{t} ×{n}")).collect();
            println!("  whole  {:>6}        {}  →  {}", whole.count, whole.text, variants.join(" | "));
        }
        return Ok(());
    }

    let llm = create_llm_provider(&ctx.config.provider, TERMS_SYSTEM_PROMPT)?;
    let options = DecideOptions {
        source_lang: ctx.config.project.source_lang.clone(),
        target_lang: ctx.config.project.target_lang.clone(),
        concurrency: ctx.config.translation.as_ref().map_or(yeokja_core::config::default_concurrency(), |t| t.concurrency),
    };
    let outcome = decide(llm, &corpus, candidates, &mut file, &ctx.glossary, &options).await;
    file.save(terms_path)?;
    println!(
        "Screened {} term(s), {} picked; added {} term(s) and {} segment entr(ies); {} rejected; {} batch(es) failed.",
        outcome.screened, outcome.picked, outcome.terms_added, outcome.whole_added, outcome.rejected, outcome.failed_batches
    );

    let glossary = Glossary::load_project(Path::new("."), &ctx.config.project)?;
    let report = adopt_path(Path::new("."), &ctx.config, &glossary, &factory)?;
    print_adopt_report(&report);
    if outcome.failed_batches > 0 {
        anyhow::bail!("{} batch(es) failed; run again to ask about them", outcome.failed_batches);
    }
    Ok(())
}
