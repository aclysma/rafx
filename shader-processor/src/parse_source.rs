use fnv::FnvHashSet;
use std::collections::VecDeque;
use std::ops::Range;
use std::path::{Path, PathBuf};

use super::IncludeType;

fn range_of_line_at_position(
    code: &[char],
    position: usize,
) -> Range<usize> {
    let mut begin_of_line = position;
    let mut end_of_line = position;

    for i in position..code.len() {
        end_of_line = i + 1;
        if code[i] == '\n' {
            break;
        }
    }

    if position > 0 {
        for i in (0..=position - 1).rev() {
            if code[i] == '\n' {
                break;
            }

            begin_of_line = i;
        }
    }

    begin_of_line..end_of_line
}

pub(crate) fn skip_whitespace(
    code: &[char],
    position: &mut usize,
) {
    *position = next_non_whitespace(code, *position);
}

pub(crate) fn next_non_whitespace(
    code: &[char],
    mut position: usize,
) -> usize {
    for i in position..code.len() {
        match code[position] {
            ' ' | '\t' | '\r' | '\n' => {}
            _ => break,
        }
        position = i + 1;
    }

    position
}

// fn next_whitespace(
//     code: &[char],
//     mut position: usize,
// ) -> usize {
//     for i in position..code.len() {
//         match code[position] {
//             ' ' | '\t' | '\r' | '\n' => break,
//             _ => { },
//         }
//         position = i + 1;
//     }
//
//     position
// }

// I'm ignoring that identifiers usually can't start with numbers
pub(crate) fn is_identifier_char(c: char) -> bool {
    if c >= 'a' && c <= 'z' {
    } else if c >= 'A' && c <= 'Z' {
    } else if is_number_char(c) {
    } else if c == '_' {
    } else {
        return false;
    }

    return true;
}

// I'm ignoring that identifiers usually can't start with numbers
pub(crate) fn is_number_char(c: char) -> bool {
    c >= '0' && c <= '9'
}

pub(crate) fn next_non_identifer(
    code: &[char],
    mut position: usize,
) -> usize {
    for i in position..code.len() {
        if !is_identifier_char(code[position]) {
            break;
        }
        position = i + 1;
    }

    position
}

// pub(crate) fn next_word_at_position(
//     code: &[char],
//     mut position: usize,
// ) -> String {
//     let begin = next_non_whitespace(code, position);
//     let end = next_whitespace(code, begin);
//     characters_to_string(&code[begin..end])
// }

// pub(crate) fn identifier_at_position(
//     code: &[char],
//     mut position: usize,
// ) -> String {
//     let begin = next_non_whitespace(code, position);
//
//
// }

fn next_char(
    code: &[char],
    mut position: usize,
    search_char: char,
) -> usize {
    for i in position..code.len() {
        if code[position] == search_char {
            break;
        }

        position = i + 1;
    }

    position
}

pub(crate) fn try_consume_identifier(
    code: &[char],
    position: &mut usize,
) -> Option<String> {
    let begin = next_non_whitespace(code, *position);

    if begin < code.len() && is_identifier_char(code[begin]) {
        let end = next_non_identifer(code, begin);
        *position = end;
        Some(characters_to_string(&code[begin..end]))
    } else {
        None
    }
}

pub(crate) fn try_consume_array_index(
    code: &[char],
    position: &mut usize,
) -> Option<usize> {
    let begin = next_non_whitespace(code, *position);
    if begin < code.len() && is_number_char(code[begin]) {
        let end = next_non_identifer(code, begin);

        // If this fails, then we may have a string like "123xyz"
        let number: usize = characters_to_string(&code[begin..end]).parse().ok()?;

        *position = end;
        Some(number)
    } else {
        None
    }
}

// Return option so we can do .ok_or("error message")?
pub(crate) fn try_consume_literal(
    code: &[char],
    position: &mut usize,
    literal: &str,
) -> Option<()> {
    if is_string_at_position(code, *position, literal) {
        *position += literal.len();
        Some(())
    } else {
        None
    }
}

pub(crate) fn characters_to_string(characters: &[char]) -> String {
    let mut string = String::with_capacity(characters.len());
    for &c in characters {
        string.push(c);
    }

    string
}

pub(crate) fn is_string_at_position(
    code: &[char],
    position: usize,
    s: &str,
) -> bool {
    if code.len() < s.len() + position {
        return false;
    }

    for (index, c) in s.to_string().chars().into_iter().enumerate() {
        if code[position + index] != c {
            return false;
        }
    }

    return true;
}

fn remove_line_continuations(code: &[char]) -> Vec<char> {
    let mut result = Vec::with_capacity(code.len());

    let mut previous_non_whitespace = None;
    let mut consecutive_whitespace_character_count = 0;
    for &c in code.iter() {
        match c {
            '\n' => {
                if previous_non_whitespace == Some('\\') {
                    // Pop off any whitespace that came after the \ and the \ itself
                    for _ in 0..=consecutive_whitespace_character_count {
                        result.pop();
                    }

                    consecutive_whitespace_character_count = 0;
                } else {
                    result.push(c);
                }
                previous_non_whitespace = None;
            }
            c @ ' ' | c @ '\t' | c @ '\r' => {
                consecutive_whitespace_character_count += 1;
                result.push(c);
            }
            c @ _ => {
                // Cache what the previous non-whitespace was
                previous_non_whitespace = Some(c);
                consecutive_whitespace_character_count = 0;
                result.push(c);
            }
        }
    }

    result
}

#[derive(Debug)]
pub struct CommentText {
    pub position: usize,
    pub text: Vec<char>,
}

struct RemoveCommentsResult {
    without_comments: Vec<char>,
    comments: VecDeque<CommentText>,
}

fn remove_comments(code: &[char]) -> RemoveCommentsResult {
    let mut in_single_line_comment = false;
    let mut in_multiline_comment = false;
    let mut skip_this_character = false;
    let mut skip_this_character_in_comment_text = false;
    let mut in_string = false;
    let mut without_comments: Vec<char> = Vec::with_capacity(code.len());
    let mut comments = VecDeque::<CommentText>::default();
    let mut comment_text = Vec::<char>::default();
    let mut was_in_comment = false;

    let mut previous_character = None;
    for &c in code.iter() {
        match c {
            '"' => {
                // Begin/end string literals
                if !in_single_line_comment && !in_multiline_comment {
                    in_string = !in_string;
                }
            }
            '\n' => {
                // End single-line comments
                if in_single_line_comment {
                    in_single_line_comment = false;
                    // Don't include the * in the comment text
                    skip_this_character_in_comment_text = true;
                    //skip_this_character = true;
                    // But do add the newline to the code without comments
                    //without_comments.push('\n');
                }
            }
            '/' => {
                if !in_single_line_comment && !in_string {
                    if in_multiline_comment {
                        // End multi-line comment
                        if previous_character == Some('*') {
                            in_multiline_comment = false;
                            // Don't include the / in the resulting code
                            skip_this_character = true;
                            // Remove the * from the comment text
                            comment_text.pop();
                        }
                    } else {
                        // Start a single line comment
                        if previous_character == Some('/') {
                            in_single_line_comment = true;
                            // Remove the / before this
                            without_comments.pop();
                            //// Add a space where comments are to produce correct tokenization
                            //without_comments.push(' ');
                            // Don't include the / in the comment text
                            skip_this_character_in_comment_text = true;
                        }
                    }
                }
            }
            '*' => {
                // Start multi-line comment
                if !in_single_line_comment
                    && !in_multiline_comment
                    && !in_string
                    && previous_character == Some('/')
                {
                    in_multiline_comment = true;
                    // Remove the / before this
                    without_comments.pop();
                    //// Add a space where comments are to produce correct tokenization
                    //without_comments.push(' ');
                    // Don't include the * in the comment text
                    skip_this_character_in_comment_text = true;
                }
            }
            _ => {}
        }

        let in_comment = in_multiline_comment || in_single_line_comment;

        if in_comment && !skip_this_character_in_comment_text {
            comment_text.push(c);
        }

        if !in_comment && !comment_text.is_empty() {
            // If we have comment text we've been accumulating, store it
            let mut text = Vec::default();
            std::mem::swap(&mut text, &mut comment_text);
            comments.push_back(CommentText {
                position: without_comments.len(),
                text,
            });
        }

        if was_in_comment && !in_comment {
            // Add a space where comments are to produce correct tokenization
            without_comments.push(' ');
        }

        if !in_comment && !skip_this_character {
            without_comments.push(c);
        }

        skip_this_character = false;
        skip_this_character_in_comment_text = false;
        previous_character = Some(c);

        if was_in_comment && !in_comment {
            // Hack to handle /**//**/ appearing like a multiline comment and then a single line
            // comment. If we end a comment, then the previous input has been consumed and we should
            // not refer back to it to start a new one.
            previous_character = None;
        }

        was_in_comment = in_comment;
    }

    RemoveCommentsResult {
        without_comments,
        comments,
    }
}

#[derive(PartialEq, Debug)]
struct ParseIncludeResult {
    end_position: usize,
    include_type: IncludeType,
    path: PathBuf,
}

fn try_parse_include(
    code: &[char],
    mut position: usize,
) -> Option<ParseIncludeResult> {
    if position >= code.len() {
        return None;
    }

    if code[position] != '#' {
        // Quick early out, we only do detection if we are at the start of a # directive
        return None;
    }

    // Find start and end of current line
    let line_range = range_of_line_at_position(code, position);

    let first_char = next_non_whitespace(code, line_range.start);
    if position != first_char {
        // We found non-whitespace in front of the #, bail
        None
    } else {
        // Consume the #
        position += 1;

        // Try to find the "include" after the #
        position = next_non_whitespace(code, position);
        if try_consume_literal(code, &mut position, "include").is_some() {
            skip_whitespace(code, &mut position);

            match code[position] {
                '"' => {
                    let end = next_char(code, position + 1, '"');
                    let as_str = characters_to_string(&code[(position + 1)..end]);
                    Some(ParseIncludeResult {
                        end_position: line_range.end,
                        include_type: IncludeType::Relative,
                        path: as_str.into(),
                    })
                }
                '<' => {
                    let end = next_char(code, position + 1, '>');
                    let as_str = characters_to_string(&code[(position + 1)..end]);
                    Some(ParseIncludeResult {
                        end_position: line_range.end,
                        include_type: IncludeType::Standard,
                        path: as_str.into(),
                    })
                }
                _ => None,
            }
        } else {
            None
        }
    }
}

fn try_consume_preprocessor_directive(
    code: &[char],
    position: usize,
) -> Option<usize> {
    assert!(position < code.len());

    if code[position] != '#' {
        // Quick early out, we only do detection if we are at the start of a # directive
        return None;
    }

    // Find start and end of current line
    let line_range = range_of_line_at_position(code, position);

    let first_char = next_non_whitespace(code, line_range.start);
    if position != first_char {
        // We found non-whitespace in front of the #, bail
        None
    } else {
        //println!("preprocessor directive at {:?}", line_range);
        //print_range(code, &line_range);
        Some(line_range.end)
    }
}

fn try_consume_declaration(
    code: &[char],
    position: usize,
) -> Option<usize> {
    assert!(position < code.len());
    if !is_string_at_position(code, position, "layout")
        && !is_string_at_position(code, position, "struct")
        && !is_string_at_position(code, position, "const")
    {
        return None;
    }

    let mut brace_count = 0;
    for i in position..code.len() {
        if code[i] == '{' {
            brace_count += 1;
        } else if code[i] == '}' {
            brace_count -= 1;
        }

        if code[i] == ';' && brace_count == 0 {
            //let range = position..(i+1);
            //println!("declaration at {:?}\n{}", range, characters_to_string(&code[range.clone()]));
            return Some(i + 1);
        }
    }

    None
}

// Skip past a curly brace block we don't recognize
fn try_consume_unknown_block(
    code: &[char],
    position: usize,
) -> Option<usize> {
    assert!(position < code.len());
    if code[position] != '{' {
        // Quick early out, we only do detection if we are at the start of a curly brace block
        return None;
    }

    let mut brace_count = 0;
    for i in position..code.len() {
        if code[i] == '{' {
            brace_count += 1;
        } else if code[i] == '}' {
            brace_count -= 1;

            if brace_count == 0 {
                //let range = position..(i+1);
                //println!("unknown block at {:?}", range);
                //print_range(code, &range);
                return Some(i + 1);
            }
        }
    }

    None
}

fn find_annotations_in_comments(comments: &[CommentText]) -> Vec<AnnotationText> {
    let mut previous_character = None;
    let mut in_annotation = false;
    let mut bracket_count = 0;
    let mut skip_this_character = false;

    let mut annotations = Vec::default();
    let mut annotation = Vec::<char>::default();

    for comment in comments {
        for &c in &comment.text {
            match c {
                '[' => {
                    if !in_annotation && bracket_count == 0 && previous_character == Some('@') {
                        skip_this_character = true;
                        in_annotation = true;
                    }

                    if in_annotation {
                        bracket_count += 1;
                    }
                }
                ']' => {
                    if in_annotation {
                        bracket_count -= 1;
                        if bracket_count == 0 {
                            in_annotation = false;

                            let mut text = Vec::default();
                            std::mem::swap(&mut text, &mut annotation);
                            annotations.push(AnnotationText {
                                position: comment.position,
                                text,
                            });
                        }
                    }
                }
                _ => {}
            }

            if in_annotation && !skip_this_character {
                annotation.push(c);
            }

            previous_character = Some(c);
            skip_this_character = false;
        }

        // Insert whitespace between contiguous comment runs
        annotation.push(' ');

        // Don't allow splitting @ and [ across comments
        previous_character = None;
    }

    annotations
}

#[derive(Debug)]
pub struct FileToProcess {
    pub path: PathBuf,
    pub include_type: IncludeType,
    pub requested_from: PathBuf,
    pub include_depth: usize,
}

fn pop_comments_up_to_position(
    comments: &mut VecDeque<CommentText>,
    position: usize,
) -> Vec<CommentText> {
    let mut result = Vec::default();

    while let Some(comment) = comments.front() {
        if comment.position < position {
            result.push(comments.pop_front().unwrap());
        } else {
            break;
        }
    }

    result
}

#[derive(Debug)]
pub struct DeclarationText {
    pub text: Vec<char>,
    pub annotations: Vec<AnnotationText>,
}

#[derive(Debug)]
pub struct AnnotationText {
    pub text: Vec<char>,
    pub position: usize,
}

pub struct ShaderText {
    pub declarations: Vec<DeclarationText>,
}

pub fn parse_glsl(file_path: &Path) -> Result<ShaderText, String> {
    let first_file = FileToProcess {
        path: file_path.to_path_buf(),
        include_type: IncludeType::Relative,
        requested_from: PathBuf::new(),
        include_depth: 0,
    };

    let mut included_files = FnvHashSet::<PathBuf>::default();
    included_files.insert(file_path.to_path_buf());
    let mut declarations = Vec::default();

    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Could not read file {:?}: {:?}", file_path, e))?;
    let code: Vec<char> = content.chars().collect();
    parse_shader_source_text(&first_file, &mut declarations, &mut included_files, &code)?;

    Ok(ShaderText { declarations })
}

pub fn parse_shader_source_recursive(
    file_to_process: &FileToProcess,
    declarations: &mut Vec<DeclarationText>,
    included_files: &mut FnvHashSet<PathBuf>,
) -> Result<(), String> {
    log::trace!("parse_shader_source_recursive {:?}", file_to_process);
    let resolved_include = super::include_impl(
        &file_to_process.path,
        file_to_process.include_type,
        &file_to_process.requested_from,
        file_to_process.include_depth,
    )?;

    if included_files.contains(&resolved_include.resolved_path) {
        return Ok(());
    }

    included_files.insert(resolved_include.resolved_path.clone());

    let code: Vec<char> = resolved_include.content.chars().collect();
    parse_shader_source_text(file_to_process, declarations, included_files, &code)
}

pub(crate) fn parse_shader_source_text(
    file_to_process: &FileToProcess,
    declarations: &mut Vec<DeclarationText>,
    included_files: &mut FnvHashSet<PathBuf>,
    code: &Vec<char>,
) -> Result<(), String> {
    let code = remove_line_continuations(&code);
    let remove_comments_result = remove_comments(&code);

    let code = remove_comments_result.without_comments;
    let mut comments = remove_comments_result.comments;
    // for comment in &comments {
    //     println!("comment at {}: {:?}", comment.position, characters_to_string(&comment.text[..]));
    // }

    let mut position = 0;
    skip_whitespace(&code, &mut position);

    while position < code.len() {
        //println!("Skip forward to non-whitespace char at {}, which is {:?}", position, code[position]);

        if let Some(new_position) = try_consume_preprocessor_directive(&code, position) {
            let parse_include_result = try_parse_include(&code, position);
            if let Some(parse_include_result) = parse_include_result {
                //println!("handle include {:?}", parse_include_result);

                let included_file = FileToProcess {
                    path: parse_include_result.path,
                    include_type: parse_include_result.include_type,
                    requested_from: file_to_process.path.clone(),
                    include_depth: file_to_process.include_depth + 1,
                };

                parse_shader_source_recursive(&included_file, declarations, included_files)?;

                //println!("finish include");
            }

            position = new_position;
        } else if let Some(new_position) = try_consume_declaration(&code, position) {
            // Drain comments that we've passed and haven't taken
            let relevant_comments = pop_comments_up_to_position(&mut comments, new_position);
            let annotations = find_annotations_in_comments(&relevant_comments);
            // for comment in &relevant_comments {
            //     println!("  comment at {}: {:?}", comment.position, characters_to_string(&comment.text[..]));
            // }

            let text = code[position..new_position].iter().cloned().collect();

            declarations.push(DeclarationText {
                text,
                annotations, //comments: relevant_comments
            });
            position = new_position
        } else if let Some(new_position) = try_consume_unknown_block(&code, position) {
            position = new_position
        } else {
            // Likely this is from reading functions which are hard to detect without more detailed
            // parsing.
            //println!("Did not consume input {}, which is {:?}", position, code[position]);
            position += 1;
        }

        //println!("advanced to position {}", position);

        // Drain comments that we've passed and haven't taken
        pop_comments_up_to_position(&mut comments, position);

        skip_whitespace(&code, &mut position);
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    fn chars_to_string(chars: &[char]) -> String {
        let mut s = String::with_capacity(chars.len());
        for &c in chars {
            s.push(c);
        }
        s
    }

    #[test]
    fn test_skip_whitespace_forward() {
        let code: Vec<char> = "     text".to_string().chars().collect();
        assert_eq!(next_non_whitespace(&code, 0), 5);

        let code: Vec<char> = "text".to_string().chars().collect();
        assert_eq!(next_non_whitespace(&code, 0), 0);

        let code: Vec<char> = "a    text".to_string().chars().collect();
        assert_eq!(next_non_whitespace(&code, 1), 5);

        let code: Vec<char> = "    ".to_string().chars().collect();
        assert_eq!(next_non_whitespace(&code, 1), 4);

        let code: Vec<char> = "asdf".to_string().chars().collect();
        assert_eq!(next_non_whitespace(&code, 1), 1);
    }

    #[test]
    fn test_range_of_line_at_position_a() {
        let code: Vec<char> = "text\ntext".to_string().chars().collect();
        assert_eq!(range_of_line_at_position(&code, 0), 0..5);

        let code: Vec<char> = "text\ntext".to_string().chars().collect();
        assert_eq!(range_of_line_at_position(&code, 2), 0..5);

        let code: Vec<char> = "text\ntext".to_string().chars().collect();
        assert_eq!(range_of_line_at_position(&code, 4), 0..5);
    }

    #[test]
    fn test_range_of_line_at_position_b() {
        let code: Vec<char> = "text\ntext".to_string().chars().collect();
        assert_eq!(range_of_line_at_position(&code, 5), 5..9);

        let code: Vec<char> = "text\ntext".to_string().chars().collect();
        assert_eq!(range_of_line_at_position(&code, 7), 5..9);

        let code: Vec<char> = "text\ntext".to_string().chars().collect();
        assert_eq!(range_of_line_at_position(&code, 8), 5..9);
    }

    #[test]
    fn test_is_string_at_position_a() {
        let example: Vec<char> = "test_string".to_string().chars().collect();
        assert!(is_string_at_position(&example, 0, "test_string"));
    }

    #[test]
    fn test_is_string_at_position_b() {
        let example: Vec<char> = "test_strXng".to_string().chars().collect();
        assert!(!is_string_at_position(&example, 0, "test_string"));
    }

    #[test]
    fn test_is_string_at_position_c() {
        let example: Vec<char> = "aaaa".to_string().chars().collect();
        assert!(is_string_at_position(&example, 0, "a"));
    }

    #[test]
    fn test_is_string_at_position_d() {
        let example: Vec<char> = "a".to_string().chars().collect();
        assert!(!is_string_at_position(&example, 0, "aaaa"));
    }

    #[test]
    fn test_is_string_at_position_e() {
        let example: Vec<char> = "a".to_string().chars().collect();
        assert!(!is_string_at_position(&example, 20, "aaaa"));
    }

    #[test]
    fn test_is_string_at_position_f() {
        let example: Vec<char> = "a".to_string().chars().collect();
        assert!(!is_string_at_position(&example, 3, ""));
    }

    #[test]
    fn test_is_string_at_position_g() {
        let example: Vec<char> = "abaa".to_string().chars().collect();
        assert!(!is_string_at_position(&example, 1, "a"));
    }

    #[test]
    fn test_remove_line_continuations_empty() {
        let example: Vec<char> = "".to_string().chars().collect();
        let without_comments = chars_to_string(&remove_line_continuations(&example));
        assert_eq!("", without_comments);
    }

    #[test]
    fn test_remove_line_continuations_no_continuations() {
        let example: Vec<char> = "asdf\nasdf".to_string().chars().collect();
        let without_comments = chars_to_string(&remove_line_continuations(&example));
        assert_eq!("asdf\nasdf", without_comments);
    }

    #[test]
    fn test_remove_line_continuations_simple() {
        let example: Vec<char> = "asdf\\\nasdf".to_string().chars().collect();
        let without_comments = chars_to_string(&remove_line_continuations(&example));
        assert_eq!("asdfasdf", without_comments);
    }

    #[test]
    fn test_remove_line_continuations_whitespace() {
        let example: Vec<char> = "asdf\\ \nasdf".to_string().chars().collect();
        let without_comments = chars_to_string(&remove_line_continuations(&example));
        assert_eq!("asdfasdf", without_comments);
    }

    #[test]
    fn test_remove_comments_empty() {
        let example: Vec<char> = "".to_string().chars().collect();
        let result = remove_comments(&example);
        let without_comments = chars_to_string(&result.without_comments);
        assert_eq!("", without_comments);
        assert_eq!(result.comments.len(), 0);
    }

    #[test]
    fn test_remove_comments_multiline() {
        let example: Vec<char> = "test/* */test".to_string().chars().collect();
        let result = remove_comments(&example);
        let without_comments = chars_to_string(&result.without_comments);
        assert_eq!("test test", without_comments);

        assert_eq!(1, result.comments.len());
        assert_eq!(4, result.comments[0].position);
        assert_eq!(" ", chars_to_string(&result.comments[0].text));
    }

    #[test]
    fn test_remove_comments_singleline() {
        let example: Vec<char> = "test//test \ntest".to_string().chars().collect();
        let result = remove_comments(&example);
        let without_comments = chars_to_string(&result.without_comments);
        assert_eq!("test \ntest", without_comments);

        assert_eq!(1, result.comments.len());
        assert_eq!(4, result.comments[0].position);
        assert_eq!("test ", chars_to_string(&result.comments[0].text));
    }

    #[test]
    fn test_remove_comments_single_escapes_multi() {
        let example: Vec<char> = "test// /*test \ntest*/".to_string().chars().collect();
        let result = remove_comments(&example);
        let without_comments = chars_to_string(&result.without_comments);
        assert_eq!("test \ntest*/", without_comments);

        assert_eq!(1, result.comments.len());
        assert_eq!(4, result.comments[0].position);
        assert_eq!(" /*test ", chars_to_string(&result.comments[0].text));
    }

    #[test]
    fn test_remove_comments_string_escapes_single() {
        let example: Vec<char> = "test\"// test \"\ntest".to_string().chars().collect();
        let result = remove_comments(&example);
        let without_comments = chars_to_string(&result.without_comments);
        assert_eq!("test\"// test \"\ntest", without_comments);
        assert_eq!(0, result.comments.len());
    }

    #[test]
    fn test_remove_comments_string_escapes_multi() {
        let example: Vec<char> = "test\"/* test \"\ntes*/t".to_string().chars().collect();
        let result = remove_comments(&example);
        let without_comments = chars_to_string(&result.without_comments);
        assert_eq!("test\"/* test \"\ntes*/t", without_comments);
        assert_eq!(0, result.comments.len());
    }

    #[test]
    fn test_remove_comments_single_escapes_string() {
        let example: Vec<char> = "test//\"tes\"\n\"t".to_string().chars().collect();
        let result = remove_comments(&example);
        let without_comments = chars_to_string(&result.without_comments);
        assert_eq!("test \n\"t", without_comments);

        assert_eq!(1, result.comments.len());
        assert_eq!(4, result.comments[0].position);
        assert_eq!("\"tes\"", chars_to_string(&result.comments[0].text));
    }

    #[test]
    fn test_remove_comments_multi_escapes_string() {
        let example: Vec<char> = "test/*\"tes\"\n*/\"t".to_string().chars().collect();
        let result = remove_comments(&example);
        let without_comments = chars_to_string(&result.without_comments);
        assert_eq!("test \"t", without_comments);

        assert_eq!(1, result.comments.len());
        assert_eq!(4, result.comments[0].position);
        assert_eq!("\"tes\"\n", chars_to_string(&result.comments[0].text));
    }

    #[test]
    fn test_remove_comments_empty_multiline() {
        let example: Vec<char> = "/**//* */test".to_string().chars().collect();
        let result = remove_comments(&example);
        let without_comments = chars_to_string(&result.without_comments);
        assert_eq!("  test", without_comments);

        assert_eq!(1, result.comments.len());
        assert_eq!(1, result.comments[0].position);
        assert_eq!(" ", chars_to_string(&result.comments[0].text));
    }

    #[test]
    fn test_remove_comments_complex() {
        let example: Vec<char> = "/**//* *///test\n/*\"tes\"\n*/\"t//"
            .to_string()
            .chars()
            .collect();
        let result = remove_comments(&example);
        let without_comments = chars_to_string(&result.without_comments);
        assert_eq!("   \n \"t//", without_comments);

        assert_eq!(3, result.comments.len());
        assert_eq!(1, result.comments[0].position);
        assert_eq!(" ", chars_to_string(&result.comments[0].text));
        assert_eq!(2, result.comments[1].position);
        assert_eq!("test", chars_to_string(&result.comments[1].text));
        assert_eq!(4, result.comments[2].position);
        assert_eq!("\"tes\"\n", chars_to_string(&result.comments[2].text));
    }

    #[test]
    fn test_parse_include_a() {
        let code: Vec<char> = "#include \"asdf\"".to_string().chars().collect();
        let result = try_parse_include(&code, 0);
        assert_eq!(
            result,
            Some(ParseIncludeResult {
                end_position: code.len(),
                include_type: IncludeType::Relative,
                path: "asdf".into()
            })
        );
    }

    #[test]
    fn test_parse_include_b() {
        let code: Vec<char> = "#include <asdf>".to_string().chars().collect();
        let result = try_parse_include(&code, 0);
        assert_eq!(
            result,
            Some(ParseIncludeResult {
                end_position: code.len(),
                include_type: IncludeType::Standard,
                path: "asdf".into()
            })
        );
    }

    #[test]
    fn test_parse_include_c() {
        let code: Vec<char> = "     #include \"asdf\"".to_string().chars().collect();
        let result = try_parse_include(&code, 5);
        assert_eq!(
            result,
            Some(ParseIncludeResult {
                end_position: code.len(),
                include_type: IncludeType::Relative,
                path: "asdf".into()
            })
        );
    }

    #[test]
    fn test_parse_include_d() {
        let code: Vec<char> = "   a #include \"asdf\"".to_string().chars().collect();
        let result = try_parse_include(&code, 5);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_include_e() {
        let code: Vec<char> = "#include \"asdf".to_string().chars().collect();
        let result = try_parse_include(&code, 5);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_include_f() {
        let code: Vec<char> = "#include x \"asdf\"".to_string().chars().collect();
        let result = try_parse_include(&code, 5);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_include_g() {
        let code: Vec<char> = "".to_string().chars().collect();
        let result = try_parse_include(&code, 0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_annotation_in_comments_a() {
        let comments = vec![CommentText {
            position: 0,
            text: "".to_string().chars().collect(),
        }];

        let annotations = find_annotations_in_comments(&comments);
        assert_eq!(annotations.len(), 0);
    }

    #[test]
    fn test_find_annotation_in_comments_b() {
        let comments = vec![CommentText {
            position: 0,
            text: "asdf".to_string().chars().collect(),
        }];

        let annotations = find_annotations_in_comments(&comments);
        assert_eq!(annotations.len(), 0);
    }

    #[test]
    fn test_find_annotation_in_comments_c() {
        let comments = vec![CommentText {
            position: 0,
            text: "@[".to_string().chars().collect(),
        }];

        let annotations = find_annotations_in_comments(&comments);
        assert_eq!(annotations.len(), 0);
    }

    #[test]
    fn test_find_annotation_in_comments_d() {
        let comments = vec![CommentText {
            position: 0,
            text: "@[test]".to_string().chars().collect(),
        }];

        let annotations = find_annotations_in_comments(&comments);
        assert_eq!(annotations.len(), 1);
        assert_eq!(characters_to_string(&annotations[0].text[..]), "test");
    }

    #[test]
    fn test_find_annotation_in_comments_e() {
        let comments = vec![CommentText {
            position: 0,
            text: "@[test]@[test]".to_string().chars().collect(),
        }];

        let annotations = find_annotations_in_comments(&comments);
        assert_eq!(annotations.len(), 2);
        assert_eq!(characters_to_string(&annotations[0].text[..]), "test");
        assert_eq!(characters_to_string(&annotations[1].text[..]), "test");
    }

    #[test]
    fn test_find_annotation_in_comments_f() {
        let comments = vec![CommentText {
            position: 0,
            text: "@[[[test]]]@[test]".to_string().chars().collect(),
        }];

        let annotations = find_annotations_in_comments(&comments);
        assert_eq!(annotations.len(), 2);
        assert_eq!(characters_to_string(&annotations[0].text[..]), "[[test]]");
        assert_eq!(characters_to_string(&annotations[1].text[..]), "test");
    }

    #[test]
    fn test_find_annotation_in_comments_g() {
        let comments = vec![CommentText {
            position: 0,
            text: "]".to_string().chars().collect(),
        }];

        let annotations = find_annotations_in_comments(&comments);
        assert_eq!(annotations.len(), 0);
    }

    #[test]
    fn test_find_annotation_in_comments_h() {
        let comments = vec![CommentText {
            position: 0,
            text: "@ []".to_string().chars().collect(),
        }];

        let annotations = find_annotations_in_comments(&comments);
        assert_eq!(annotations.len(), 0);
    }

    #[test]
    fn test_find_annotation_in_comments_i() {
        let comments = vec![
            CommentText {
                position: 0,
                text: "@[asdf".to_string().chars().collect(),
            },
            CommentText {
                position: 0,
                text: "asdf]".to_string().chars().collect(),
            },
        ];

        let annotations = find_annotations_in_comments(&comments);
        assert_eq!(annotations.len(), 1);
        assert_eq!(characters_to_string(&annotations[0].text[..]), "asdf asdf");
    }
}
