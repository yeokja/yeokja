use anyhow::Result;
use std::path::Path;
use yeokja_core::glossary::{remove_term_in_file, upsert_term_in_file};
use yeokja_core::project::ProjectContext;
use yeokja_core::state::StateFile;

pub fn list() -> Result<()> {
    let ctx = ProjectContext::load()?;

    let terms = ctx.glossary.terms();
    if terms.is_empty() {
        println!("Glossary is empty.");
        return Ok(());
    }

    println!("Glossary ({} terms):", terms.len());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut sorted: Vec<_> = terms.iter().collect();
    sorted.sort_by_key(|(k, _)| k.as_str());

    for (term, translation) in sorted {
        println!("  {} → {}", term, translation);
    }

    Ok(())
}

pub fn set(term: &str, translation: &str) -> Result<()> {
    let ctx = ProjectContext::load()?;
    let glossary_path = Path::new(&ctx.config.project.glossary);
    upsert_term_in_file(glossary_path, term, translation)?;
    println!("Set: {} → {}", term, translation);
    Ok(())
}

pub fn remove(term: &str) -> Result<()> {
    let ctx = ProjectContext::load()?;
    let glossary_path = Path::new(&ctx.config.project.glossary);
    if remove_term_in_file(glossary_path, term)? {
        println!("Removed: {}", term);
    } else {
        println!("Term not found: {}", term);
    }
    Ok(())
}

/// Accept, without a model, the translations under `path` that already
/// render every glossary entry they use (see `yeokja_translate::terms::adopt_path`).
/// With `prefer`, first bring back the translations the state held at that
/// git revision wherever they meet the glossary as well.
pub fn adopt(path: &str, prefer: Option<&str>) -> Result<()> {
    let ctx = ProjectContext::load()?;
    let earlier = |state_path: &Path| -> Option<StateFile> {
        let rev = prefer?;
        let relative = state_path.strip_prefix(".").unwrap_or(state_path);
        let out = std::process::Command::new("git").args(["show", &format!("{rev}:./{}", relative.display())]).output().ok()?;
        out.status.success().then(|| serde_json::from_slice(&out.stdout).ok()).flatten()
    };
    let earlier: Option<yeokja_translate::terms::EarlierState> = prefer.map(|_| &earlier as _);
    let report = yeokja_translate::terms::adopt_path(Path::new(path), &ctx.config, &ctx.glossary, &super::parser_factory(), earlier)?;
    super::terms::print_adopt_report(&report);
    Ok(())
}
