use Token::{MinLengthWildcard, ExactLengthWildcard, Literal};
use GlobParseError::{UnknownEscapeSequence, UnterminatedEscapeSequence};
use crate::multislice::MultiSlice;

#[derive(Debug, PartialEq, Eq)]
pub enum Token<'g> {
    ExactLengthWildcard(usize), // length
    MinLengthWildcard(usize), // minimum length
    Literal(MultiSlice<'g>),
}

/// returned if parsing a glob string fails, e.g.:
/// ```
/// # use glob::ParsedGlobString;
/// # use glob::GlobParseError;
/// let pattern = ParsedGlobString::try_from("Foo\\n");
/// assert!(pattern.is_err());
/// assert_eq!(pattern.unwrap_err(), GlobParseError::UnknownEscapeSequence(3, "\\n"));
/// ```
#[derive(Debug, PartialEq, Eq)]
pub enum GlobParseError<'g> {
    /// returned when there is an unsupported escape sequence, i.e. a (unescaped) backslash
    /// any character other than `*`, `?` or `\`. Encapsulates the index at which the escape
    /// sequence is found in the pattern string and the escape sequence itself.
    UnknownEscapeSequence(usize, &'g str), //index, escape sequence
    /// returned when there is an unescaped backslash at the end of the pattern string. Encapsulates
    /// the index at which the offending backslash is in the pattern string.
    UnterminatedEscapeSequence(usize), // index
}

fn wildcard_for_character<'g>(c : char) -> Token<'g> {
    match c {
        '*' => MinLengthWildcard(0),
        '?' => ExactLengthWildcard(1),
        _ => panic!("character {} does not stand for a wildcard", c)
    }
}

enum ParserState {
    ExpectNew,
    BorrowedLiteral(usize, usize), // start, end index in the parsed string
    ExpectEscapedCharacter
}

fn merge_wildcard_tokens<'g>(token1: Token, token2: Token) -> Token<'g> {
    match (token1, token2) {
        (ExactLengthWildcard(length1), ExactLengthWildcard(length2)) => ExactLengthWildcard(length1 + length2),
        (MinLengthWildcard(min_length1) | ExactLengthWildcard(min_length1), MinLengthWildcard(min_length2) | ExactLengthWildcard(min_length2)) => {
            MinLengthWildcard(min_length1 + min_length2)
        },
        (token1, token2) => panic!("one of the tokens is not a wildcard: {:?}, {:?}", token1, token2),
    }
}

fn append_wildcard_to_token_sequence<'g>(token_sequence : &mut Vec<Token<'g>>, token: Token<'g>) {
    let last_token : Option<Token<'g>> = token_sequence.pop();
    match last_token {
        Option::None => token_sequence.push(token),
        Option::Some(last_token) => match last_token {
            Literal(_) => {
                token_sequence.push(last_token);
                token_sequence.push(token);
            },
            wildcard => token_sequence.push(merge_wildcard_tokens(wildcard, token)),
        },
    }
}
fn append_literal_to_token_sequence<'g>(token_sequence: &mut Vec<Token<'g>>, literal: &'g str) {
    let last_token = token_sequence.last_mut();
    match last_token {
        Option::None => {
            let literal_token = Literal(MultiSlice::from(literal));
            token_sequence.push(literal_token);
        },
        Option::Some(last_token) => match last_token {
            Literal(multi_slice) => multi_slice.push(literal),
            ExactLengthWildcard(_) | MinLengthWildcard(_) => {
                token_sequence.push(Literal(MultiSlice::from(literal)))
            }
        }
    }
}

pub fn parse_glob_string(str: &str) -> Result<Vec<Token>, GlobParseError> {
    let mut output = Vec::new();
    let mut parser_state = ParserState::ExpectNew;
    for (i, c) in str.chars().enumerate() {
        match c {
            '*' | '?' => match parser_state {
                ParserState::ExpectNew => append_wildcard_to_token_sequence(&mut output, wildcard_for_character(c)),
                ParserState::BorrowedLiteral(start, end) => {
                    append_literal_to_token_sequence(&mut output, &str[start..end]);
                    output.push(wildcard_for_character(c));
                    parser_state = ParserState::ExpectNew;
                }
                ParserState::ExpectEscapedCharacter => {
                    parser_state = ParserState::BorrowedLiteral(i, i + 1);
                },
                // ParserState::ChangedLiteral(changed_literal) => {
                //     append_literal_to_token_sequence(&mut output, )
                //     output.push(Token::ChangedLiteral(changed_literal));
                //     output.push(wildcard_for_character(c));
                //     parser_state = ParserState::ExpectNew;
                // }
                // ParserState::ChangedEscaped(mut changed_literal) => {
                //     changed_literal.push(c);
                //     parser_state = ParserState::ChangedLiteral(changed_literal);
                // }
            },
            '\\' => {
                match parser_state {
                    ParserState::ExpectNew => {
                        parser_state = ParserState::ExpectEscapedCharacter
                    },
                    ParserState::BorrowedLiteral(start, end) => {
                        append_literal_to_token_sequence(&mut output, &str[start..end]);
                        parser_state = ParserState::ExpectEscapedCharacter
                    },
                    ParserState::ExpectEscapedCharacter => {
                        parser_state = ParserState::BorrowedLiteral(i, i+1);
                    },
                    // ParserState::ChangedLiteral(changed_literal) => {
                    //     parser_state = ParserState::ChangedEscaped(changed_literal);
                    // },
                    // ParserState::ChangedEscaped(mut changed_literal) => {
                    //     changed_literal.push(c);
                    //     parser_state = ParserState::ChangedLiteral(changed_literal);
                    // }
                }
            },
            _ => {
                match parser_state {
                    ParserState::ExpectNew => {
                        parser_state = ParserState::BorrowedLiteral(i, i+1);
                    },
                    ParserState::BorrowedLiteral(start, _) => {
                        parser_state = ParserState::BorrowedLiteral(start, i + 1);
                    },
                    // ParserState::ChangedLiteral(mut changed_string) => {
                    //     changed_string.push(c);
                    //     parser_state = ParserState::ChangedLiteral(changed_string);
                    // },
                    ParserState::ExpectEscapedCharacter => {
                        return Result::Err(UnknownEscapeSequence(i-1, &str[i - 1..=i]));
                    },
                }
            }
        }
    } // end of for loop

    // append the current state as token
    match parser_state {
        ParserState::ExpectNew => {},
        ParserState::BorrowedLiteral(start, end) => append_literal_to_token_sequence(&mut output, &str[start..end]),
        //ParserState::ChangedLiteral(changed_string) => output.push(Token::ChangedLiteral(changed_string)),
        ParserState::ExpectEscapedCharacter => return Result::Err(UnterminatedEscapeSequence(str.len() - 1)),
    }

    return Result::Ok(output);

}


#[cfg(test)]
mod tests {
    use super::GlobParseError;
    use super::GlobParseError::*;
    use super::{Token};
    use super::{parse_glob_string};
    use super::Token::{Literal, MinLengthWildcard, ExactLengthWildcard};
    use core::iter::zip;
    use super::MultiSlice;

    fn test_single_token(glob_string: &str, token: Token) {
        let mut tokens = Vec::new();
        tokens.push(token);
        test_multiple_tokens(glob_string, &tokens);
    }

    fn test_multiple_tokens(glob_string : &str, tokens: &[Token]) {
        let result = parse_glob_string(glob_string);
        assert!(result.is_ok());
        match result {
            Ok(token_sequence) => {
                assert_eq!(token_sequence.len(), tokens.len());
                for (actual, expected) in zip(token_sequence.into_iter(), tokens.into_iter()) {
                    assert_eq!(&actual, expected);
                }
            },
            Err(_) => panic!("previous assert should prevent this"),
        }
    }

    fn test_parse_failure(glob_string : &str, expected_error : GlobParseError) {
        let result = parse_glob_string(glob_string);
        assert!(result.is_err());
        match result {
            Ok(_) => panic!("previous assert should prevent this"),
            Err(err) => assert_eq!(err, expected_error),
        }
    }

    #[test]
    fn test_parse_only_literal() {
        test_single_token("abc", Literal(MultiSlice::from("abc")));
    }

    #[test]
    fn test_parse_only_asterisk() {
        test_single_token("*", MinLengthWildcard(0));
    }

    #[test]
    fn test_parse_only_question_mark() {
        test_single_token("?", ExactLengthWildcard(1));
    }

    #[test]
    fn test_parse_multiple_wildcards() {
        test_single_token("?*?**?", MinLengthWildcard(3));
    }

    #[test]
    fn test_mixed_wildcard_literal_wildcard() {
        test_multiple_tokens("*.yam?", &[Token::MinLengthWildcard(0), Token::Literal(MultiSlice::from(".yam")), Token::ExactLengthWildcard(1)]);
    }

    #[test]
    fn test_escaped_asterisk() {
        test_single_token("abc\\*def", Token::Literal(MultiSlice::from("abc*def")));
    }

    #[test]
    fn test_escaped_question_mark() {
        test_multiple_tokens("Hello *, how are you\\?", &[Literal(MultiSlice::from("Hello ")), Token::MinLengthWildcard(0), Token::Literal(MultiSlice::from(", how are you?"))]);
    }

    #[test]
    fn test_escaped_backslash() {
        test_single_token("\\\\", Token::Literal(MultiSlice::from("\\")));
    }

    #[test]
    fn test_failure_with_single_backslash() {
        test_parse_failure("\\", UnterminatedEscapeSequence(0));
    }

    #[test]
    fn test_failure_with_backslash_at_end() {
        test_parse_failure("abc\\", UnterminatedEscapeSequence(3));
    }

    #[test]
    fn test_failure_with_wildcards_and_backslash_at_end() {
        test_parse_failure("*-page-*.txt\\", UnterminatedEscapeSequence(12));
    }

    #[test]
    fn test_failure_with_uneven_number_of_backslashes_at_end() {
        test_parse_failure("a\\\\\\", UnterminatedEscapeSequence(3));
    }

    #[test]
    fn test_success_with_two_backslashes_at_end() {
        test_single_token("a\\\\", Token::Literal(MultiSlice::from("a\\")));
    }

    #[test]
    fn test_success_with_even_number_of_backslashes_at_end() {
        test_multiple_tokens("a\\\\\\\\", &[Literal(MultiSlice::from("a\\\\"))]);
    }

    #[test]
    fn test_failure_with_illegal_escape_sequence() {
        test_parse_failure("\\n", UnknownEscapeSequence(0, "\\n"));
    }

    #[test]
    fn test_wild_mixture() {
        let glob_str = "ab\\*c-*-?-???-?*?-de\\\\f-gh\\?i.foobar\\*?";
        let tokens = [
            Literal(MultiSlice::from("ab*c-")),
            MinLengthWildcard(0), // *
            Literal(MultiSlice::from("-")),
            ExactLengthWildcard(1), // ?
            Literal(MultiSlice::from("-")),
            ExactLengthWildcard(3), // ???
            Literal(MultiSlice::from("-")),
            MinLengthWildcard(2), // ?*?
            Literal(MultiSlice::from("-de\\f-gh?i.foobar*")),
            ExactLengthWildcard(1) // ?
        ];
        test_multiple_tokens(glob_str, &tokens);
    }

}
