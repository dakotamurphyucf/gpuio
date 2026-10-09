//! Bounded native regex preparation, independent of windows and application state.
//! The compiled object stays in Rust; the OCaml preflight receives only a result.
use gpuio_protocol::input_validation::{
    Error, MAX_DIAGNOSTIC_BYTES, Matching, Preparation, Rule, Source,
};
use regex_automata::meta::{Config, Regex};
use regex_automata::nfa::thompson::WhichCaptures;
use regex_syntax::{
    ParserBuilder,
    hir::{Hir, Look},
};
use std::sync::Mutex;

const MAX_COMPILED_BYTES: usize = 256 * 1024;
const MAX_CACHE_BYTES: usize = 256 * 1024;
const MAX_NESTING: u32 = 32;

fn diagnostic(message: impl std::fmt::Display) -> Error {
    let mut message = message.to_string();
    if message.len() > MAX_DIAGNOSTIC_BYTES {
        let mut end = MAX_DIAGNOSTIC_BYTES;
        while !message.is_char_boundary(end) {
            end -= 1;
        }
        message.truncate(end);
    }
    Error::InvalidRegex(message)
}

pub(crate) fn compile(source: &Source) -> Result<Regex, Error> {
    if !source.is_valid() {
        return Err(Error::InvalidSource);
    }
    let mut parser = ParserBuilder::new()
        .nest_limit(MAX_NESTING)
        .case_insensitive(!source.case_sensitive)
        .utf8(true)
        .unicode(true)
        .build();
    let hir = parser.parse(&source.pattern).map_err(diagnostic)?;
    // Structural anchoring preserves alternation and free-spacing comments.
    // Appending a textual suffix can be swallowed by an inline `(?x)` comment;
    // checking the span of the first match mishandles `a|ab` against `ab`.
    let hir = match source.matching {
        Matching::WholeValue => {
            Hir::concat(vec![Hir::look(Look::Start), hir, Hir::look(Look::End)])
        }
        Matching::Substring => hir,
    };
    Regex::builder()
        .configure(
            Config::new()
                // Input validation never exposes capture groups. Keep only the
                // whole-match slots, so fallback caches cannot multiply NFA
                // state count by the number of user-written capture groups.
                .which_captures(WhichCaptures::Implicit)
                // Avoid a text-length-dependent visited bitmap/stack. Hybrid
                // matching remains available; Unicode fallback uses PikeVM.
                .backtrack(false)
                .nfa_size_limit(Some(MAX_COMPILED_BYTES))
                .dfa_size_limit(Some(MAX_COMPILED_BYTES))
                .onepass_size_limit(Some(MAX_COMPILED_BYTES))
                .hybrid_cache_capacity(MAX_CACHE_BYTES),
        )
        .build_from_hir(&hir)
        .map_err(|error| {
            if error.size_limit().is_some() {
                Error::TooComplex
            } else {
                diagnostic(error)
            }
        })
}

/// One accepted policy and one explicit cache, shared by the tree and editor.
/// Matching never uses Regex's implicit per-thread cache pool.
#[derive(Debug)]
pub(crate) struct Policy {
    rule: Rule,
    regex: Regex,
    cache: Mutex<regex_automata::meta::Cache>,
    retained_bytes: usize,
}

impl PartialEq for Policy {
    fn eq(&self, other: &Self) -> bool {
        self.rule == other.rule
    }
}

impl Policy {
    pub(crate) fn new(rule: Rule) -> Result<Self, Error> {
        let regex = compile(&rule.regex)?;
        let cache = regex.create_cache();
        // Reserve beyond the engine's approximate live-size report: lazy
        // PikeVM active sets/whole-match slots/epsilon-stack capacities, both
        // hybrid caches including vector growth, and owner bookkeeping. The
        // backtracker is disabled and user capture tables are not compiled.
        // This is admission accounting, not an allocator/RSS quota.
        let retained_bytes = regex
            .memory_usage()
            .saturating_mul(16)
            .saturating_add(cache.memory_usage())
            .saturating_add(4 * MAX_CACHE_BYTES)
            .saturating_add(rule.regex.pattern.len() + std::mem::size_of::<Self>() + 512);
        Ok(Self {
            rule,
            regex,
            cache: Mutex::new(cache),
            retained_bytes,
        })
    }

    pub(crate) fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }

    pub(crate) fn rule(&self) -> &Rule {
        &self.rule
    }

    pub(crate) fn accepts(&self, text: &str) -> bool {
        if text.len() > gpuio_protocol::v1::MAX_TEXT_BYTES {
            return false;
        }
        if text.is_empty() && self.rule.allow_empty {
            return true;
        }
        let mut cache = self.cache.lock().expect("input validation cache poisoned");
        self.regex
            .search_with(&mut cache, &regex_automata::Input::new(text).earliest(true))
            .is_some()
    }
}

pub(crate) fn prepare(bytes: &[u8]) -> Preparation {
    let source = match gpuio_protocol::decode_input_validation_source(bytes) {
        Ok(source) => source,
        Err(_) => return Preparation::Failed(Error::InvalidSource),
    };
    match compile(&source) {
        Ok(_) => Preparation::Checked,
        Err(error) => Preparation::Failed(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use binprot::BinProtWrite;

    fn source(pattern: &str, matching: Matching) -> Source {
        Source {
            pattern: pattern.into(),
            matching,
            case_sensitive: true,
        }
    }

    #[test]
    fn input_validation_anchoring_preserves_alternation_flags_comments_and_unicode() {
        for (pattern, text, expected) in [
            ("a|ab", "ab", true),
            ("a|ab", "abc", false),
            ("(?x)a # trailing comment", "a", true),
            ("(?x)a # trailing comment", "ab", false),
            ("(?m)^a$", "a\n", false),
            (r"\p{L}+", "世界", true),
            (r"\p{L}+", "世界1", false),
            ("", "", true),
            ("", "a", false),
        ] {
            assert_eq!(
                compile(&source(pattern, Matching::WholeValue))
                    .unwrap()
                    .is_match(text),
                expected,
                "{pattern:?}: {text:?}"
            );
        }
        assert!(
            compile(&source("a|ab", Matching::Substring))
                .unwrap()
                .is_match("zabc")
        );
        let mut insensitive = source("hello", Matching::WholeValue);
        insensitive.case_sensitive = false;
        assert!(compile(&insensitive).unwrap().is_match("HeLLo"));
        assert!(
            !compile(&source("hello", Matching::WholeValue))
                .unwrap()
                .is_match("HeLLo")
        );
    }

    #[test]
    fn input_validation_predicates_do_not_allocate_unused_capture_tables() {
        // A non-ASCII boundary and long input exercise the PikeVM fallback:
        // the bounded backtracker cannot hold this state/input product. An
        // unused cache or short input misses its lazy capture-table allocation.
        let pattern = format!(r"\bé{}b*", "(a?)".repeat(200));
        let regex = compile(&source(&pattern, Matching::WholeValue)).unwrap();
        let text = format!("é{}{}", "a".repeat(200), "b".repeat(8192));
        let mut cache = regex.create_cache();
        assert!(
            regex
                .search_with(&mut cache, &regex_automata::Input::new(&text))
                .is_some()
        );
        let cache_bytes = cache.memory_usage();
        assert!(
            cache_bytes < 1024 * 1024,
            "predicate cache uses {cache_bytes} bytes"
        );
        assert!(regex.is_match(&text));
        assert!(!regex.is_match(&format!("{text}a")));
    }

    #[test]
    fn retained_policy_owns_one_cache_and_reserves_for_representative_growth() {
        for pattern in [
            "[a-z]*".to_owned(),
            r"\p{L}*".into(),
            format!(r"\bé{}b*", "(a?)".repeat(200)),
        ] {
            let policy = Policy::new(Rule {
                regex: source(&pattern, Matching::WholeValue),
                allow_empty: true,
            })
            .unwrap();
            let reserved = policy.retained_bytes();
            for text in [
                "".to_owned(),
                "hello".into(),
                "é".repeat(4096),
                "é".repeat(131072),
                format!("é{}{}", "a".repeat(200), "b".repeat(8192)),
            ] {
                assert_eq!(
                    policy.accepts(&text),
                    text.is_empty() || compile(&policy.rule.regex).unwrap().is_match(&text)
                );
                assert!(
                    policy.regex.memory_usage() + policy.cache.lock().unwrap().memory_usage()
                        < reserved
                );
                assert_eq!(policy.retained_bytes(), reserved);
            }
            assert!(!policy.accepts(&"a".repeat(gpuio_protocol::v1::MAX_TEXT_BYTES + 1)));
        }
        let empty = |allow_empty| {
            Policy::new(Rule {
                regex: source("x+", Matching::WholeValue),
                allow_empty,
            })
            .unwrap()
        };
        assert!(empty(true).accepts(""));
        assert!(!empty(false).accepts(""));
    }

    #[test]
    fn input_validation_preflight_rejects_syntax_expansion_and_nesting_with_bounded_errors() {
        for pattern in [
            "[",
            r"(a)\1",
            "(?=a)",
            &format!("{}a{}", "(".repeat(40), ")".repeat(40)),
        ] {
            let error = compile(&source(pattern, Matching::WholeValue)).unwrap_err();
            let Error::InvalidRegex(message) = error else {
                panic!("expected syntax error")
            };
            assert!(!message.is_empty() && message.len() <= MAX_DIAGNOSTIC_BYTES);
        }
        assert!(matches!(
            compile(&source("a{100000000}", Matching::WholeValue)),
            Err(Error::TooComplex)
        ));
        assert!(matches!(
            compile(&source(&"a".repeat(2049), Matching::WholeValue)),
            Err(Error::InvalidSource)
        ));
        assert!(matches!(
            compile(&source("a\0", Matching::WholeValue)),
            Err(Error::InvalidSource)
        ));
        let mut bytes = vec![];
        source("[a-z]*", Matching::WholeValue)
            .binprot_write(&mut bytes)
            .unwrap();
        assert_eq!(prepare(&bytes), Preparation::Checked);
        for end in 0..bytes.len() {
            assert_eq!(
                prepare(&bytes[..end]),
                Preparation::Failed(Error::InvalidSource)
            );
        }
        bytes.push(0);
        assert_eq!(prepare(&bytes), Preparation::Failed(Error::InvalidSource));
    }
}
