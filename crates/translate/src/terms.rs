//! The project term base: decide once, for the whole project, how recurring
//! terms and recurring short segments are translated.
//!
//! A file is translated as many requests at once, and no request sees what
//! another chose: provider became "프로바이더" in one numbered list item and
//! "제공자" in the next, and Abstract is "초록", "개요" or "요약" depending on
//! the PEP. This module finds such candidates without a model ([`mine`]),
//! asks the model which are terminology and how to render them
//! ([`decide`]), and accepts the translations that already use the decided
//! rendering ([`adopt_path`]) so only the others are translated again. The
//! decisions live in the term base file (`TermsFile`) and reach every
//! request and evaluation through the glossary.
//!
//! See `docs/superpowers/specs/2026-10-06-project-term-base-design.md`.

use crate::evaluator_glossary::glossary_source;
use crate::orchestrator::{OrchestratorError, ParserFactory, collect_files, scan_file};
use crate::provider::{CompletionRequest, LlmProvider};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;
use yeokja_core::config::ProjectConfig;
use yeokja_core::glossary::{Glossary, GlossaryEntry, TermsFile, normalize_whole};
use yeokja_core::state::StateFile;

/// The system prompt for CLI-backed providers.
pub const TERMS_SYSTEM_PROMPT: &str = "You are a terminologist for a technical documentation translation \
project. Answer exactly in the format requested, with nothing else.";

/// One translatable segment of the project, with its current translation
/// when the saved one belongs to this source text.
#[derive(Debug, Clone)]
pub struct Occurrence {
    /// Index into [`Corpus::files`].
    pub file: usize,
    pub source: String,
    pub translation: Option<String>,
    /// A word or phrase the LaTeX parser lifted out of `\emph{...}` and the
    /// like to translate on its own (a block without a source span). It recurs
    /// like a heading but is not one: "not", "must", "if".
    pub fragment: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Corpus {
    pub files: Vec<PathBuf>,
    pub segments: Vec<Occurrence>,
}

/// Every translatable segment of every file under the project's sources.
/// Untranslated files count too: a term base is for the translations still to
/// come as much as for the ones made. A file that fails to parse is skipped
/// with a warning.
pub fn collect_corpus(config: &ProjectConfig, glossary: &Glossary, factory: &ParserFactory) -> Result<Corpus, OrchestratorError> {
    let mut corpus = Corpus::default();
    for file in collect_files(Path::new("."), config)? {
        let (doc, reconciled) = match scan_file(&file, config, glossary, factory) {
            Ok(scanned) => scanned,
            Err(error) => {
                tracing::warn!(file = %file.display(), %error, "Skipped in the term search");
                continue;
            }
        };
        let index = corpus.files.len();
        let latex = factory(&file, config).markup() == yeokja_core::parser::Markup::Latex;
        // `reconciled` follows `Document::translatable_segments`: every
        // segment of every translatable block, in order.
        let segments = doc
            .sections
            .iter()
            .flat_map(|section| &section.blocks)
            .filter(|block| block.translatable)
            .flat_map(|block| block.segments.iter().map(move |segment| (segment, latex && block.span.is_none())));
        for ((segment, fragment), rs) in segments.zip(&reconciled) {
            let translation = rs.state.translation.clone().filter(|_| rs.state.source_hash == segment.source_hash);
            corpus.segments.push(Occurrence { file: index, source: segment.source.clone(), translation, fragment });
        }
        corpus.files.push(file);
    }
    Ok(corpus)
}

// --- Mining ---

/// English function words: an n-gram that starts or ends with one is a piece
/// of a sentence, not a term ("the provider", "provider is").
const STOPWORDS: &[&str] = &[
    "a", "about", "above", "after", "again", "against", "all", "also", "am", "an", "and", "any", "are", "as", "at",
    "be", "because", "been", "before", "being", "below", "between", "both", "but", "by", "can", "could", "did", "do",
    "does", "doing", "down", "during", "each", "either", "else", "etc", "even", "ever", "every", "few", "for", "from",
    "further", "had", "has", "have", "having", "he", "her", "here", "hers", "him", "his", "how", "however", "i", "if",
    "in", "into", "is", "it", "its", "itself", "just", "let", "may", "me", "might", "more", "most", "much", "must",
    "my", "neither", "no", "nor", "not", "now", "of", "off", "on", "once", "one", "only", "or", "other", "our", "ours",
    "out", "over", "own", "per", "rather", "same", "shall", "she", "should", "since", "so", "some", "such", "than",
    "that", "the", "their", "theirs", "them", "then", "there", "these", "they", "this", "those", "though", "through",
    "thus", "to", "too", "under", "until", "up", "upon", "us", "very", "via", "was", "we", "were", "what", "when",
    "where", "whether", "which", "while", "who", "whom", "whose", "why", "will", "with", "within", "without", "would",
    "yet", "you", "your", "yours", "it's", "don't", "doesn't", "isn't", "can't", "won't", "we're", "you're", "that's",
    "there's", "let's", "i'm", "we'll", "you'll", "e.g", "i.e",
];

/// The longest n-gram counted.
const MAX_WORDS: usize = 3;
/// A longer n-gram that holds at least this share of a shorter one's
/// occurrences stands for it: "ray class group" for "ray class".
const CLOSED_SHARE: f64 = 0.9;
/// Whole-segment candidates: at most this many words.
const WHOLE_MAX_WORDS: usize = 6;
/// Whole-segment candidates: at least this many occurrences.
const WHOLE_MIN_COUNT: usize = 3;
/// Examples shown per term.
const EXAMPLES: usize = 5;

/// Runs of prose words in `source`, with markup that is not prose removed:
/// code spans, math, HTML tags, URLs, link targets, reST roles and LaTeX
/// commands. Sentence punctuation, digits and one-letter words end a run, so
/// no n-gram crosses them.
pub fn prose_chunks(source: &str) -> Vec<Vec<String>> {
    let chars: Vec<char> = source.chars().collect();
    let mut chunks: Vec<Vec<String>> = Vec::new();
    let mut words: Vec<String> = Vec::new();
    let mut word = String::new();

    fn end_word(word: &mut String, words: &mut Vec<String>, chunks: &mut Vec<Vec<String>>) {
        let trimmed = word.trim_matches(|c: char| c == '-' || c == '\'' || c == '’');
        let token = trimmed.replace('’', "'");
        if token.chars().count() >= 2 && !token.chars().any(|c| c.is_ascii_digit()) {
            words.push(token);
        } else if !token.is_empty() {
            end_chunk(words, chunks);
        }
        word.clear();
    }
    fn end_chunk(words: &mut Vec<String>, chunks: &mut Vec<Vec<String>>) {
        if !words.is_empty() {
            chunks.push(std::mem::take(words));
        }
    }

    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        // Verbatim runs: `code`, ``code``, $math$, $$math$$.
        if c == '`' || c == '$' {
            let run = chars[at..].iter().take_while(|&&d| d == c).count();
            let close = (at + run..chars.len().saturating_sub(run - 1))
                .find(|&i| chars[i..i + run].iter().all(|&d| d == c));
            end_word(&mut word, &mut words, &mut chunks);
            end_chunk(&mut words, &mut chunks);
            at = close.map_or(at + run, |i| i + run);
            continue;
        }
        // <tag>, <https://autolink>.
        if c == '<'
            && let Some(len) = chars[at + 1..].iter().take(200).position(|&d| d == '>' || d == '\n')
            && chars[at + 1 + len] == '>'
        {
            end_word(&mut word, &mut words, &mut chunks);
            end_chunk(&mut words, &mut chunks);
            at += len + 2;
            continue;
        }
        // ](target)
        if c == ']' && chars.get(at + 1) == Some(&'(') {
            let close = chars[at..].iter().position(|&d| d == ')').map_or(chars.len(), |p| at + p + 1);
            end_word(&mut word, &mut words, &mut chunks);
            end_chunk(&mut words, &mut chunks);
            at = close;
            continue;
        }
        // URLs.
        if word.is_empty() {
            let rest: String = chars[at..chars.len().min(at + 8)].iter().collect();
            if rest.starts_with("http://") || rest.starts_with("https://") {
                end_chunk(&mut words, &mut chunks);
                while at < chars.len() && !chars[at].is_whitespace() && !matches!(chars[at], ')' | '>' | ']') {
                    at += 1;
                }
                continue;
            }
        }
        // :role:`...` — the role name goes with its code span.
        if c == ':' && word.is_empty() {
            let name = chars[at + 1..]
                .iter()
                .take_while(|d| d.is_ascii_alphanumeric() || matches!(d, ':' | '-' | '+' | '.' | '_'))
                .count();
            if name > 1 && chars[at + name] == ':' && chars.get(at + name + 1) == Some(&'`') {
                end_chunk(&mut words, &mut chunks);
                at += name + 1;
                continue;
            }
        }
        // \command — the name only; its braces end the run like punctuation.
        if c == '\\' {
            end_word(&mut word, &mut words, &mut chunks);
            end_chunk(&mut words, &mut chunks);
            at += 1;
            while at < chars.len() && chars[at].is_ascii_alphabetic() {
                at += 1;
            }
            if at < chars.len() && !chars[at].is_ascii_alphabetic() && chars[at - 1] == '\\' {
                at += 1;
            }
            continue;
        }
        if c.is_alphanumeric() || ((c == '-' || c == '\'' || c == '’') && !word.is_empty()) {
            word.push(c);
        } else if c.is_whitespace() || matches!(c, '*' | '_' | '~') {
            end_word(&mut word, &mut words, &mut chunks);
        } else {
            end_word(&mut word, &mut words, &mut chunks);
            end_chunk(&mut words, &mut chunks);
        }
        at += 1;
    }
    end_word(&mut word, &mut words, &mut chunks);
    end_chunk(&mut words, &mut chunks);
    chunks
}

/// Lowercased n-gram keys of one chunk, with their surface spelling.
fn ngrams(chunk: &[String], stopwords: &HashSet<&str>) -> Vec<(String, String)> {
    let lowered: Vec<String> = chunk.iter().map(|w| w.to_lowercase()).collect();
    let mut out = Vec::new();
    for n in 1..=MAX_WORDS {
        for start in 0..chunk.len().saturating_sub(n - 1) {
            let first = &lowered[start];
            let last = &lowered[start + n - 1];
            if stopwords.contains(first.as_str()) || stopwords.contains(last.as_str()) {
                continue;
            }
            if n == 1 && first.chars().count() < 3 {
                continue;
            }
            out.push((lowered[start..start + n].join(" "), chunk[start..start + n].join(" ")));
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct TermCandidate {
    /// The spelling used most often.
    pub term: String,
    /// Its plural spellings that occur too ("processes" for "process"). The
    /// glossary matches whole words, so a decision is recorded for each.
    pub forms: Vec<String>,
    pub count: usize,
    pub files: usize,
    /// Indices into [`Corpus::segments`], from different files first and
    /// translated ones first.
    pub examples: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WholeCandidate {
    /// The segment as written most often.
    pub text: String,
    /// [`normalize_whole`] of it.
    pub key: String,
    pub count: usize,
    /// Current translations, as written most often per normalized variant,
    /// with their counts, most frequent first.
    pub variants: Vec<(String, usize)>,
}

#[derive(Debug, Clone, Default)]
pub struct Candidates {
    pub terms: Vec<TermCandidate>,
    pub whole: Vec<WholeCandidate>,
}

/// What `mine` should not propose: keys already in a glossary or decided in
/// the term base (lowercased terms, [`normalize_whole`] keys).
#[derive(Debug, Clone, Default)]
pub struct Known {
    pub terms: HashSet<String>,
    pub whole: HashSet<String>,
}

/// Candidates for the term base, without a model.
///
/// Terms: 1–3 word n-grams of prose that occur at least `min_count` times,
/// minus those a longer n-gram stands for, most frequent first, at most
/// `limit`. Whole segments: short segments that recur at least three times
/// and whose current translations disagree.
pub fn mine(corpus: &Corpus, known: &Known, min_count: usize, limit: usize) -> Candidates {
    let stopwords: HashSet<&str> = STOPWORDS.iter().copied().collect();
    let chunked: Vec<Vec<Vec<String>>> = corpus.segments.iter().map(|o| prose_chunks(&o.source)).collect();

    // Pass 1: counts only, so the bookkeeping below is for survivors only.
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut capitalized: HashMap<String, usize> = HashMap::new();
    for chunks in &chunked {
        for chunk in chunks {
            for (key, surface) in ngrams(chunk, &stopwords) {
                if surface.chars().next().is_some_and(char::is_uppercase) {
                    *capitalized.entry(key.clone()).or_default() += 1;
                }
                *counts.entry(key).or_default() += 1;
            }
        }
    }
    // An English plural counts towards its singular: "processes" is the
    // term "process". Only the last word inflects ("page tables").
    let mut fold: HashMap<String, String> = HashMap::new();
    // A name is not a plural: "Windows" is not more than one "window". A
    // plural nearly always capitalized whose singular nearly never is stays
    // a word of its own.
    let share = |key: &str| capitalized.get(key).copied().unwrap_or(0) as f64 / counts[key].max(1) as f64;
    for key in counts.keys() {
        if let Some(singular) = singular_of(key, &counts)
            && !(share(key) >= 0.9 && share(&singular) <= 0.1)
        {
            fold.insert(key.clone(), singular);
        }
    }
    for (plural, singular) in &fold {
        let count = counts.remove(plural).unwrap_or(0);
        *counts.entry(singular.clone()).or_default() += count;
    }
    counts.retain(|key, count| *count >= min_count && !known.terms.contains(key));

    // A longer n-gram that holds nearly all of a shorter one's occurrences
    // stands for it.
    let mut covered: HashSet<String> = HashSet::new();
    for (key, &count) in &counts {
        let words: Vec<&str> = key.split(' ').collect();
        if words.len() < 2 {
            continue;
        }
        for sub in [words[..words.len() - 1].join(" "), words[1..].join(" ")] {
            if let Some(&sub_count) = counts.get(&sub)
                && count as f64 >= CLOSED_SHARE * sub_count as f64
            {
                covered.insert(sub);
            }
        }
    }
    counts.retain(|key, _| !covered.contains(key));

    let mut ranked: Vec<(&String, &usize)> = counts.iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    let wanted: HashMap<String, usize> =
        ranked.into_iter().take(limit).enumerate().map(|(rank, (key, _))| (key.clone(), rank)).collect();

    // Pass 2: spellings, files and examples of the survivors.
    struct Stat {
        count: usize,
        files: HashSet<usize>,
        /// Key as counted (singular or a plural folded into it) → spelling → count.
        spellings: HashMap<String, HashMap<String, usize>>,
        examples: Vec<usize>,
        spare: Vec<usize>,
    }
    let mut stats: HashMap<&str, Stat> = HashMap::new();
    for (index, chunks) in chunked.iter().enumerate() {
        let occurrence = &corpus.segments[index];
        let mut seen_here: HashSet<String> = HashSet::new();
        for chunk in chunks {
            for (form, surface) in ngrams(chunk, &stopwords) {
                let key = fold.get(&form).cloned().unwrap_or_else(|| form.clone());
                let Some((wanted_key, _)) = wanted.get_key_value(&key) else { continue };
                let stat = stats.entry(wanted_key.as_str()).or_insert_with(|| Stat {
                    count: 0,
                    files: HashSet::new(),
                    spellings: HashMap::new(),
                    examples: Vec::new(),
                    spare: Vec::new(),
                });
                stat.count += 1;
                *stat.spellings.entry(form).or_default().entry(surface).or_default() += 1;
                if !seen_here.insert(key) {
                    continue;
                }
                let new_file = stat.files.insert(occurrence.file);
                if new_file && occurrence.translation.is_some() && stat.examples.len() < EXAMPLES {
                    stat.examples.push(index);
                } else if stat.spare.len() < EXAMPLES {
                    stat.spare.push(index);
                }
            }
        }
    }
    let mut terms: Vec<(usize, TermCandidate)> = stats
        .into_iter()
        .map(|(key, mut stat)| {
            for index in stat.spare.drain(..) {
                if stat.examples.len() >= EXAMPLES {
                    break;
                }
                if !stat.examples.contains(&index) {
                    stat.examples.push(index);
                }
            }
            let spelling = |form: &str| {
                stat.spellings
                    .get(form)
                    .and_then(|m| m.iter().max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0))))
                    .map(|(s, _)| s.clone())
                    .unwrap_or_else(|| form.to_string())
            };
            let term = spelling(key);
            let mut forms: Vec<String> = stat.spellings.keys().filter(|f| f.as_str() != key).map(|f| spelling(f)).collect();
            forms.sort();
            (wanted[key], TermCandidate { term, forms, count: stat.count, files: stat.files.len(), examples: stat.examples })
        })
        .collect();
    terms.sort_by_key(|(rank, _)| *rank);

    Candidates { terms: terms.into_iter().map(|(_, c)| c).collect(), whole: mine_whole(corpus, known, limit) }
}

/// The singular `key` is the plural of, when that singular occurs as well:
/// "processes" → "process", "policies" → "policy", "page tables" → "page table".
fn singular_of(key: &str, counts: &HashMap<String, usize>) -> Option<String> {
    let (head, last) = match key.rsplit_once(' ') {
        Some((head, last)) => (format!("{head} "), last),
        None => (String::new(), key),
    };
    if last.len() < 4 || last.ends_with("ss") || last.ends_with("us") || last.ends_with("is") {
        return None;
    }
    let mut stems = Vec::new();
    if let Some(stem) = last.strip_suffix("ies") {
        stems.push(format!("{stem}y"));
    }
    if let Some(stem) = last.strip_suffix("es") {
        stems.push(stem.to_string());
    }
    if let Some(stem) = last.strip_suffix('s') {
        stems.push(stem.to_string());
    }
    stems.into_iter().map(|stem| format!("{head}{stem}")).find(|singular| counts.contains_key(singular))
}

fn mine_whole(corpus: &Corpus, known: &Known, limit: usize) -> Vec<WholeCandidate> {
    struct Stat {
        count: usize,
        spellings: HashMap<String, usize>,
        /// normalized translation → (spelling → count)
        variants: HashMap<String, HashMap<String, usize>>,
    }
    let mut stats: HashMap<String, Stat> = HashMap::new();
    for occurrence in corpus.segments.iter().filter(|o| !o.fragment) {
        let source = occurrence.source.trim();
        if source.split_whitespace().count() > WHOLE_MAX_WORDS || source.chars().count() > 80 {
            continue;
        }
        if prose_chunks(source).is_empty() {
            continue;
        }
        // A heading or label starts with a capital and does not trail off
        // into the next line: "and", "commutes." and "Thus," are pieces of a
        // sentence cut by a formula or code, translated to fit around it.
        let opening = source.trim_start_matches(|c: char| !c.is_alphanumeric()).chars().next();
        if !opening.is_some_and(char::is_uppercase) || source.ends_with(',') {
            continue;
        }
        let key = normalize_whole(source);
        if key.is_empty() || known.whole.contains(&key) {
            continue;
        }
        let stat = stats.entry(key).or_insert_with(|| Stat { count: 0, spellings: HashMap::new(), variants: HashMap::new() });
        stat.count += 1;
        *stat.spellings.entry(source.to_string()).or_default() += 1;
        if let Some(translation) = &occurrence.translation {
            *stat
                .variants
                .entry(normalize_whole(translation))
                .or_default()
                .entry(translation.trim().to_string())
                .or_default() += 1;
        }
    }
    let most = |m: &HashMap<String, usize>| {
        m.iter().max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0))).map(|(s, _)| s.clone()).unwrap_or_default()
    };
    let mut whole: Vec<WholeCandidate> = stats
        .into_iter()
        .filter(|(_, stat)| stat.count >= WHOLE_MIN_COUNT && stat.variants.len() >= 2)
        .map(|(key, stat)| {
            let mut variants: Vec<(String, usize)> =
                stat.variants.values().map(|spellings| (most(spellings), spellings.values().sum())).collect();
            variants.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            WholeCandidate { text: most(&stat.spellings), key, count: stat.count, variants }
        })
        .collect();
    whole.sort_by(|a, b| b.count.cmp(&a.count).then(a.key.cmp(&b.key)));
    whole.truncate(limit);
    whole
}

// --- Prompts and replies ---

/// Terms per screening request.
pub const SCREEN_BATCH: usize = 250;
/// Entries per decision request.
pub const DECIDE_BATCH: usize = 20;
/// Characters of each example shown.
const EXAMPLE_CHARS: usize = 240;
/// The longest rendering accepted.
const MAX_RENDERING_CHARS: usize = 80;

/// "process / processes": the term with the plural spellings folded into it.
fn shown_term(term: &TermCandidate) -> String {
    std::iter::once(term.term.as_str()).chain(term.forms.iter().map(String::as_str)).collect::<Vec<_>>().join(" / ")
}

pub fn screen_prompt(source_lang: &str, target_lang: &str, terms: &[&TermCandidate]) -> String {
    let mut prompt = format!(
        "The following words and phrases recur in a documentation project being translated from \
{source_lang} to {target_lang}. Each line gives a number, the phrase, and how many times it occurs.\n\n\
Pick the ones that are terminology: a technical term, a defined concept, the name of a role, process, \
tool, product, organisation or person, or another proper noun — anything whose {target_lang} \
translation should be the same in every document.\n\
Pick nouns, noun phrases and names only. Leave out verbs, adjectives and adverbs on their own \
(\"launch\", \"separately\"), everyday words and phrases whose translation depends on the sentence \
(\"value\", \"make sure\", \"following\"), fragments of a longer term, phrases that merely contain a \
term (keep the term itself), and words that only ever name code.\n\n\
Reply with the numbers you pick, separated by commas, and nothing else. Reply \"none\" if there are \
none.\n\n"
    );
    for (i, term) in terms.iter().enumerate() {
        prompt.push_str(&format!("{}. {} ({})\n", i + 1, shown_term(term), term.count));
    }
    prompt
}

/// The numbers picked, or an error when the reply is not a list of numbers
/// (then nothing from the batch is recorded).
pub fn parse_screen_reply(reply: &str, len: usize) -> Result<HashSet<usize>, String> {
    let reply = reply.trim();
    if reply.trim_end_matches('.').eq_ignore_ascii_case("none") {
        return Ok(HashSet::new());
    }
    let mut picked = HashSet::new();
    for token in reply.split(|c: char| c == ',' || c.is_whitespace()).filter(|t| !t.is_empty()) {
        let number: usize =
            token.trim_end_matches('.').parse().map_err(|_| format!("not a list of numbers: {token:?}"))?;
        if (1..=len).contains(&number) {
            picked.insert(number);
        }
    }
    Ok(picked)
}

/// An entry of a decision request.
#[derive(Debug, Clone)]
pub enum DecideItem {
    /// A term, with the fixed renderings of its parts (term → rendering).
    Term(TermCandidate, Vec<(String, String)>),
    Whole(WholeCandidate),
}

fn window(text: &str, term: &str, chars: usize) -> String {
    let lowered = text.to_lowercase();
    let start = lowered.find(&term.to_lowercase()).map(|byte| lowered[..byte].chars().count()).unwrap_or(0);
    let total = text.chars().count();
    let from = start.saturating_sub(chars / 3).min(total.saturating_sub(chars));
    let snippet: String = text.chars().skip(from).take(chars).collect();
    let snippet = snippet.replace('\n', " ");
    format!("{}{}{}", if from > 0 { "…" } else { "" }, snippet, if from + chars < total { "…" } else { "" })
}

pub fn decide_prompt(source_lang: &str, target_lang: &str, items: &[DecideItem], corpus: &Corpus) -> String {
    let mut prompt = format!(
        "You are fixing how recurring terms are translated across a documentation project \
({source_lang} → {target_lang}), so that every document uses the same rendering. Different parts were \
translated at the same time and some terms were rendered in different ways.\n\n\
For each numbered entry, examples show sentences that contain the term and their current translations. \
Reply with one line per entry:\n[N] rendering\n\
where rendering is the {target_lang} word or phrase that every translation of a sentence containing \
the term must contain. Rules:\n\
- Prefer the rendering the current translations already use most, unless it is wrong: every \
translation that uses another rendering will be translated again.\n\
- Write the bare word or phrase as it stands inside a sentence: no particles or endings, no \
explanation, no parentheses, no alternatives.\n\
- A term shown as \"process / processes\" lists its plural spelling; one rendering covers both.\n\
- Keep a name in {source_lang} when translations should leave it as written (a product name, say).\n\
- Reply [N] - when the term should not have one fixed rendering: any example uses it in its everyday \
sense or in a different technical sense (\"translation\" of addresses and of languages), or a fixed \
rendering would force unnatural sentences. A rendering is enforced in every sentence that contains \
the term, so when in doubt, reply -.\n\
- Reply [N] - as well when an example uses the word as a name in code — a class, type, function or \
command the translations keep as written (\"the new interval class\", \"lambertian scatter()\").\n\
- Where part of a term is fixed already (shown as \"fixed already\"), render that part the same way.\n\
Entries marked SEGMENT are a whole heading or label that recurs on its own; their current \
translations are listed with counts. Give the complete translation of the segment, without a \
trailing colon. Reply [N] - for a SEGMENT that is a piece of a sentence cut off by a formula, \
code or a list (\"Then\", \"For example\") rather than a heading or label: its translation has \
to fit what follows it.\n"
    );
    // A segment entry is enforced verbatim, so it has to be in the register
    // the translations are held to (see `prompt::build_prompt`).
    if target_lang.starts_with("ko") {
        prompt.push_str(
            "Write segment translations in formal written Korean (합쇼체): statements end in \
             ~합니다/~입니다, questions in ~니까?, never in the polite ~요 register (~까요?, ~해요) \
             or the plain ~다 register. Headings that are noun phrases stay noun phrases.\n",
        );
    }
    prompt.push('\n');
    for (i, item) in items.iter().enumerate() {
        match item {
            DecideItem::Term(term, fixed) => {
                prompt.push_str(&format!("[{}] {} ({} occurrences)\n", i + 1, shown_term(term), term.count));
                if !fixed.is_empty() {
                    let fixed: Vec<String> = fixed.iter().map(|(t, r)| format!("{t} → {r}")).collect();
                    prompt.push_str(&format!("  fixed already: {}\n", fixed.join("; ")));
                }
                for &index in &term.examples {
                    let occurrence = &corpus.segments[index];
                    prompt.push_str(&format!("  - {}: {}\n", source_lang, window(&occurrence.source, &term.term, EXAMPLE_CHARS)));
                    match &occurrence.translation {
                        Some(translation) => {
                            let shown: String = translation.chars().take(EXAMPLE_CHARS).collect();
                            prompt.push_str(&format!("    {}: {}\n", target_lang, shown.replace('\n', " ")));
                        }
                        None => prompt.push_str(&format!("    {target_lang}: (not translated yet)\n")),
                    }
                }
            }
            DecideItem::Whole(whole) => {
                let variants: Vec<String> =
                    whole.variants.iter().map(|(text, count)| format!("\"{text}\" ×{count}")).collect();
                prompt.push_str(&format!(
                    "[{}] SEGMENT \"{}\" ({} occurrences)\n  current translations: {}\n",
                    i + 1,
                    whole.text,
                    whole.count,
                    variants.join(", ")
                ));
            }
        }
    }
    prompt
}

#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    Render(String),
    Reject,
}

/// Decisions by entry number. An entry that is missing or whose rendering
/// is unusable (empty, too long, bracketed, an alternative) is left out:
/// nothing is recorded for it and the next run asks again.
pub fn parse_decide_reply(reply: &str, len: usize) -> Result<HashMap<usize, Decision>, String> {
    let parsed = crate::prompt::parse_response(reply)?;
    let mut decisions = HashMap::new();
    for (number, text) in parsed {
        if !(1..=len).contains(&number) {
            continue;
        }
        let text = text.trim().trim_matches(|c| c == '"' || c == '“' || c == '”').trim();
        if matches!(text, "-" | "—" | "–") {
            decisions.insert(number, Decision::Reject);
            continue;
        }
        let usable = !text.is_empty()
            && text.chars().count() <= MAX_RENDERING_CHARS
            && !text.contains(['[', ']', '(', ')', '/', '→', '\n'])
            && !text.contains(" or ");
        if usable {
            decisions.insert(number, Decision::Render(text.trim_end_matches([':', '：']).trim().to_string()));
        } else {
            tracing::warn!(entry = number, rendering = text, "Unusable rendering; asked again next run");
        }
    }
    Ok(decisions)
}

// --- Running ---

#[derive(Debug, Clone)]
pub struct DecideOptions {
    pub source_lang: String,
    pub target_lang: String,
    pub concurrency: usize,
}

#[derive(Debug, Clone, Default)]
pub struct DecideOutcome {
    pub screened: usize,
    pub picked: usize,
    pub terms_added: usize,
    pub whole_added: usize,
    pub rejected: usize,
    pub failed_batches: usize,
}

async fn ask_all(llm: &Arc<dyn LlmProvider>, prompts: Vec<String>, concurrency: usize) -> Vec<Option<String>> {
    let semaphore = Arc::new(Semaphore::new(concurrency.max(1)));
    let handles: Vec<_> = prompts
        .into_iter()
        .map(|prompt| {
            let llm = llm.clone();
            let semaphore = semaphore.clone();
            tokio::spawn(async move {
                let _permit = semaphore.acquire().await.ok()?;
                match llm.complete(CompletionRequest { prompt }).await {
                    Ok(response) => Some(response.text),
                    Err(error) => {
                        tracing::warn!(%error, "Term base request failed");
                        None
                    }
                }
            })
        })
        .collect();
    let mut replies = Vec::new();
    for handle in handles {
        replies.push(handle.await.ok().flatten());
    }
    replies
}

/// Screen the term candidates, then decide in two waves: whole segments and
/// one-word terms first, then longer terms with the renderings their parts
/// were given (here or in `base`), so "fetch-execute cycle" agrees with
/// "fetch-execute". Every decision is recorded in `file`. A batch that fails
/// or answers out of format records nothing.
pub async fn decide(
    llm: Arc<dyn LlmProvider>,
    corpus: &Corpus,
    candidates: Candidates,
    file: &mut TermsFile,
    base: &Glossary,
    options: &DecideOptions,
) -> DecideOutcome {
    let mut outcome = DecideOutcome { screened: candidates.terms.len(), ..Default::default() };

    let batches: Vec<Vec<&TermCandidate>> =
        candidates.terms.chunks(SCREEN_BATCH).map(|chunk| chunk.iter().collect()).collect();
    let prompts =
        batches.iter().map(|batch| screen_prompt(&options.source_lang, &options.target_lang, batch)).collect();
    let mut picked_terms: Vec<TermCandidate> = Vec::new();
    for (batch, reply) in batches.iter().zip(ask_all(&llm, prompts, options.concurrency).await) {
        let picked = match reply.as_deref().map(|r| parse_screen_reply(r, batch.len())) {
            Some(Ok(picked)) => picked,
            Some(Err(error)) => {
                tracing::warn!(%error, "Screening reply out of format; batch asked again next run");
                outcome.failed_batches += 1;
                continue;
            }
            None => {
                outcome.failed_batches += 1;
                continue;
            }
        };
        for (i, term) in batch.iter().enumerate() {
            if picked.contains(&(i + 1)) {
                picked_terms.push((*term).clone());
            } else {
                for form in std::iter::once(&term.term).chain(&term.forms) {
                    file.rejected.terms.insert(form.to_lowercase());
                }
                outcome.rejected += 1;
            }
        }
    }
    outcome.picked = picked_terms.len();

    // A term that is also a recurring segment is a heading or label: the
    // segment entry governs it there. As a term it would also govern prose,
    // with the heading's rendering — "Abstract → 초록" demanded inside
    // "abstract base class" across the PEPs — so it is not fixed as a term.
    let whole_keys: HashSet<&str> = candidates.whole.iter().map(|w| w.key.as_str()).collect();
    picked_terms.retain(|term| {
        if !whole_keys.contains(normalize_whole(&term.term).as_str()) {
            return true;
        }
        for form in std::iter::once(&term.term).chain(&term.forms) {
            file.rejected.terms.insert(form.to_lowercase());
        }
        outcome.rejected += 1;
        false
    });

    // Alphabetical order keeps related terms in one request.
    picked_terms.sort_by_key(|term| term.term.to_lowercase());
    let (single, compound): (Vec<TermCandidate>, Vec<TermCandidate>) =
        picked_terms.into_iter().partition(|term| !term.term.contains(' '));

    let mut first: Vec<DecideItem> = candidates.whole.into_iter().map(DecideItem::Whole).collect();
    first.extend(single.into_iter().map(|term| DecideItem::Term(term, Vec::new())));
    decide_wave(&llm, corpus, first, file, options, &mut outcome).await;
    let mut fixed: HashMap<String, String> = base.terms().clone();
    fixed.extend(file.terms.iter().map(|(term, entry)| (term.clone(), entry.translation.clone())));
    let fixed: Glossary = fixed.into_iter().collect();
    let second = compound
        .into_iter()
        .map(|term| {
            let mut parts: Vec<(String, String)> = fixed.find_matching_terms(&term.term).into_iter().collect();
            parts.sort();
            DecideItem::Term(term, parts)
        })
        .collect();
    decide_wave(&llm, corpus, second, file, options, &mut outcome).await;
    outcome
}

async fn decide_wave(
    llm: &Arc<dyn LlmProvider>,
    corpus: &Corpus,
    items: Vec<DecideItem>,
    file: &mut TermsFile,
    options: &DecideOptions,
    outcome: &mut DecideOutcome,
) {
    let batches: Vec<&[DecideItem]> = items.chunks(DECIDE_BATCH).collect();
    let prompts =
        batches.iter().map(|batch| decide_prompt(&options.source_lang, &options.target_lang, batch, corpus)).collect();
    for (batch, reply) in batches.iter().zip(ask_all(llm, prompts, options.concurrency).await) {
        let decisions = match reply.as_deref().map(|r| parse_decide_reply(r, batch.len())) {
            Some(Ok(decisions)) => decisions,
            Some(Err(error)) => {
                tracing::warn!(%error, "Decision reply out of format; batch asked again next run");
                outcome.failed_batches += 1;
                continue;
            }
            None => {
                outcome.failed_batches += 1;
                continue;
            }
        };
        for (i, item) in batch.iter().enumerate() {
            let entry = |translation: &str| GlossaryEntry { translation: translation.to_string(), note: None, except: vec![] };
            match (item, decisions.get(&(i + 1))) {
                (_, None) => {}
                (DecideItem::Term(term, _), Some(Decision::Render(rendering))) => {
                    for form in std::iter::once(&term.term).chain(&term.forms) {
                        file.terms.insert(form.clone(), entry(rendering));
                    }
                    outcome.terms_added += 1;
                }
                (DecideItem::Term(term, _), Some(Decision::Reject)) => {
                    for form in std::iter::once(&term.term).chain(&term.forms) {
                        file.rejected.terms.insert(form.to_lowercase());
                    }
                    outcome.rejected += 1;
                }
                (DecideItem::Whole(whole), Some(Decision::Render(rendering))) => {
                    file.whole.insert(whole.text.clone(), entry(rendering));
                    outcome.whole_added += 1;
                }
                (DecideItem::Whole(whole), Some(Decision::Reject)) => {
                    file.rejected.whole.insert(whole.key.clone());
                    outcome.rejected += 1;
                }
            }
        }
    }
}

// --- Adoption ---

/// Whether a stored issue is the glossary evaluator's (see `evaluator_glossary`).
fn is_glossary_issue(issue: &str) -> bool {
    (issue.starts_with("Term '") || issue.starts_with("Segment '")) && issue.contains("' should be translated as ")
}

/// What [`adopt_state`] did to one state file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Adopted {
    /// Snapshots brought up to date.
    pub adopted: usize,
    /// Translations whose stored glossary issues no longer hold and were dropped.
    pub cleared: usize,
    /// For each segment left glossary-stale, the entries it does not render.
    pub left: Vec<Vec<String>>,
}

/// Accept the translations in `state` that already render every glossary
/// entry their source uses: their snapshot is brought up to date, so they
/// are not translated again. A translation that meets the glossary also loses
/// any glossary issue stored with it — a translation kept after its retries
/// ran out records why, and the record goes stale once the glossary or its
/// matching changes.
pub fn adopt_state(state: &mut StateFile, glossary: &Glossary, markup: yeokja_core::parser::Markup) -> Adopted {
    let mut out = Adopted::default();
    for segment in &mut state.segments {
        let Some(translation) = &segment.translation else { continue };
        let stale = glossary.is_snapshot_stale(&segment.glossary_snapshot, &segment.source);
        let flagged = segment.issues.iter().any(|issue| is_glossary_issue(issue));
        if !stale && !flagged {
            continue;
        }
        let unmet = glossary.unmet_terms(&glossary_source(markup, &segment.source), translation);
        if !unmet.is_empty() {
            if stale {
                out.left.push(unmet.into_iter().map(|(term, _)| term).collect());
            }
            continue;
        }
        if stale {
            segment.glossary_snapshot = glossary.find_matching_terms(&segment.source);
            out.adopted += 1;
        }
        if flagged {
            segment.issues.retain(|issue| !is_glossary_issue(issue));
            out.cleared += 1;
        }
    }
    out
}

/// Bring back the translations `earlier` held where they are as good for the
/// glossary as the current ones: same segment, same source text, and every
/// entry rendered. Undoes translating again for a match the glossary should
/// never have made (a fixed matcher bug), and keeps the earlier wording and
/// its `translated_at`. Returns how many were restored.
pub fn restore_earlier(
    state: &mut StateFile,
    earlier: &StateFile,
    glossary: &Glossary,
    markup: yeokja_core::parser::Markup,
) -> usize {
    let by_id: HashMap<&str, &yeokja_core::state::SegmentState> =
        earlier.segments.iter().map(|segment| (segment.id.0.as_str(), segment)).collect();
    let mut restored = 0;
    for segment in &mut state.segments {
        let Some(old) = by_id.get(segment.id.0.as_str()) else { continue };
        let Some(old_translation) = &old.translation else { continue };
        if old.source_hash != segment.source_hash || segment.translation.as_ref() == Some(old_translation) {
            continue;
        }
        if glossary.unmet_terms(&glossary_source(markup, &segment.source), old_translation).is_empty() {
            segment.translation = Some(old_translation.clone());
            segment.translated_at = old.translated_at;
            segment.issues = old.issues.clone();
            segment.glossary_snapshot = glossary.find_matching_terms(&segment.source);
            restored += 1;
        }
    }
    restored
}

#[derive(Debug, Clone, Default)]
pub struct AdoptReport {
    pub adopted: usize,
    /// Earlier translations brought back (see [`restore_earlier`]).
    pub restored: usize,
    /// Translations whose stale glossary issues were dropped.
    pub cleared: usize,
    pub files_written: usize,
    /// Segments still glossary-stale.
    pub stale: usize,
    /// Entry → segments still glossary-stale for not rendering it.
    pub stale_by_term: BTreeMap<String, usize>,
}

/// A file's earlier state, looked up by its state path.
pub type EarlierState<'a> = &'a dyn Fn(&Path) -> Option<StateFile>;

/// [`adopt_state`] over every file under `path` that has saved state, after
/// [`restore_earlier`] where `earlier` gives a file's earlier state.
pub fn adopt_path(
    path: &Path,
    config: &ProjectConfig,
    glossary: &Glossary,
    factory: &ParserFactory,
    earlier: Option<EarlierState<'_>>,
) -> Result<AdoptReport, OrchestratorError> {
    let mut report = AdoptReport::default();
    for file in collect_files(path, config)? {
        let state_path = StateFile::state_file_path(&file, config.state_dir());
        if !state_path.exists() {
            continue;
        }
        let mut state = StateFile::load(&state_path)?;
        let markup = factory(&file, config).markup();
        let restored = earlier
            .and_then(|earlier| earlier(&state_path))
            .map_or(0, |old| restore_earlier(&mut state, &old, glossary, markup));
        let Adopted { adopted, cleared, left } = adopt_state(&mut state, glossary, markup);
        if adopted + restored + cleared > 0 {
            state.save(&state_path)?;
            report.files_written += 1;
            report.adopted += adopted;
            report.restored += restored;
            report.cleared += cleared;
        }
        report.stale += left.len();
        for term in left.into_iter().flatten() {
            *report.stale_by_term.entry(term).or_default() += 1;
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{CompletionResponse, TranslateError};
    use async_trait::async_trait;
    use std::sync::Mutex;
    use yeokja_core::model::SegmentId;
    use yeokja_core::parser::Markup;
    use yeokja_core::state::SegmentState;

    fn words(source: &str) -> Vec<Vec<String>> {
        prose_chunks(source)
    }

    #[test]
    fn prose_chunks_drop_code_urls_math_and_roles() {
        assert_eq!(
            words("Use ``foo_bar`` and `x` with the **Ray Class** see https://x.org/a and $n^2$ :pep:`8` now."),
            vec![vec!["Use"], vec!["and"], vec!["with", "the", "Ray", "Class", "see"], vec!["and"], vec!["now"]]
                .into_iter()
                .map(|c| c.into_iter().map(String::from).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );
        assert_eq!(words("[the docs](./docs.md) say"), vec![vec!["the".to_string(), "docs".into()], vec!["say".into()]]);
        assert_eq!(words("A \\emph{group} has"), vec![vec!["group".to_string()], vec!["has".into()]]);
        assert_eq!(words("Python 3 is copy-on-write"), vec![vec!["Python".to_string()], vec!["is".into(), "copy-on-write".into()]]);
    }

    fn corpus(rows: &[(usize, &str, Option<&str>)]) -> Corpus {
        Corpus {
            files: (0..=rows.iter().map(|r| r.0).max().unwrap_or(0)).map(|i| PathBuf::from(format!("f{i}.md"))).collect(),
            segments: rows
                .iter()
                .map(|(file, source, translation)| Occurrence {
                    file: *file,
                    source: source.to_string(),
                    translation: translation.map(String::from),
                    fragment: false,
                })
                .collect(),
        }
    }

    #[test]
    fn mine_counts_terms_skips_stopword_edges_and_known_keys() {
        let rows: Vec<(usize, &str, Option<&str>)> = (0..4)
            .map(|i| (i, "Find a sponsor for the query planner.", Some("쿼리 플래너의 스폰서를 찾으십시오.")))
            .collect();
        let mined = mine(&corpus(&rows), &Known { terms: ["find".to_string()].into(), whole: HashSet::new() }, 4, 100);
        let terms: Vec<&str> = mined.terms.iter().map(|t| t.term.as_str()).collect();
        assert!(terms.contains(&"sponsor"));
        assert!(terms.contains(&"query planner"));
        // "query" and "planner" always occur as "query planner".
        assert!(!terms.contains(&"query") && !terms.contains(&"planner"));
        assert!(!terms.iter().any(|t| t.starts_with("a ") || t.ends_with(" the") || *t == "find"));
        let sponsor = mined.terms.iter().find(|t| t.term == "sponsor").unwrap();
        assert_eq!((sponsor.count, sponsor.files, sponsor.examples.len()), (4, 4, 4));
    }

    #[test]
    fn mine_folds_plurals_into_the_singular() {
        let mut rows = vec![(0, "Each process has a page table.", None); 2];
        rows.extend(vec![(1, "All processes share page tables and policies.", None); 2]);
        rows.extend(vec![(2, "One policy wins.", None); 2]);
        let mined = mine(&corpus(&rows), &Known::default(), 4, 100);
        let find = |t: &str| mined.terms.iter().find(|c| c.term == t);
        let process = find("process").unwrap();
        assert_eq!((process.count, process.forms.clone()), (4, vec!["processes".to_string()]));
        assert_eq!(find("page table").unwrap().forms, vec!["page tables".to_string()]);
        assert_eq!(find("policy").unwrap().forms, vec!["policies".to_string()]);
        assert!(find("processes").is_none() && find("page").is_none());
    }

    #[test]
    fn mine_does_not_fold_a_capitalized_name_into_a_common_noun() {
        let mut rows = vec![(0, "Open the window.", None), (0, "A window closes.", None), (0, "Each window moves.", None)];
        rows.extend(vec![(1, "It runs on Windows.", None), (1, "Windows is old.", None), (1, "Install Windows first.", None)]);
        let mined = mine(&corpus(&rows), &Known::default(), 3, 100);
        let find = |t: &str| mined.terms.iter().find(|c| c.term == t);
        assert!(find("window").unwrap().forms.is_empty());
        assert!(find("Windows").is_some());
    }

    #[test]
    fn mine_keeps_the_shorter_term_when_it_also_stands_alone() {
        let mut rows = vec![(0, "The ray class group is finite.", None); 4];
        rows.extend(vec![(1, "Every ray class is a coset.", None); 4]);
        let mined = mine(&corpus(&rows), &Known::default(), 4, 100);
        let terms: Vec<&str> = mined.terms.iter().map(|t| t.term.as_str()).collect();
        assert!(terms.contains(&"ray class"));
        assert!(terms.contains(&"ray class group"));
        assert!(!terms.contains(&"class group"));
    }

    #[test]
    fn mine_proposes_recurring_segments_only_when_translations_disagree() {
        let rows = vec![
            (0, "Abstract", Some("초록")),
            (1, "Abstract", Some("개요")),
            (2, "Abstract", Some("초록")),
            (0, "**Propose using:**", Some("**제안 방법:**")),
            (1, "Propose using:", Some("제안 방법:")),
            (2, "Propose using:", Some("제안 방법")),
            (0, "``x``", Some("``x``")),
            (0, "and", Some("그리고")),
            (1, "and", Some("과")),
            (2, "and", Some("그리고")),
            (0, "Thus,", Some("따라서,")),
            (1, "Thus,", Some("따라서")),
            (2, "Thus,", Some("따라서,")),
        ];
        let mined = mine(&corpus(&rows), &Known::default(), 99, 100);
        assert_eq!(mined.whole.len(), 1);
        assert_eq!(mined.whole[0].text, "Abstract");
        assert_eq!(mined.whole[0].variants, vec![("초록".to_string(), 2), ("개요".to_string(), 1)]);
        let mut fragments = corpus(&rows);
        fragments.segments.iter_mut().for_each(|o| o.fragment = true);
        assert!(mine(&fragments, &Known::default(), 99, 100).whole.is_empty());
        let known = Known { terms: HashSet::new(), whole: ["abstract".to_string()].into() };
        assert!(mine(&corpus(&rows), &known, 99, 100).whole.is_empty());
    }

    #[test]
    fn screen_reply_is_a_number_list_or_none() {
        assert_eq!(parse_screen_reply("1, 3,7", 5).unwrap(), [1, 3].into());
        assert_eq!(parse_screen_reply("None.", 5).unwrap(), HashSet::new());
        assert!(parse_screen_reply("Here are the terms: 1, 3", 5).is_err());
    }

    #[test]
    fn decide_reply_keeps_usable_renderings() {
        let reply = "[1] 스폰서\n[2] -\n[3] 쿼리 (질의)\n[4] \"초록\"\n[5] 제안 방법:\n[9] 무시";
        let decisions = parse_decide_reply(reply, 6).unwrap();
        assert_eq!(decisions.get(&1), Some(&Decision::Render("스폰서".into())));
        assert_eq!(decisions.get(&2), Some(&Decision::Reject));
        assert_eq!(decisions.get(&3), None);
        assert_eq!(decisions.get(&4), Some(&Decision::Render("초록".into())));
        assert_eq!(decisions.get(&5), Some(&Decision::Render("제안 방법".into())));
        assert_eq!(decisions.len(), 4);
    }

    struct Scripted(Mutex<Vec<String>>);

    #[async_trait]
    impl LlmProvider for Scripted {
        async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, TranslateError> {
            self.0.lock().unwrap().push(request.prompt.clone());
            let text = if request.prompt.contains("Reply with the numbers") {
                "1, 3".to_string()
            } else if request.prompt.contains("[1] SEGMENT") {
                // Wave one: the segment, then "sponsor".
                "[1] 초록\n[2] 스폰서".to_string()
            } else {
                "[1] 스폰서 계약".to_string()
            };
            Ok(CompletionResponse { text, usage: None })
        }
    }

    #[tokio::test]
    async fn decide_records_renderings_and_rejections_in_two_waves() {
        let rows = vec![
            (0, "Find a sponsor agreement, maybe.", Some("스폰서 계약을 찾으십시오.")),
            (1, "A sponsor helps, maybe.", Some("후원자가 돕습니다.")),
        ];
        let corpus = corpus(&rows);
        let term = |t: &str, forms: Vec<String>| TermCandidate { term: t.into(), forms, count: 2, files: 2, examples: vec![0, 1] };
        let candidates = Candidates {
            terms: vec![
                term("sponsor", vec!["sponsors".into()]),
                term("maybe", vec![]),
                term("sponsor agreement", vec![]),
            ],
            whole: vec![WholeCandidate {
                text: "Abstract".into(),
                key: "abstract".into(),
                count: 3,
                variants: vec![("초록".into(), 2), ("개요".into(), 1)],
            }],
        };
        let llm = Arc::new(Scripted(Mutex::new(Vec::new())));
        let mut file = TermsFile::default();
        let options = DecideOptions { source_lang: "en".into(), target_lang: "ko".into(), concurrency: 2 };
        let outcome = decide(llm.clone(), &corpus, candidates, &mut file, &Glossary::empty(), &options).await;
        assert_eq!(file.terms["sponsor"].translation, "스폰서");
        assert_eq!(file.terms["sponsors"].translation, "스폰서");
        assert_eq!(file.terms["sponsor agreement"].translation, "스폰서 계약");
        assert_eq!(file.whole["Abstract"].translation, "초록");
        assert!(file.rejected.terms.contains("maybe"));
        assert_eq!((outcome.terms_added, outcome.whole_added, outcome.rejected), (2, 1, 1));
        let prompts = llm.0.lock().unwrap();
        assert!(prompts[1].contains("후원자가 돕습니다.") && prompts[1].contains("SEGMENT \"Abstract\""));
        assert!(prompts[2].contains("fixed already: sponsor → 스폰서"));
    }

    #[tokio::test]
    async fn a_term_that_is_also_a_segment_is_left_to_the_segment() {
        let corpus = corpus(&[(0, "System specificity matters.", Some("시스템 특정성이 중요합니다."))]);
        let candidates = Candidates {
            terms: vec![TermCandidate { term: "System specificity".into(), forms: vec![], count: 4, files: 1, examples: vec![0] }],
            whole: vec![WholeCandidate {
                text: "System specificity".into(),
                key: "system specificity".into(),
                count: 3,
                variants: vec![("시스템 특정성".into(), 2), ("시스템 종속성".into(), 1)],
            }],
        };
        struct Reply;
        #[async_trait]
        impl LlmProvider for Reply {
            async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, TranslateError> {
                let text = if request.prompt.contains("Reply with the numbers") { "1" } else { "[1] 시스템 특정성" };
                assert!(!request.prompt.contains("[2]"), "the term is not asked about separately");
                Ok(CompletionResponse { text: text.to_string(), usage: None })
            }
        }
        let mut file = TermsFile::default();
        let options = DecideOptions { source_lang: "en".into(), target_lang: "ko".into(), concurrency: 1 };
        decide(Arc::new(Reply), &corpus, candidates, &mut file, &Glossary::empty(), &options).await;
        assert_eq!(file.whole["System specificity"].translation, "시스템 특정성");
        assert!(!file.terms.contains_key("System specificity"));
        assert!(file.rejected.terms.contains("system specificity"));
    }

    fn segment(source: &str, translation: &str) -> SegmentState {
        SegmentState {
            id: SegmentId::new(0, 0, 0),
            source: source.to_string(),
            source_hash: 0,
            context_hash: 0,
            translation: Some(translation.to_string()),
            glossary_snapshot: HashMap::new(),
            translated_at: None,
            issues: vec![],
        }
    }

    #[test]
    fn restore_earlier_brings_back_translations_that_meet_the_glossary() {
        let glossary = Glossary::from_toml("[terms.sponsor]\ntranslation = \"스폰서\"\n").unwrap();
        let mut earlier = StateFile::new(0);
        earlier.segments = vec![segment("Use camera::run().", "camera::run()을 사용하십시오."), segment("Find a sponsor.", "후원자를 찾으십시오.")];
        earlier.segments[1].id = SegmentId::new(0, 0, 1);
        let mut state = earlier.clone();
        state.segments[0].translation = Some("카메라의 camera::run()을 사용하십시오.".into());
        state.segments[1].translation = Some("스폰서를 찾으십시오.".into());
        assert_eq!(restore_earlier(&mut state, &earlier, &glossary, Markup::Markdown), 1);
        assert_eq!(state.segments[0].translation.as_deref(), Some("camera::run()을 사용하십시오."));
        assert_eq!(state.segments[1].translation.as_deref(), Some("스폰서를 찾으십시오."));
    }

    #[test]
    fn adopt_drops_glossary_issues_that_no_longer_hold() {
        let glossary = Glossary::from_toml("[terms.sponsor]\ntranslation = \"스폰서\"\n").unwrap();
        let mut state = StateFile::new(0);
        let mut met = segment("Find a sponsor.", "스폰서를 찾으십시오.");
        met.glossary_snapshot = glossary.find_matching_terms("Find a sponsor.");
        met.issues = vec![
            "Term 'sponsor' should be translated as '스폰서' but was not found in translation".into(),
            "Unclosed emphasis".into(),
        ];
        let mut unmet = segment("Find a sponsor.", "후원자를 찾으십시오.");
        unmet.glossary_snapshot = glossary.find_matching_terms("Find a sponsor.");
        unmet.issues = vec!["Term 'sponsor' should be translated as '스폰서' but was not found in translation".into()];
        state.segments = vec![met, unmet];
        let adopted = adopt_state(&mut state, &glossary, Markup::Markdown);
        assert_eq!((adopted.adopted, adopted.cleared), (0, 1));
        assert_eq!(state.segments[0].issues, vec!["Unclosed emphasis".to_string()]);
        assert_eq!(state.segments[1].issues.len(), 1);
    }

    #[test]
    fn adopt_accepts_translations_that_already_render_the_terms() {
        let glossary =
            Glossary::from_toml("[terms.sponsor]\ntranslation = \"스폰서\"\n[whole.Abstract]\ntranslation = \"초록\"\n").unwrap();
        let mut state = StateFile::new(0);
        state.segments = vec![
            segment("Find a sponsor.", "스폰서를 찾으십시오."),
            segment("Find a sponsor.", "후원자를 찾으십시오."),
            segment("Abstract", "초록"),
            segment("Abstract", "개요"),
            segment("Nothing here.", "여기는 없습니다."),
        ];
        let adopted = adopt_state(&mut state, &glossary, Markup::Markdown);
        assert_eq!(adopted.adopted, 2);
        assert_eq!(adopted.left, vec![vec!["sponsor".to_string()], vec!["Abstract".to_string()]]);
        assert_eq!(state.segments[0].glossary_snapshot.get("sponsor").map(String::as_str), Some("스폰서"));
        assert!(state.segments[1].glossary_snapshot.is_empty());
        assert!(!glossary.is_snapshot_stale(&state.segments[2].glossary_snapshot, "Abstract"));
    }
}
