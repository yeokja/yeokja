use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ops::Range;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlossaryEntry {
    pub translation: String,
    #[serde(default)]
    pub note: Option<String>,
    /// Phrases in which the term is an ordinary word, not the glossary term:
    /// `future` the async value against "in the future". An occurrence
    /// inside one of these (compared without regard to case) is not a use.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub except: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlossaryFile {
    #[serde(default)]
    pub terms: HashMap<String, GlossaryEntry>,
    /// Entries that apply only where a whole segment is the term: a heading
    /// or label such as "Abstract" or "Propose using:". As an ordinary term
    /// "abstract" would demand the heading's rendering inside "abstract base
    /// class" too. See [`normalize_whole`] for how segments are compared.
    #[serde(default)]
    pub whole: HashMap<String, GlossaryEntry>,
}

#[derive(Debug, Clone, Default)]
pub struct Glossary {
    terms: HashMap<String, String>,
    /// Term → phrases in which it is not the glossary term.
    except: HashMap<String, Vec<String>>,
    /// [`normalize_whole`] key → (entry as written, translation).
    whole: HashMap<String, (String, String)>,
    /// Whether a term also matches its regular English plural (see
    /// [`plural_forms`]): the project's `glossary_plurals`.
    plurals: bool,
}

/// A segment as whole-segment entries compare it: without surrounding
/// whitespace, emphasis marks around it, and a trailing colon or period, and
/// without regard to case. `**Propose using:**`, `Propose using:` and
/// `propose using` are the same label.
pub fn normalize_whole(text: &str) -> String {
    let mut text = text.trim();
    loop {
        let before = text;
        text = text
            .trim_matches(|c: char| c == '*' || c == '_' || c.is_whitespace())
            .trim_end_matches([':', '.', '：']);
        if text == before {
            break;
        }
    }
    text.to_lowercase()
}

/// Find terms from the given map that the text uses as prose.
///
/// A term only counts where a translator could act on it. Inside a code span,
/// a URL, or a longer identifier it is a name being quoted, and the translation
/// has to leave it exactly as it stands — so asking for the glossary rendering
/// there fails a translation that is correct, and the block is translated again
/// and fails again. Of the 229 segments theBeamBook's glossary check rejected,
/// 204 were this.
pub fn find_terms_in_text(terms: &HashMap<String, String>, text: &str) -> HashMap<String, String> {
    let chars: Vec<char> = text.to_lowercase().chars().collect();
    term_uses(terms, &HashMap::new(), &chars, false).into_iter().map(|u| (u.term, u.translation)).collect()
}

/// The regular English plurals of `term` (lowercased), which inflects in its
/// last word: `huge page` → `huge pages`, `patch` → `patches`, `policy` →
/// `policies`. Nothing for a term that does not end in an ASCII letter
/// (`C++`). Irregular plurals and verb endings are not guessed.
pub fn plural_forms(term: &str) -> Vec<String> {
    let chars: Vec<char> = term.chars().collect();
    let Some(&last) = chars.last() else { return Vec::new() };
    if !last.is_ascii_alphabetic() {
        return Vec::new();
    }
    let before = chars.len().checked_sub(2).map(|at| chars[at]);
    let mut forms = vec![format!("{term}s")];
    if matches!(last, 's' | 'x' | 'z' | 'o') || (last == 'h' && matches!(before, Some('c' | 's'))) {
        forms.push(format!("{term}es"));
    }
    if last == 'y' && before.is_some_and(|c| c.is_ascii_alphabetic() && !"aeiou".contains(c)) {
        let stem: String = chars[..chars.len() - 1].iter().collect();
        forms.push(format!("{stem}ies"));
    }
    forms
}

/// A term the text uses as prose, and where: char ranges of the lowercased
/// text, a plural ending included.
struct TermUse {
    term: String,
    translation: String,
    spans: Vec<Range<usize>>,
}

/// The terms `chars` (lowercased text) uses as prose, passing over
/// occurrences that stand inside one of the term's `except` phrases. With
/// `plurals`, a term's [`plural_forms`] are occurrences of it too.
fn term_uses(
    terms: &HashMap<String, String>,
    except: &HashMap<String, Vec<String>>,
    chars: &[char],
    plurals: bool,
) -> Vec<TermUse> {
    let verbatim = verbatim_spans(chars);

    // Longest match wins: a term nested in a longer one ("tensor" inside
    // "Tensor Unit") is not a term of its own where the longer one stands.
    // Without this the evaluator demands a Korean word inside a name the
    // glossary itself says to keep in English, and the retry cannot converge.
    let mut ordered: Vec<(&String, &String)> = terms.iter().collect();
    ordered.sort_by(|(a, _), (b, _)| b.chars().count().cmp(&a.chars().count()).then(a.cmp(b)));

    let mut claimed: Vec<Range<usize>> = Vec::new();
    let mut found = Vec::new();
    for (term, translation) in ordered {
        let lowered = term.to_lowercase();
        let ordinary: Vec<Range<usize>> = except
            .get(term)
            .into_iter()
            .flatten()
            .flat_map(|phrase| {
                let phrase: Vec<char> = phrase.to_lowercase().chars().collect();
                occurrences(chars, &phrase)
                    .map(|at| at..at + phrase.len())
                    .collect::<Vec<_>>()
            })
            .collect();
        let mut forms = vec![lowered.clone()];
        if plurals {
            forms.extend(plural_forms(&lowered));
        }
        let mut spans: Vec<Range<usize>> = Vec::new();
        for form in forms {
            let form: Vec<char> = form.chars().collect();
            for start in occurrences(chars, &form) {
                let end = start + form.len();
                let standalone = (start == 0 || !is_word_char(chars[start - 1]))
                    && (end >= chars.len() || !is_word_char(chars[end]))
                    && !joined_into_a_name(chars, start, end);
                let hidden = verbatim.iter().any(|s| s.start < end && start < s.end);
                let inside_longer = claimed.iter().any(|c| c.start <= start && end <= c.end);
                let as_ordinary_word = ordinary.iter().any(|o| o.start <= start && end <= o.end);
                if standalone && !hidden && !inside_longer && !as_ordinary_word {
                    spans.push(start..end);
                }
            }
        }
        if !spans.is_empty() {
            claimed.extend(spans.iter().cloned());
            found.push(TermUse { term: term.clone(), translation: translation.clone(), spans });
        }
    }
    found
}

/// Char ranges of the *delimited* stretches of `chars`: the inside of a
/// bracket pair (a link's text), of a pair of double quotation marks, and of
/// an emphasis. These are where a document quotes a title — a commit
/// subject, an article, a book — that a translation copies as it stands.
fn delimited_spans(chars: &[char]) -> Vec<Range<usize>> {
    let mut spans = Vec::new();
    // Brackets nest; quotation marks and emphasis runs pair in order.
    let mut brackets: Vec<usize> = Vec::new();
    let mut straight: Option<usize> = None;
    let mut curly: Option<usize> = None;
    let mut emphasis: Vec<(char, usize, usize)> = Vec::new(); // mark, run length, content start
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 1,
            '[' => brackets.push(i + 1),
            ']' => {
                if let Some(start) = brackets.pop() {
                    spans.push(start..i);
                }
            }
            '"' => match straight.take() {
                Some(start) => spans.push(start..i),
                None => straight = Some(i + 1),
            },
            '“' => curly = Some(i + 1),
            '”' => {
                if let Some(start) = curly.take() {
                    spans.push(start..i);
                }
            }
            mark @ ('*' | '_') => {
                let run = chars[i..].iter().take_while(|c| **c == mark).count();
                // `snake_case` is a word, not an emphasis.
                let inside_word = mark == '_'
                    && i > 0
                    && chars[i - 1].is_alphanumeric()
                    && chars.get(i + run).is_some_and(|c| c.is_alphanumeric());
                if !inside_word {
                    match emphasis.iter().rposition(|(m, len, _)| *m == mark && *len == run) {
                        Some(at) => {
                            spans.push(emphasis[at].2..i);
                            emphasis.truncate(at);
                        }
                        None => emphasis.push((mark, run, i + run)),
                    }
                }
                i += run - 1;
            }
            _ => {}
        }
        i += 1;
    }
    spans
}

/// Whether the occurrence `at` of a term stands in a stretch of the source
/// that the translation kept as written: a delimited span (see
/// [`delimited_spans`]) that holds more words than the term and whose text
/// is in `translation` (lowercased) unchanged. A title left untranslated
/// cannot show the term's rendering, and asking for it starts a retry that
/// never converges. A span that is the term and nothing else (`**huge
/// page**`) is the term left in English, which is still unmet.
fn kept_as_written(chars: &[char], delimited: &[Range<usize>], at: &Range<usize>, translation: &str) -> bool {
    delimited.iter().any(|span| {
        span.start <= at.start
            && at.end <= span.end
            && (chars[span.start..at.start].iter().any(|c| is_word_char(*c))
                || chars[at.end..span.end].iter().any(|c| is_word_char(*c)))
            && {
                // English puts a comma inside the closing quotation mark
                // ("mm: fix …," the commit); Korean puts it outside.
                let text: String = chars[span.clone()].iter().collect();
                translation.contains(text.trim_matches(|c: char| c.is_whitespace() || ",.;:".contains(c)))
            }
    })
}

/// Every position in `haystack` where `needle` starts.
fn occurrences<'a>(haystack: &'a [char], needle: &'a [char]) -> impl Iterator<Item = usize> + 'a {
    let last = haystack.len().checked_sub(needle.len());
    last.filter(|_| !needle.is_empty())
        .into_iter()
        .flat_map(move |last| (0..=last).filter(move |&i| haystack[i..i + needle.len()] == *needle))
}

/// What a term has to be free of on both sides to be a word of its own.
///
/// Underscore belongs here even though it is not alphanumeric: `min_heap_size`
/// is one identifier, not a sentence containing "heap". This is also how
/// Asciidoctor reads a word — its `\p{Word}` covers `_`.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether the word at `start..end` is part of a longer name that stays as
/// written: a handle or reference (`@reviewer`, `#t-release`), a TeX control
/// word (`\code`), a flag
/// (`--edition=2021`), a file name (`package.nix`), or a path or identifier
/// joined by slashes or hyphens (`compiler/rustc_hir/src/hir.rs`,
/// `trait-system-refactor-initiative#102`).
///
/// Prose joins words the same way — `package-level`, `compiler/linker` — and
/// is translated, so a hyphen or a slash alone does not make a name: the
/// joined token has to show a file extension, an underscore, a `#`, or a
/// leading slash. A dot ends a sentence unless a letter or digit follows it.
fn joined_into_a_name(chars: &[char], start: usize, end: usize) -> bool {
    let before = start.checked_sub(1).map(|at| chars[at]);
    let after = chars.get(end).copied();
    let alnum_at = |at: usize| chars.get(at).is_some_and(|c| c.is_ascii_alphanumeric());
    // `\code{...}`, `\emph`: a word right after a backslash is a TeX
    // control word, a command's name rather than prose.
    if matches!(before, Some('@' | '#' | '\\'))
        || (before == Some('-') && start >= 2 && chars[start - 2] == '-')
        || (after == Some('=') && alnum_at(end + 1))
        || (before == Some('.') && start >= 2 && alnum_at(start - 2))
        || (after == Some('.') && alnum_at(end + 1))
        // `camera::ray_color()`, `std::vec`: a path through `::`.
        || (after == Some(':') && chars.get(end + 1) == Some(&':'))
        || (before == Some(':') && start >= 2 && chars[start - 2] == ':')
        // `sec:identity-types`, `part:groups`, `axiom:funext`: a
        // cross-reference label, on either side of its colon.
        || (before == Some(':') && start >= 2 && alnum_at(start - 2))
        || (after == Some(':') && alnum_at(end + 1))
    {
        return true;
    }
    if !matches!(before, Some('/' | '-')) && !matches!(after, Some('/' | '-')) {
        return false;
    }
    // The token runs to whitespace, a bracket or quote, or LaTeX markup (`$`,
    // `\`, braces); emphasis marks and sentence punctuation at its edges are
    // not part of it.
    let bound = |c: &char| c.is_whitespace() || "()[]<>\"'`,;{}$\\".contains(*c);
    let edge = |c: &char| "*_.:!?".contains(*c);
    let mut from = chars[..start]
        .iter()
        .rposition(bound)
        .map_or(0, |at| at + 1);
    let mut to = chars[end..]
        .iter()
        .position(bound)
        .map_or(chars.len(), |at| end + at);
    while from < start && edge(&chars[from]) {
        from += 1;
    }
    while to > end && edge(&chars[to - 1]) {
        to -= 1;
    }
    let token = &chars[from..to];
    token.first() == Some(&'/')
        || token.iter().any(|c| matches!(c, '_' | '#'))
        || token.windows(3).any(|w| w[0].is_ascii_alphanumeric() && w[1] == ':' && w[2].is_ascii_alphanumeric())
        || token
            .windows(2)
            .any(|pair| pair[0] == '.' && pair[1].is_ascii_alphanumeric())
}

/// Char ranges of `chars` that a reader sees exactly as written: inline code
/// spans, the targets of URLs and macros, HTML tags, and the labels of full
/// reference links.
///
/// The link *text* is deliberately left out — `link:https://…[ERTS Reference]`
/// carries prose after the `[` that a translator may well render.
fn verbatim_spans(chars: &[char]) -> Vec<Range<usize>> {
    let mut spans = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if let Some(end) = code_span_end(chars, i)
            .or_else(|| target_end(chars, i))
            .or_else(|| html_tag_end(chars, i))
            .or_else(|| reference_label_end(chars, i))
        {
            spans.push(i..end);
            i = end;
        } else {
            i += 1;
        }
    }
    spans
}

/// Where the code span opening at `at` ends, if one does. Both markups write it
/// between two runs of backticks of the same length, so `` ``x`` `` closes on
/// the pair and not on the first backtick of it.
fn code_span_end(chars: &[char], at: usize) -> Option<usize> {
    let run = |from: usize| chars[from..].iter().take_while(|c| **c == '`').count();
    let open = run(at);
    if open == 0 {
        return None;
    }
    let mut from = at + open;
    loop {
        let offset = chars[from..].iter().position(|c| *c == '`')?;
        let close = from + offset;
        let len = run(close);
        if len == open {
            return Some(close + len);
        }
        from = close + len;
    }
}

/// Where the HTML tag opening at `at` ends, if one does: `<a id="x">`, `</a>`,
/// `<br>`. A tag is a bare name or a name with `=` attributes, so `a <b then c>`
/// stays prose.
fn html_tag_end(chars: &[char], at: usize) -> Option<usize> {
    if chars[at] != '<' || (at > 0 && chars[at - 1] == '<') {
        return None;
    }
    let close = at + chars[at..].iter().position(|c| *c == '>' || *c == '\n')?;
    if chars[close] != '>' {
        return None;
    }
    let inner: String = chars[at + 1..close].iter().collect();
    let inner = inner
        .strip_prefix('/')
        .unwrap_or(&inner)
        .trim_end_matches('/');
    let name_len = inner
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
        .count();
    let attributes = &inner[name_len..];
    (inner.starts_with(|c: char| c.is_ascii_alphabetic())
        && (attributes.trim().is_empty()
            || (attributes.starts_with(char::is_whitespace) && attributes.contains('='))))
    .then_some(close + 1)
}

/// Where the label of a full reference link starting at `at` ends, if one
/// does: the `[label]` right after the `]` of `[text][label]`.
fn reference_label_end(chars: &[char], at: usize) -> Option<usize> {
    if chars[at] != '[' || at == 0 || chars[at - 1] != ']' {
        return None;
    }
    let offset = chars[at + 1..]
        .iter()
        .position(|c| matches!(c, ']' | '[' | '\n'))?;
    (chars[at + 1 + offset] == ']').then_some(at + 1 + offset + 1)
}

/// Where the URL or macro target starting at `at` ends, if one does. A target
/// runs to the whitespace or the `[` that ends it; a Markdown one to its `)`.
fn target_end(chars: &[char], at: usize) -> Option<usize> {
    const SCHEMES: [&str; 9] = [
        "http://", "https://", "ftp://", "mailto:", "link:", "xref:", "image:", "video:", "audio:",
    ];
    // Markdown writes the target after the link text, with no scheme of its own.
    if chars[at] == ']' && chars.get(at + 1) == Some(&'(') {
        let offset = chars[at + 2..].iter().position(|c| *c == ')')?;
        return Some(at + 2 + offset + 1);
    }
    if at > 0 && is_word_char(chars[at - 1]) {
        return None;
    }
    let starts_with = |s: &str| chars[at..].iter().copied().take(s.chars().count()).eq(s.chars());
    SCHEMES.iter().any(|s| starts_with(s)).then(|| {
        at + chars[at..]
            .iter()
            .position(|c| c.is_whitespace() || *c == '[')
            .unwrap_or(chars.len() - at)
    })
}

impl Glossary {
    pub fn load(path: &Path) -> Result<Self, GlossaryError> {
        let content = std::fs::read_to_string(path)
            .map_err(GlossaryError::Io)?;
        Self::from_toml(&content)
    }

    pub fn from_toml(content: &str) -> Result<Self, GlossaryError> {
        let file: GlossaryFile = toml::from_str(content)
            .map_err(|e| GlossaryError::Parse(e.to_string()))?;
        Ok(Self::from_file(file))
    }

    fn from_file(file: GlossaryFile) -> Self {
        let mut terms = HashMap::new();
        let mut except = HashMap::new();
        for (term, entry) in file.terms {
            if !entry.except.is_empty() {
                except.insert(term.clone(), entry.except);
            }
            terms.insert(term, entry.translation);
        }
        let whole = file
            .whole
            .into_iter()
            .map(|(text, entry)| (normalize_whole(&text), (text, entry.translation)))
            .collect();
        Self { terms, except, whole, plurals: false }
    }

    /// The same glossary, matching each term's regular English plural as
    /// well (or not).
    pub fn with_plurals(mut self, plurals: bool) -> Self {
        self.plurals = plurals;
        self
    }

    /// The project's glossary: the curated `glossary` file with the generated
    /// `terms` file (see [`TermsFile`]) beneath it. Either may be absent. A
    /// term in both takes the curated translation.
    pub fn load_project(
        dir: &Path,
        settings: &crate::config::ProjectSettings,
    ) -> Result<Self, GlossaryError> {
        let curated_path = dir.join(&settings.glossary);
        let curated = if curated_path.exists() {
            let content = std::fs::read_to_string(&curated_path)?;
            toml::from_str(&content).map_err(|e| GlossaryError::Parse(e.to_string()))?
        } else {
            GlossaryFile { terms: HashMap::new(), whole: HashMap::new() }
        };
        let terms_path = dir.join(&settings.terms);
        let generated = if terms_path.exists() { TermsFile::load(&terms_path)? } else { TermsFile::default() };
        Ok(Self::merged(curated, generated).with_plurals(settings.glossary_plurals))
    }

    /// `curated` over `generated`: an entry in both keeps the curated one.
    /// Terms and whole-segment entries are compared without regard to case,
    /// the way they are matched.
    pub fn merged(mut curated: GlossaryFile, generated: TermsFile) -> Self {
        let curated_terms: std::collections::HashSet<String> =
            curated.terms.keys().map(|k| k.to_lowercase()).collect();
        let curated_whole: std::collections::HashSet<String> =
            curated.whole.keys().map(|k| normalize_whole(k)).collect();
        for (term, entry) in generated.terms {
            if !curated_terms.contains(&term.to_lowercase()) {
                curated.terms.insert(term, entry);
            }
        }
        for (text, entry) in generated.whole {
            if !curated_whole.contains(&normalize_whole(&text)) {
                curated.whole.insert(text, entry);
            }
        }
        Self::from_file(curated)
    }

    /// Lowercased keys of every term, and [`normalize_whole`] keys of every
    /// whole-segment entry: what a term search should not propose again.
    pub fn known_keys(&self) -> (std::collections::HashSet<String>, std::collections::HashSet<String>) {
        (self.terms.keys().map(|k| k.to_lowercase()).collect(), self.whole.keys().cloned().collect())
    }

    /// The whole-segment entry `source` is, as (entry as written, translation).
    pub fn whole_match(&self, source: &str) -> Option<(&str, &str)> {
        if self.whole.is_empty() {
            return None;
        }
        self.whole
            .get(&normalize_whole(source))
            .map(|(text, translation)| (text.as_str(), translation.as_str()))
    }

    /// The entries `source` uses that `translation` does not render, as
    /// (term, expected translation), sorted. A term is met when the
    /// translation contains its rendering (without regard to case); a
    /// whole-segment entry only when the translation, normalized the same
    /// way, is its rendering. The glossary evaluator and snapshot adoption
    /// share this judgement.
    ///
    /// A term is also met where the translation could not have rendered it:
    /// every occurrence stands in a stretch of the source the translation
    /// kept as written (see [`kept_as_written`]) — the text of a link, a
    /// quotation or an emphasis that is a title rather than prose.
    pub fn unmet_terms(&self, source: &str, translation: &str) -> Vec<(String, String)> {
        let lowered = translation.to_lowercase();
        let chars: Vec<char> = source.to_lowercase().chars().collect();
        let delimited = delimited_spans(&chars);
        let mut unmet: Vec<(String, String)> = Vec::new();
        let whole = self.whole_match(source);
        if let Some((entry, expected)) = whole
            && normalize_whole(translation) != normalize_whole(expected)
        {
            unmet.push((entry.to_string(), expected.to_string()));
        }
        for used in term_uses(&self.terms, &self.except, &chars, self.plurals) {
            // The whole-segment entry replaces an ordinary term spelled the same.
            if whole.is_some_and(|(entry, _)| entry == used.term) {
                continue;
            }
            if !lowered.contains(&used.translation.to_lowercase())
                && !used.spans.iter().all(|at| kept_as_written(&chars, &delimited, at, &lowered))
            {
                unmet.push((used.term, used.translation));
            }
        }
        unmet.sort();
        unmet
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn terms(&self) -> &HashMap<String, String> {
        &self.terms
    }

    /// Find glossary terms the given text uses as prose.
    /// Uses char-based boundary checking to handle non-ASCII (Korean, CJK) text correctly.
    /// A segment that is a whole-segment entry also matches that entry, under
    /// the entry's own spelling; it replaces an ordinary term spelled the same.
    pub fn find_matching_terms(&self, text: &str) -> HashMap<String, String> {
        let chars: Vec<char> = text.to_lowercase().chars().collect();
        let mut found: HashMap<String, String> = term_uses(&self.terms, &self.except, &chars, self.plurals)
            .into_iter()
            .map(|used| (used.term, used.translation))
            .collect();
        if let Some((entry, translation)) = self.whole_match(text) {
            found.insert(entry.to_string(), translation.to_string());
        }
        found
    }

    /// Check if a glossary snapshot is stale compared to current glossary.
    pub fn is_snapshot_stale(&self, snapshot: &HashMap<String, String>, source_text: &str) -> bool {
        let current_matches = self.find_matching_terms(source_text);
        for (term, translation) in &current_matches {
            match snapshot.get(term) {
                Some(snap_translation) if snap_translation == translation => {}
                _ => return true,
            }
        }
        false
    }
}

/// The generated term base (`terms.toml`): terms and whole-segment entries
/// decided once for the whole project by `yeokja terms build`, and the
/// candidates decided against, so they are not asked about again. Kept
/// sorted so a rerun's diff shows only what changed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TermsFile {
    #[serde(default)]
    pub terms: BTreeMap<String, GlossaryEntry>,
    #[serde(default)]
    pub whole: BTreeMap<String, GlossaryEntry>,
    #[serde(default)]
    pub rejected: Rejected,
}

/// Candidates decided against: lowercased terms and [`normalize_whole`] keys.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Rejected {
    #[serde(default)]
    pub terms: BTreeSet<String>,
    #[serde(default)]
    pub whole: BTreeSet<String>,
}

const TERMS_FILE_HEADER: &str = "\
# Project term base. `yeokja terms build` adds to it; edit freely.
# glossary.toml wins where both have an entry. [whole] entries apply only
# where a whole segment (a heading, a label) is the entry.

";

impl TermsFile {
    pub fn load(path: &Path) -> Result<Self, GlossaryError> {
        let content = std::fs::read_to_string(path)?;
        toml::from_str(&content).map_err(|e| GlossaryError::Parse(e.to_string()))
    }

    pub fn save(&self, path: &Path) -> Result<(), GlossaryError> {
        let body = toml::to_string_pretty(self).map_err(|e| GlossaryError::Serialize(e.to_string()))?;
        std::fs::write(path, format!("{TERMS_FILE_HEADER}{body}"))?;
        Ok(())
    }

    /// Lowercased terms and whole keys this file has decided, either way.
    pub fn decided_keys(&self) -> (BTreeSet<String>, BTreeSet<String>) {
        let mut terms: BTreeSet<String> = self.terms.keys().map(|k| k.to_lowercase()).collect();
        terms.extend(self.rejected.terms.iter().cloned());
        let mut whole: BTreeSet<String> = self.whole.keys().map(|k| normalize_whole(k)).collect();
        whole.extend(self.rejected.whole.iter().cloned());
        (terms, whole)
    }
}

/// Add or update a term in a glossary TOML file, preserving existing entries
/// (including their `note` fields). Creates the file if it does not exist.
/// A glossary of plain term → translation pairs, with no `except` phrases.
impl FromIterator<(String, String)> for Glossary {
    fn from_iter<I: IntoIterator<Item = (String, String)>>(pairs: I) -> Self {
        Self {
            terms: pairs.into_iter().collect(),
            except: HashMap::new(),
            whole: HashMap::new(),
            plurals: false,
        }
    }
}

pub fn upsert_term_in_file(
    path: &Path,
    term: &str,
    translation: &str,
) -> Result<(), GlossaryError> {
    let content = if path.exists() {
        std::fs::read_to_string(path)?
    } else {
        String::new()
    };

    let mut doc: toml::Table = if content.is_empty() {
        toml::Table::new()
    } else {
        content
            .parse()
            .map_err(|e: toml::de::Error| GlossaryError::Parse(e.to_string()))?
    };

    let terms = doc
        .entry("terms")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));
    if let toml::Value::Table(terms_table) = terms {
        let entry = terms_table
            .entry(term.to_string())
            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
        if let toml::Value::Table(entry_table) = entry {
            entry_table.insert(
                "translation".to_string(),
                toml::Value::String(translation.to_string()),
            );
        } else {
            let mut entry_table = toml::Table::new();
            entry_table.insert(
                "translation".to_string(),
                toml::Value::String(translation.to_string()),
            );
            *entry = toml::Value::Table(entry_table);
        }
    }

    let serialized = toml::to_string_pretty(&doc)
        .map_err(|e| GlossaryError::Serialize(e.to_string()))?;
    std::fs::write(path, serialized)?;
    Ok(())
}

/// Remove a term from a glossary TOML file. Returns whether the term existed.
pub fn remove_term_in_file(path: &Path, term: &str) -> Result<bool, GlossaryError> {
    if !path.exists() {
        return Ok(false);
    }
    let content = std::fs::read_to_string(path)?;
    let mut doc: toml::Table = content
        .parse()
        .map_err(|e: toml::de::Error| GlossaryError::Parse(e.to_string()))?;

    let removed = match doc.get_mut("terms") {
        Some(toml::Value::Table(terms_table)) => terms_table.remove(term).is_some(),
        _ => false,
    };

    if removed {
        let serialized = toml::to_string_pretty(&doc)
            .map_err(|e| GlossaryError::Serialize(e.to_string()))?;
        std::fs::write(path, serialized)?;
    }
    Ok(removed)
}

#[derive(Debug, thiserror::Error)]
pub enum GlossaryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Serialize error: {0}")]
    Serialize(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_glossary() -> Glossary {
        let toml = r#"
[terms.repository]
translation = "저장소"
note = "Git 저장소"

[terms.commit]
translation = "커밋"
"#;
        Glossary::from_toml(toml).unwrap()
    }

    fn kernel_glossary() -> Glossary {
        Glossary::from_toml(
            r#"
[terms."huge page"]
translation = "huge page"
[terms.scheduler]
translation = "스케줄러"
[terms.patch]
translation = "패치"
[terms.policy]
translation = "정책"
[terms."C++"]
translation = "C++"
[terms.preemption]
translation = "선점"
"#,
        )
        .unwrap()
    }

    fn unmet(glossary: &Glossary, source: &str, translation: &str) -> Vec<String> {
        glossary.unmet_terms(source, translation).into_iter().map(|(term, _)| term).collect()
    }

    #[test]
    fn a_term_inside_a_title_the_translation_kept_is_met() {
        let g = kernel_glossary();
        // Link text, a quotation and an emphasis, each kept as written.
        assert!(unmet(&g, "[sched: fix scheduler wakeup race](https://x.y/1) — a fix.", "[sched: fix scheduler wakeup race](https://x.y/1) — 수정입니다.").is_empty());
        assert!(unmet(&g, "See \"The extensible scheduler class\" on LWN.", "LWN의 \"The extensible scheduler class\"를 보십시오.").is_empty());
        assert!(unmet(&g, "See “The extensible scheduler class” on LWN.", "LWN의 “The extensible scheduler class”를 보십시오.").is_empty());
        assert!(unmet(&g, "Read *Realtime preemption end game* first.", "먼저 *Realtime preemption end game*을 읽으십시오.").is_empty());
        assert!(unmet(&g, "Read [the scheduler docs][sched] first.", "먼저 [the scheduler docs][sched]를 읽으십시오.").is_empty());
        // Punctuation English keeps inside the closing quotation mark.
        assert!(unmet(&g, "Fixed by \"sched: make the scheduler fair,\" in 2023.", "2023년의 \"sched: make the scheduler fair\"로 수정되었습니다.").is_empty());
        // The comparison ignores case, as term matching does.
        assert!(unmet(&g, "See \"The Scheduler Class\".", "\"The scheduler class\"를 보십시오.").is_empty());
    }

    #[test]
    fn a_term_left_in_english_is_still_unmet() {
        let g = kernel_glossary();
        // Outside any delimited stretch.
        assert_eq!(unmet(&g, "The scheduler picks a task.", "scheduler가 태스크를 고릅니다."), ["scheduler"]);
        // The stretch is the term and nothing else.
        assert_eq!(unmet(&g, "The **scheduler** picks a task.", "**scheduler**가 태스크를 고릅니다."), ["scheduler"]);
        assert_eq!(unmet(&g, "The \"scheduler\" picks.", "\"scheduler\"가 고릅니다."), ["scheduler"]);
        // The title was translated, without the rendering.
        assert_eq!(
            unmet(&g, "See \"The extensible scheduler class\".", "\"확장 가능한 스케쥴러 클래스\"를 보십시오."),
            ["scheduler"]
        );
        // Kept in the title, but used in the prose around it as well.
        assert_eq!(
            unmet(&g, "[The scheduler class](u) changed the scheduler.", "[The scheduler class](u)가 scheduler를 바꿨습니다."),
            ["scheduler"]
        );
        // A snake_case name is not an emphasis.
        assert_eq!(unmet(&g, "Set a_x and the scheduler b_y.", "a_x and the scheduler b_y를 설정합니다."), ["scheduler"]);
    }

    #[test]
    fn a_kept_title_does_not_change_what_a_snapshot_records() {
        let g = kernel_glossary();
        let source = "See \"The extensible scheduler class\" on LWN.";
        assert!(g.find_matching_terms(source).contains_key("scheduler"));
        assert!(g.is_snapshot_stale(&HashMap::new(), source));
    }

    #[test]
    fn plurals_match_only_when_the_project_asks() {
        let source = "Huge pages need patches and policies; C++s and schedulers too.";
        let plain = kernel_glossary();
        assert!(plain.find_matching_terms(source).is_empty());
        let g = kernel_glossary().with_plurals(true);
        let mut found: Vec<String> = g.find_matching_terms(source).into_keys().collect();
        found.sort();
        assert_eq!(found, ["huge page", "patch", "policy", "scheduler"]);
        // Not a plural: another word that merely starts the same.
        assert!(g.find_matching_terms("The patched policymaker schedulerx.").is_empty());
        assert_eq!(plural_forms("patch"), ["patchs", "patches"]);
        assert_eq!(plural_forms("policy"), ["policys", "policies"]);
        assert_eq!(plural_forms("key"), ["keys"]);
        assert!(plural_forms("c++").is_empty());
    }

    #[test]
    fn a_plural_entry_of_its_own_wins_over_the_singular() {
        let g = Glossary::from_toml(
            "[terms.\"huge page\"]\ntranslation = \"거대 페이지\"\n[terms.\"huge pages\"]\ntranslation = \"거대 페이지들\"\n",
        )
        .unwrap()
        .with_plurals(true);
        let found = g.find_matching_terms("Use huge pages.");
        assert_eq!(found.keys().collect::<Vec<_>>(), ["huge pages"]);
    }

    #[test]
    fn a_plural_that_is_the_whole_emphasis_is_the_term_left_in_english() {
        let g = kernel_glossary().with_plurals(true);
        assert_eq!(unmet(&g, "Use **schedulers** here.", "여기서 **schedulers**를 씁니다."), ["scheduler"]);
        assert!(unmet(&g, "See \"Pluggable schedulers\".", "\"Pluggable schedulers\"를 보십시오.").is_empty());
    }

    #[test]
    fn except_phrases_are_not_uses_of_the_term() {
        let g = Glossary::from_toml(
            r#"
[terms.future]
translation = "퓨처"
except = ["in the future", "near future"]
"#,
        )
        .unwrap();
        assert!(g.find_matching_terms("More languages will follow In The Future.").is_empty());
        assert!(g.find_matching_terms("In the near future, registries will work.").is_empty());
        assert_eq!(
            g.find_matching_terms("Awaiting the future in the future.").get("future").unwrap(),
            "퓨처"
        );
        assert!(!g.is_snapshot_stale(&HashMap::new(), "in the future"));
    }

    #[test]
    fn load_glossary_from_toml() {
        let g = test_glossary();
        assert_eq!(g.terms().get("repository").unwrap(), "저장소");
        assert_eq!(g.terms().get("commit").unwrap(), "커밋");
    }

    #[test]
    fn find_matching_terms_word_boundary() {
        let g = test_glossary();
        let matches = g.find_matching_terms("The repository stores commits.");
        assert_eq!(matches.get("repository").unwrap(), "저장소");
        assert_eq!(matches.get("commit"), None);
    }

    #[test]
    fn a_longer_term_wins_over_the_shorter_one_it_contains() {
        let g = Glossary::from_toml(
            r#"
[terms.tensor]
translation = "텐서"

[terms."Tensor Unit"]
translation = "Tensor Unit"
"#,
        )
        .unwrap();
        // Only the proper noun occurs, so its parts must not be demanded too.
        let matches = g.find_matching_terms("The Fetch Engine feeds the Tensor Unit.");
        assert_eq!(matches.get("Tensor Unit").unwrap(), "Tensor Unit");
        assert_eq!(matches.get("tensor"), None);
        // The bare word still counts where it stands on its own.
        let both = g.find_matching_terms("A tensor enters the Tensor Unit.");
        assert_eq!(both.get("tensor").unwrap(), "텐서");
        assert_eq!(both.get("Tensor Unit").unwrap(), "Tensor Unit");
    }

    #[test]
    fn find_matching_terms_no_partial() {
        let g = test_glossary();
        let matches = g.find_matching_terms("repositoryName is set");
        assert!(matches.is_empty());
    }

    fn beam_glossary() -> Glossary {
        Glossary::from_toml(
            r#"
[terms.Erlang]
translation = "Erlang"

[terms.heap]
translation = "힙"

[terms.compiler]
translation = "컴파일러"
"#,
        )
        .unwrap()
    }

    /// The case this was written for. `erlang:processes/0` has to survive the
    /// translation as it stands, so a glossary rendering can never appear for
    /// it — and every retry produced the same text and the same complaint.
    #[test]
    fn a_term_inside_a_code_span_is_a_name() {
        let g = beam_glossary();
        assert!(g.find_matching_terms("Call `erlang:processes/0` to list them.").is_empty());
        assert!(g.find_matching_terms("Set ``min_heap_size`` on spawn.").is_empty());
        // …but the same term outside one is still prose.
        let both = g.find_matching_terms("In Erlang, call `erlang:processes/0`.");
        assert_eq!(both.get("Erlang").unwrap(), "Erlang");
    }

    #[test]
    fn a_term_inside_a_url_is_a_name() {
        let g = beam_glossary();
        for text in [
            "See https://www.erlang.org/doc/apps/compiler/index.html for details.",
            "See link:https://erlang.org/x[the docs] for details.",
            "See [the docs](https://erlang.org/compiler) for details.",
        ] {
            assert!(g.find_matching_terms(text).is_empty(), "{text:?}");
        }
    }

    /// A macro's link text is prose and gets translated, so it is not masked
    /// along with the target.
    #[test]
    fn link_text_is_still_prose() {
        let g = beam_glossary();
        let matches = g.find_matching_terms("link:https://x.example/a[The Erlang compiler]");
        assert_eq!(matches.get("Erlang").unwrap(), "Erlang");
        assert_eq!(matches.get("compiler").unwrap(), "컴파일러");
    }

    /// Underscore joins a word rather than breaking it, which is also how
    /// Asciidoctor reads one.
    #[test]
    fn an_identifier_is_one_word() {
        let g = beam_glossary();
        assert!(g.find_matching_terms("The allocate_heap_zero instruction.").is_empty());
        assert_eq!(
            g.find_matching_terms("The heap grows.").get("heap").unwrap(),
            "힙"
        );
    }

    fn names_glossary() -> Glossary {
        Glossary::from_toml(
            r#"
[terms.triage]
translation = "트리아지"

[terms.term]
translation = "텀"

[terms.compiler]
translation = "컴파일러"

[terms.package]
translation = "패키지"

[terms.reviewer]
translation = "리뷰어"

[terms.edition]
translation = "에디션"

[terms.interface]
translation = "인터페이스"
"#,
        )
        .unwrap()
    }

    /// A handle, a flag, a file name or a path that stays as written holds the
    /// term as part of a longer name.
    #[test]
    fn a_term_joined_into_a_name_is_part_of_it() {
        let g = names_glossary();
        for text in [
            "See compiler/rustc_hir/src/hir.rs for the definition.",
            "See [triage-procedure#12] for details.",
            "Edit package.nix and rebuild.",
            "If @reviewer told you to merge, merge.",
            "Run it with --edition=2021 to reproduce.",
        ] {
            assert!(g.find_matching_terms(text).is_empty(), "{text:?}");
        }
        // Prose joins words with hyphens and slashes too, and those are
        // translated: a compound, a list of alternatives, an emphasis that
        // ends a sentence.
        for text in [
            "A package-level setting.",
            "The compiler/linker pair.",
            "Pick a compiler/interpreter.",
            "_It is the compiler._",
            "_A package-level_ setting.",
            "A $p$-compiler\\index{p.compiler} here.",
        ] {
            assert!(
                !g.find_matching_terms(text).is_empty(),
                "{text:?} should still match"
            );
        }
        // Sentence punctuation still ends a word.
        for text in [
            "Run the compiler.",
            "The compiler, then the linker.",
            "(the compiler)",
        ] {
            assert_eq!(
                g.find_matching_terms(text)
                    .get("compiler")
                    .map(String::as_str),
                Some("컴파일러"),
                "{text:?}"
            );
        }
    }

    /// An HTML tag is markup, its attribute values included.
    #[test]
    fn an_html_tag_is_markup() {
        let g = names_glossary();
        assert!(g.find_matching_terms("<a id=\"interface\"></a>").is_empty());
        assert!(
            g.find_matching_terms("<span id=\"term-list\">`x`</span>")
                .is_empty()
        );
        // A comparison is not a tag.
        assert_eq!(
            g.find_matching_terms("if a <b then the interface>")
                .get("interface")
                .map(String::as_str),
            Some("인터페이스")
        );
    }

    /// The label of a full reference link is not rendered: `[text][label]`
    /// shows only the text.
    #[test]
    fn a_reference_label_is_not_prose() {
        let g = names_glossary();
        assert!(
            g.find_matching_terms("See [the docs][compiler].")
                .is_empty()
        );
        assert_eq!(
            g.find_matching_terms("See [the compiler][docs].")
                .get("compiler")
                .map(String::as_str),
            Some("컴파일러")
        );
    }

    /// A backtick with nothing to pair with opens no span, and the prose after
    /// it still counts.
    #[test]
    fn a_lone_backtick_masks_nothing() {
        let g = beam_glossary();
        assert_eq!(
            g.find_matching_terms("A ` and the heap after it")
                .get("heap")
                .unwrap(),
            "힙"
        );
    }

    #[test]
    fn snapshot_not_stale_when_matching() {
        let g = test_glossary();
        let mut snapshot = HashMap::new();
        snapshot.insert("repository".to_string(), "저장소".to_string());
        assert!(!g.is_snapshot_stale(&snapshot, "The repository is ready."));
    }

    #[test]
    fn snapshot_stale_when_translation_changed() {
        let g = test_glossary();
        let mut snapshot = HashMap::new();
        snapshot.insert("repository".to_string(), "레포지토리".to_string());
        assert!(g.is_snapshot_stale(&snapshot, "The repository is ready."));
    }

    #[test]
    fn snapshot_stale_when_new_term_added() {
        let g = test_glossary();
        let snapshot = HashMap::new();
        assert!(g.is_snapshot_stale(&snapshot, "The repository is ready."));
    }

    #[test]
    fn snapshot_not_stale_when_term_not_in_text() {
        let g = test_glossary();
        let snapshot = HashMap::new();
        assert!(!g.is_snapshot_stale(&snapshot, "Hello world."));
    }

    #[test]
    fn upsert_preserves_notes_and_updates() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("glossary.toml");
        std::fs::write(
            &path,
            "[terms.repository]\ntranslation = \"저장소\"\nnote = \"Git 저장소\"\n",
        )
        .unwrap();

        upsert_term_in_file(&path, "branch", "브랜치").unwrap();
        upsert_term_in_file(&path, "repository", "리포지터리").unwrap();

        let g = Glossary::load(&path).unwrap();
        assert_eq!(g.terms().get("branch").unwrap(), "브랜치");
        assert_eq!(g.terms().get("repository").unwrap(), "리포지터리");

        // The note on the updated entry must survive.
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("Git 저장소"));
    }

    #[test]
    fn upsert_creates_file_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("glossary.toml");
        upsert_term_in_file(&path, "commit", "커밋").unwrap();
        let g = Glossary::load(&path).unwrap();
        assert_eq!(g.terms().get("commit").unwrap(), "커밋");
    }

    #[test]
    fn remove_term_removes_and_reports() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("glossary.toml");
        upsert_term_in_file(&path, "commit", "커밋").unwrap();

        assert!(remove_term_in_file(&path, "commit").unwrap());
        assert!(!remove_term_in_file(&path, "commit").unwrap());
        let g = Glossary::load(&path).unwrap();
        assert!(g.terms().is_empty());
    }

    fn whole_glossary() -> Glossary {
        Glossary::from_toml(
            r#"
[terms.abstract]
translation = "추상"
[terms.sponsor]
translation = "스폰서"
[whole.Abstract]
translation = "초록"
[whole."Propose using:"]
translation = "제안 방법"
"#,
        )
        .unwrap()
    }

    #[test]
    fn normalize_whole_ignores_emphasis_colon_and_case() {
        assert_eq!(normalize_whole("**Propose using:**"), "propose using");
        assert_eq!(normalize_whole("  Abstract. "), "abstract");
        assert_eq!(normalize_whole("_Example_::"), "example");
    }

    #[test]
    fn whole_entry_matches_only_the_whole_segment() {
        let g = whole_glossary();
        let found = g.find_matching_terms("Abstract");
        assert_eq!(found.get("Abstract").map(String::as_str), Some("초록"));
        // Spelled differently from the term, so the term matches as well.
        assert_eq!(found.get("abstract").map(String::as_str), Some("추상"));
        let prose = g.find_matching_terms("An abstract base class is abstract.");
        assert!(!prose.contains_key("Abstract"));
        assert_eq!(g.whole_match("**Propose using:**"), Some(("Propose using:", "제안 방법")));
        assert_eq!(g.whole_match("Propose using it"), None);
    }

    #[test]
    fn unmet_terms_demands_containment_for_terms_and_equality_for_whole() {
        let g = whole_glossary();
        assert!(g.unmet_terms("Find a sponsor.", "스폰서를 찾으십시오.").is_empty());
        assert_eq!(
            g.unmet_terms("Find a sponsor.", "후원자를 찾으십시오."),
            vec![("sponsor".to_string(), "스폰서".to_string())]
        );
        assert!(g.unmet_terms("**Propose using:**", "**제안 방법:**").is_empty());
        assert_eq!(
            g.unmet_terms("Propose using:", "제안 방법은 다음과 같습니다:"),
            vec![("Propose using:".to_string(), "제안 방법".to_string())]
        );
    }

    #[test]
    fn whole_entries_whose_rendering_ends_in_punctuation_are_met() {
        let g = Glossary::from_toml("[whole.\"Undocumented.\"]\ntranslation = \"문서화되어 있지 않습니다.\"\n").unwrap();
        assert!(g.unmet_terms("Undocumented.", "문서화되어 있지 않습니다.").is_empty());
        assert!(!g.unmet_terms("Undocumented.", "문서화되지 않았습니다.").is_empty());
    }

    #[test]
    fn curated_glossary_wins_over_generated_terms() {
        let curated: GlossaryFile = toml::from_str(
            r#"
[terms.Sponsor]
translation = "후원자"
[whole.abstract]
translation = "개요"
"#,
        )
        .unwrap();
        let mut generated = TermsFile::default();
        let entry = |t: &str| GlossaryEntry { translation: t.to_string(), note: None, except: vec![] };
        generated.terms.insert("sponsor".into(), entry("스폰서"));
        generated.terms.insert("query".into(), entry("쿼리"));
        generated.whole.insert("Abstract:".into(), entry("초록"));
        let g = Glossary::merged(curated, generated);
        assert_eq!(g.find_matching_terms("a sponsor").get("Sponsor").map(String::as_str), Some("후원자"));
        assert!(!g.find_matching_terms("a sponsor").contains_key("sponsor"));
        assert_eq!(g.find_matching_terms("a query").get("query").map(String::as_str), Some("쿼리"));
        assert_eq!(g.whole_match("Abstract"), Some(("abstract", "개요")));
    }

    #[test]
    fn terms_file_round_trips_sorted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("terms.toml");
        let mut file = TermsFile::default();
        let entry = |t: &str| GlossaryEntry { translation: t.to_string(), note: None, except: vec![] };
        file.terms.insert("zeta".into(), entry("제타"));
        file.terms.insert("alpha".into(), entry("알파"));
        file.whole.insert("Abstract".into(), entry("초록"));
        file.rejected.terms.insert("make sure".into());
        file.rejected.whole.insert("none".into());
        file.save(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("# Project term base."));
        assert!(text.find("[terms.alpha]").unwrap() < text.find("[terms.zeta]").unwrap());
        let back = TermsFile::load(&path).unwrap();
        assert_eq!(back.terms["alpha"].translation, "알파");
        let (terms, whole) = back.decided_keys();
        assert!(terms.contains("zeta") && terms.contains("make sure"));
        assert!(whole.contains("abstract") && whole.contains("none"));
    }

    #[test]
    fn load_project_reads_both_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("glossary.toml"), "[terms.commit]\ntranslation = \"커밋\"\n").unwrap();
        std::fs::write(dir.path().join("terms.toml"), "[whole.References]\ntranslation = \"참고 문헌\"\n").unwrap();
        let settings: crate::config::ProjectSettings =
            toml::from_str("source_lang = \"en\"\ntarget_lang = \"ko\"\n").unwrap();
        let g = Glossary::load_project(dir.path(), &settings).unwrap();
        assert_eq!(g.terms().get("commit").map(String::as_str), Some("커밋"));
        assert_eq!(g.whole_match("References"), Some(("References", "참고 문헌")));
    }

    #[test]
    fn tex_control_words_are_not_terms() {
        let g = Glossary::from_toml("[terms.code]\ntranslation = \"코드\"\n").unwrap();
        assert!(g.find_matching_terms("Install \\code{sbt} first.").is_empty());
        assert!(g.find_matching_terms("The code is short.").contains_key("code"));
    }

    #[test]
    fn path_segments_joined_by_double_colons_are_not_terms() {
        let g = Glossary::from_toml("[terms.camera]\ntranslation = \"카메라\"\n").unwrap();
        assert!(g.find_matching_terms("camera::ray_color() with depth limiting").is_empty());
        assert!(g.find_matching_terms("see render::camera for details").is_empty());
        assert!(g.find_matching_terms("Move the camera: then render.").contains_key("camera"));
    }

    #[test]
    fn cross_reference_labels_are_not_terms() {
        let g = Glossary::from_toml("[terms.types]\ntranslation = \"타입\"\n[terms.groups]\ntranslation = \"군\"\n").unwrap();
        assert!(g.find_matching_terms("empty type, \\pg{sec:coproduct-types}").is_empty());
        assert!(g.find_matching_terms("part:groups").is_empty());
        assert!(g.find_matching_terms("\\pg{groups:intro}").is_empty());
        assert!(g.find_matching_terms("Note: groups act on sets.").contains_key("groups"));
        assert!(g.find_matching_terms("product-types are types").contains_key("types"));
    }
}
