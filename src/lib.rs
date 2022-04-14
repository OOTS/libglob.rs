//! A toy implementation of glob-style pattern matching.
//!
//! This toy library implements simple glob-style pattern matching on strings:
//! ```
//! use glob::pattern_matches_partially;
//! let matches = pattern_matches_partially("path/to/*.yaml", "path/to/foo.yaml");
//! assert_eq!(matches, Ok(true));
//! ```
//! The [`pattern_matches_partially`] function returns a [`Result<bool, GlobParseError>`],
//! since parsing the pattern can fail if the pattern is not well-formed.
//! See below for a more complete description of the pattern syntax.
//!
//! If you want to use the same pattern on multiple strings, you can parse it once and use the
//! parsed result multiple times:
//! ```
//! use glob::ParsedGlobString;
//! let pattern = ParsedGlobString::try_from("[*,*,*]").unwrap();
//! assert!(pattern.matches_partially("{\"key\": [1, 2, 3]}"));
//! assert!(!pattern.matches_partially("foo/bar.yaml"));
//! ```
//!
//! Here, [`try_from`](ParsedGlobString::try_from) returns a [`Result<ParsedGlobString, GlobParseError>`] (and you should probably
//! handle the error case properly), whereas [`pattern.matches_partially`](ParsedGlobString::matches_partially) returns a plain and simple [`bool`].
//!
//! Both the [`ParsedGlobString`] and the [`GlobParseError`] currently borrow string slices from the
//! pattern string, so they have a lifetime that is limited by the pattern string's lifetime.
//!
//! # Pattern Syntax
//!
//! The asterisk character `*` acts as a wildcard for zero or more arbitrary characters. A question
//! mark `?` is a wildcard for exactly one character.
//!
//! Both `*` and `?` can be escaped by preceding them with a backslash `\`. In this case, they do
//! not act as wildcards but only match themselves:
//! ```
//! # use glob::pattern_matches_partially;
//! assert!(pattern_matches_partially("\\*", "My favourite character is '*'.").unwrap());
//! assert!(!pattern_matches_partially("\\*", "My favourite character is '#'.").unwrap());
//! ```
//! Note that in this example, we had to write `\\` because the rust compiler performs another level
//! of unescaping.
//!
//! Apart from `*` and `?`, the only other escape character is the backslash itself: `\`. An escaped
//! backslash character in the pattern matches only a backslash character in the search string:
//! ```
//! # use glob::pattern_matches_partially;
//! assert!(pattern_matches_partially("\\\\", "Windows path separator: \\").unwrap());
//! assert!(!pattern_matches_partially("\\\\", "Linux/Unix path separator: /").unwrap());
//! ```
//!
//! Using a backslash for escaping any other character or using a backslash at the end of the
//! pattern string results in a [`GlobParseError`].


mod glob_parser;
mod multislice;
use glob_parser::*;
use glob_parser::Token::*;
pub use glob_parser::GlobParseError;

/// Represents the result of parsing a glob pattern.
///
/// Currently, this structure borrows string slices from the pattern string, so it's lifetime is
/// limited by the pattern string's lifetime `'g`.
///
/// This structure is intentionally left opaque. Use the [`try_from`](Self::try_from) method to create an instance of
/// this type and the [`matches_partially`](Self::matches_partially) method to perform pattern matching.
#[derive(Debug)]
pub struct ParsedGlobString<'g> {
    tokens: Vec<Token<'g>>,
}

impl<'g> TryFrom<&'g str> for ParsedGlobString<'g> {
    type Error = GlobParseError<'g>;
    /// parses the given `string` and returns the result or an error.
    /// ```
    /// # use glob::ParsedGlobString;
    /// let pattern = ParsedGlobString::try_from("path/to/*.txt");
    /// # assert!(pattern.is_ok());
    /// ```
    fn try_from(string: &'g str) -> Result<Self, Self::Error> {
        return parse_glob_string(string).map(|tokens| ParsedGlobString { tokens: tokens });
    }
}

impl<'g> ParsedGlobString<'g> {
    /// checks if this pattern occurs anywhere in the given string.
    /// ```
    /// use glob::ParsedGlobString;
    /// let pattern = ParsedGlobString::try_from("thesis-*.pdf").unwrap();
    /// assert!(pattern.matches_partially("My Documents/thesis/thesis-final-2.pdf"));
    /// ```
    pub fn matches_partially(&self, string : &str) -> bool {
        return token_sequence_matches_partially(self.tokens.as_slice(), string);
    }
    // FIXME: implement matches_at_start
    // FIXME: maybe implement matches_completely and matches_at_end
}

/// checks if the given pattern occurs anywhere in the given string.
///
/// This is a utility function for creating a [`ParsedGlobString`] and calling [`matches_partially`](ParsedGlobString::matches_partially)
/// on it.
///
/// Returns a [`GlobParseError`] if parsing the pattern fails.
pub fn pattern_matches_partially<'g>(pattern: &'g str, string : &str) -> Result<bool, GlobParseError<'g>> {
    ParsedGlobString::try_from(pattern).map(|pgs| pgs.matches_partially(string))
}

fn token_sequence_matches_at_start<'g>(token_sequence: &[Token<'g>], string: &str) -> bool {
    match token_sequence.split_first() {
        Option::None => true,
        Option::Some((token, rest)) => match token {
            ExactLengthWildcard(length) => {
                string.len() >= *length && token_sequence_matches_at_start(rest, &string[*length..])
            },
            Literal(literal) => {
                literal.matches_string_start(string) && token_sequence_matches_at_start(rest, &string[literal.get_combined_length()..])
            },
            MinLengthWildcard(length) => {
                // FIXME: try matching from the back
                string.len() >= *length && token_sequence_matches_partially(rest, &string[*length..])
            }
        }
    }
}

fn token_sequence_matches_partially(tokens: &[Token], string : &str) -> bool {
    match tokens.split_first() {
        Option::None => true,
        Option::Some((token, rest)) => match token {
            MinLengthWildcard(length) | ExactLengthWildcard(length) => {
                string.len() >= *length && token_sequence_matches_partially(rest, &string[*length..])
            },
            Literal(literal) => {
                // FIXME: try matching from the end
                for m in literal.find_all_occurences_in(string) {
                    if token_sequence_matches_at_start(rest,&string[m + literal.get_combined_length()..]) {
                        return true
                    }
                }
                return false
            }
        }
    }
}


#[cfg(test)]
mod test {
    use crate::{GlobParseError, ParsedGlobString, pattern_matches_partially};

    fn test_matches_partially(glob_string : &str, string: &str) {
        let pgs = ParsedGlobString::try_from(glob_string).unwrap();
        assert!(pgs.matches_partially(string));
        assert_eq!(pattern_matches_partially(glob_string, string), Ok(true));
    }

    fn test_not_matches_partially(glob_string : &str, string: &str) {
        let pgs = ParsedGlobString::try_from(glob_string).unwrap();
        assert!(!pgs.matches_partially(string));
        assert_eq!(pattern_matches_partially(glob_string, string), Ok(false));
    }

    #[test]
    fn test_literal_only_matches_partially() {
        test_matches_partially(&"bc", &"abcd");
    }

    #[test]
    fn test_literal_only_matches_partially_identical_string() {
        test_matches_partially(&"abcd", &"abcd");
    }

    #[test]
    fn test_literal_only_matches_partially_at_start() {
        test_matches_partially(&"ab", &"abc");
    }

    #[test]
    fn test_literal_only_matches_partially_at_end() {
        test_matches_partially(&"bc", &"bc");
    }

    #[test]
    fn test_empty_string_matches_partially_on_empty_string() {
        test_matches_partially(&"", &"");
    }

    #[test]
    fn test_empty_string_matches_partially_on_non_empty_string() {
        test_matches_partially(&"", &"abc");
    }

    #[test]
    fn test_asterisk_only_matches_partially_on_empty_string() {
        test_matches_partially(&"*", &"");
    }

    #[test]
    fn test_asterisk_only_matches_partially_on_non_empty_string() {
        test_matches_partially(&"*", "42");
    }

    #[test]
    fn test_question_mark_only_not_matches_partially_on_empty_string() {
        test_not_matches_partially(&"?", "");
    }

    #[test]
    fn test_question_mark_only_matches_partially_on_single_character_string() {
        test_matches_partially(&"?", &"?");
    }

    #[test]
    fn test_question_mark_only_matches_partially_on_multi_character_string() {
        test_matches_partially(&"?", "???...")
    }

    #[test]
    fn test_asterisk_and_literal_not_match_partially_on_empty_string() {
        test_not_matches_partially(&"*\\*", &"");
    }

    #[test]
    fn test_asterisk_and_literal_not_match_partially_on_substr_of_literal() {
        test_not_matches_partially(&"*abc", &"ab");
    }

    #[test]
    fn test_asterisk_and_literal_match_partially_on_literal() {
        test_matches_partially(&"*foo", &"foo");
    }

    #[test]
    fn test_asterisk_and_literal_match_partially_within() {
        test_matches_partially(&"*you", &"Do you think so?");
    }

    #[test]
    fn test_asterisk_and_literal_not_match_partially_on_unrelated_string() {
        test_not_matches_partially(&"*you", &"I don't think so.");
    }

    #[test]
    fn test_asterisk_and_literal_match_partially_at_string_end() {
        test_matches_partially(&"*otherwise\\?", &"Why do you think otherwise?");
    }

    #[test]
    fn test_question_mark_and_literal_dont_match_empty_string() {
        test_not_matches_partially(&"?a", &"");
    }

    #[test]
    fn test_question_mark_and_literal_dont_match_partially_on_literal() {
        test_not_matches_partially(&"?a", &"a");
    }

    #[test]
    fn test_question_mark_and_literal_match_partially_at_start() {
        test_matches_partially(&"?bc", "abcd");
    }

    #[test]
    fn test_question_mark_and_literal_match_partially_exact() {
        test_matches_partially(&"?bc", &"abc");
    }

    #[test]
    fn test_question_mark_and_literal_match_partially_within_string() {
        test_matches_partially(&"?cde", &"abcdef");
    }

    #[test]
    fn test_question_mark_and_literal_match_partially_at_end() {
        test_matches_partially(&"?f", &"abcdef");
    }

    #[test]
    fn test_question_mark_and_literal_not_match_partially_on_case_mismatch() {
        test_not_matches_partially(&"?AR", "foobarbaz");
    }

    #[test]
    fn test_literal_and_asterisk_not_match_partially_on_empty_string() {
        test_not_matches_partially("Letter.*", "");
    }

    #[test]
    fn test_literal_and_asterisk_not_match_on_substring() {
        test_not_matches_partially("letter*", "let");
    }

    #[test]
    fn test_literal_and_asterisk_match_partially_on_literal() {
        test_matches_partially(&"foo*", &"foo");
    }

    #[test]
    fn test_literal_and_asterisk_match_partially_within() {
        test_matches_partially(&"you*", &"Do you think so?");
    }

    #[test]
    fn test_literal_and_asterisk_not_match_partially_on_unrelated_string() {
        test_not_matches_partially(&"you*", &"I don't think so.");
    }

    #[test]
    fn test_literal_and_asterisk_match_partially_at_string_end() {
        test_matches_partially(&"otherwise\\?*", &"Why do you think otherwise?");
    }

    #[test]
    fn test_literal_and_question_mark_dont_match_empty_string() {
        test_not_matches_partially(&"a?", &"");
    }

    #[test]
    fn test_literal_and_question_mark_dont_match_partially_on_literal() {
        test_not_matches_partially(&"a?", &"a");
    }

    #[test]
    fn test_literal_and_question_mark_match_partially_at_start() {
        test_matches_partially(&"ab?", "abcd");
    }

    #[test]
    fn test_literal_and_question_mark_match_partially_exact() {
        test_matches_partially(&"ab?", &"abc");
    }

    #[test]
    fn test_literal_and_question_mark_match_partially_within_string() {
        test_matches_partially(&"cd?", &"abcdef");
    }

    #[test]
    fn test_literal_and_question_mark_match_partially_at_end() {
        test_matches_partially(&"de?", &"abcdef");
    }

    #[test]
    fn test_literal_and_question_mark_not_match_partially_on_case_mismatch() {
        test_not_matches_partially(&"AR?", "foobarbaz");
    }

    #[test]
    fn test_question_mark_and_asterisk_not_match_partially_on_empty_string() {
        test_not_matches_partially("?*", "");
        test_not_matches_partially("*?", "");
    }

    #[test]
    fn test_question_mark_and_asterisk_match_partially_on_single_char() {
        test_matches_partially("*?", "a");
        test_matches_partially("?*", "a");
    }

    #[test]
    fn test_question_mark_and_asterisk_match_partially_on_longer_strings() {
        test_matches_partially("*?", "01");
        test_matches_partially("?*", "10");
        test_matches_partially("?*", "Hello, World!");
        test_matches_partially("*?", "foo");
    }

    #[test]
    fn test_wildcards_only_on_empty_string() {
        test_matches_partially("**", "");
        test_matches_partially("****", "");
        test_not_matches_partially("??", "");
        test_not_matches_partially("?**", "");
        test_not_matches_partially("*?*", "");
        test_not_matches_partially("**?", "");
        test_not_matches_partially("??", "");
    }

    #[test]
    fn test_wildcards_only_on_single_char() {
        test_matches_partially("**", "a");
        test_matches_partially("****", "a");
        test_not_matches_partially("??", "0");
        test_matches_partially("?**", "1");
        test_matches_partially("*?*", "2");
        test_matches_partially("**?", "3");
        test_not_matches_partially("??", " ");
    }

    #[test]
    fn test_wildcard_literal_wildcard_not_matches_partially_empty_string() {
        test_not_matches_partially("*-*", "");
    }

    #[test]
    fn test_wildcard_literal_wildcard_matches_literal() {
        test_matches_partially("*de*", "de");
    }

    #[test]
    fn test_wildcard_literal_wildcard_matches_partially_at_start() {
        test_matches_partially("*.*", ".bin");
    }

    #[test]
    fn test_wildcard_literal_wildcard_matches_partially_at_end() {
        test_matches_partially("*.od?", "Spreadsheet.ods");
    }

    #[test]
    fn test_wildcard_literal_wildcard_matches_partially_within() {
        test_matches_partially("*-final.*", "thesis-final.pdf");
    }

    #[test]
    fn test_wildcard_literal_wildcard_not_matches_partially_on_unrelated_string() {
        test_not_matches_partially("*-final-2.*", "thesis-final-3.pdf");
    }

    #[test]
    fn test_complicated_patterns_matches_partially_let_statement() {
        test_matches_partially("let mut ? = ?", "let mut i = 0;");
        test_not_matches_partially("let mut ??? = ?", "let mut i = 0;");
        test_matches_partially("let mut * = *;", "let mut i : usize = 0;");
        test_matches_partially("let mut * = *", "let mut my_string = \"abc\"");
        test_matches_partially("let * = *", "let mut foo = bar");
        test_not_matches_partially("let * = *", "let a=1;");
    }

    #[test]
    fn test_complicated_patterns_match_partially_on_json() {
        test_matches_partially("\"*\": *", "{\"key\": \"value\"}");
        test_not_matches_partially("\"*\": *", "{\"key\":\"value\"");
        test_not_matches_partially("[*,*,*]", "[]");
        test_not_matches_partially("[*,*,*]", "[1]");
        test_not_matches_partially("[*,*,*]", "[1, 2]");
        test_matches_partially("[*,*,*]", "[1, 2, 3]");
    }

    #[test]
    fn test_complicated_patterns_match_partially_on_paths() {
        test_matches_partially("*.json", "foo.json");
        test_matches_partially("*.json", "folder/foo.json");
        test_matches_partially(".json", "path/to/foo.json");
        test_matches_partially("json", "path/to/json.py");
        test_not_matches_partially("*.yaml", "path/to/foo.json");
        test_matches_partially("*.yaml", "statefulset.yaml");
        test_matches_partially("*.y*ml", "path/to/deployment.yml");
        test_matches_partially(".y*ml", "path/to/daemonset.yml");
        test_matches_partially(".y*ml", "path/to/configmap.yaml");
        test_not_matches_partially("*.ods", "path/to/secret.yaml");
        test_not_matches_partially("thesis*", "path/to/netpol.yaml");
        test_matches_partially("thesis*", "path/to/thesis-final-3.pdf")
    }

    #[test]
    fn test_unknown_escape_sequence_creates_globparseerror() {
        let parsed = ParsedGlobString::try_from("\\n");
        assert!(parsed.is_err());
        assert_eq!(parsed.unwrap_err(), GlobParseError::UnknownEscapeSequence(0, "\\n"));
    }

    #[test]
    fn test_backslash_at_end_creates_globparseerror() {
        let s = "a backslash at the end: \\";
        let parsed = ParsedGlobString::try_from(s);
        assert!(parsed.is_err());
        assert_eq!(parsed.unwrap_err(), GlobParseError::UnterminatedEscapeSequence(s.len() - 1));
    }

}