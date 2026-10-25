//! `cPreprocessParser` (`system/PreprocessParser.h`): the engine's shader
//! preprocessor.
//!
//! Syntax handled (everything else is copied through as text):
//!
//! * `$name` or `$name$`: replaced with the variable's value.
//! * `@keyword ...` up to the end of the line: `@ifdef a && b`, `@elseif a || b`,
//!   `@else`, `@endif`, `@define name value`, `@include file`.
//!
//! The input is first split into symbols, exactly as the C++ does, and the
//! symbols are then evaluated. Variables are looked up and defined through a
//! [`PreprocessHost`], which lets the C++ engine keep its own variable
//! containers (`cParserVarContainer`); [`PreprocessParser`] is a pure Rust
//! host with the same lookup order as the C++ class.
//!
//! Errors and warnings are logged with the original messages.

use crate::WChar;
use crate::string::{set_file_path, to_16char, to_8char};
use std::borrow::Cow;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Host interface
// ---------------------------------------------------------------------------

/// What the parser needs from its environment.
pub trait PreprocessHost {
    /// `GetVar`: the value of a variable, or `None` if it is not defined.
    fn get_var(&self, name: &[u8]) -> Option<&[u8]>;

    /// `@define name value`: add (or change) a parsing variable.
    fn define(&mut self, name: &[u8], value: &[u8]);

    /// `@include`: the contents of the file at `path`, or `None` if it does
    /// not exist.
    fn read_include(&mut self, path: &[WChar]) -> Option<Vec<u8>>;
}

/// `cParserVarContainer`: name -> value map.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VarContainer {
    vars: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl VarContainer {
    pub fn new() -> Self {
        Self::default()
    }

    /// `Add(name, value)`: adds or changes a variable.
    pub fn add(&mut self, name: &[u8], value: &[u8]) {
        self.vars.insert(name.to_vec(), value.to_vec());
    }

    /// `Add(name, int)`: `cString::ToString(value)`.
    pub fn add_int(&mut self, name: &[u8], value: i32) {
        self.add(name, crate::string::int_to_string(value, 0).as_bytes());
    }

    /// `Add(name, float)`: `sprintf("%f")`.
    pub fn add_float(&mut self, name: &[u8], value: f32) {
        self.add(name, crate::crt::format_fixed(value as f64, 6).as_bytes());
    }

    /// `Remove`: false if the variable did not exist.
    pub fn remove(&mut self, name: &[u8]) -> bool {
        self.vars.remove(name).is_some()
    }

    /// `Get`
    pub fn get(&self, name: &[u8]) -> Option<&[u8]> {
        self.vars.get(name).map(Vec::as_slice)
    }

    /// `Clear`
    pub fn clear(&mut self) {
        self.vars.clear();
    }

    pub fn len(&self) -> usize {
        self.vars.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vars.is_empty()
    }
}

/// `cPreprocessParser` with its two variable containers. Variables are
/// looked up in the environment, then the container passed to
/// [`parse`](Self::parse), then the variables defined while parsing.
#[derive(Clone, Debug, Default)]
pub struct PreprocessParser {
    /// `GetEnvVarContainer`
    pub env: VarContainer,
    /// `GetParsingVarContainer`: cleared at the start of every parse.
    pub parsing: VarContainer,
}

struct ParserHost<'a, F> {
    parser: &'a mut PreprocessParser,
    vars: Option<&'a VarContainer>,
    read_file: F,
}

impl<F: FnMut(&[WChar]) -> Option<Vec<u8>>> PreprocessHost for ParserHost<'_, F> {
    fn get_var(&self, name: &[u8]) -> Option<&[u8]> {
        self.parser
            .env
            .get(name)
            .or_else(|| self.vars.and_then(|v| v.get(name)))
            .or_else(|| self.parser.parsing.get(name))
    }

    fn define(&mut self, name: &[u8], value: &[u8]) {
        self.parser.parsing.add(name, value);
    }

    fn read_include(&mut self, path: &[WChar]) -> Option<Vec<u8>> {
        (self.read_file)(path)
    }
}

impl PreprocessParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// `Parse`: appends the processed `input` to `output`. `read_file`
    /// returns the contents of an included file, or `None` if it does not
    /// exist. On failure `output` keeps what was produced so far.
    pub fn parse(
        &mut self,
        input: &[u8],
        output: &mut Vec<u8>,
        vars: Option<&VarContainer>,
        dir: &[WChar],
        read_file: impl FnMut(&[WChar]) -> Option<Vec<u8>>,
    ) -> bool {
        self.parsing.clear();
        let mut host = ParserHost { parser: self, vars, read_file };
        parse(input, output, dir, &mut host)
    }
}

// ---------------------------------------------------------------------------
// Symbols
// ---------------------------------------------------------------------------

/// `eParserKeyword` (values used in error messages).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
enum Keyword {
    Define = 0,
    Ifdef = 1,
    Else = 2,
    Elseif = 3,
    Endif = 4,
    Include = 5,
}

/// `eParserOperator`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operator {
    And,
    Or,
}

/// `iParserSymbol` and its subclasses.
#[derive(Clone, Debug, PartialEq, Eq)]
enum SymbolKind {
    Text(Vec<u8>),
    Keyword(Keyword),
    Operator(Operator),
    Variable(Vec<u8>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Symbol {
    kind: SymbolKind,
    row: i32,
}

impl Symbol {
    /// `eParserSymbol` value, for error messages.
    fn type_id(&self) -> i32 {
        match self.kind {
            SymbolKind::Text(_) => 0,
            SymbolKind::Keyword(_) => 1,
            SymbolKind::Operator(_) => 2,
            SymbolKind::Variable(_) => 3,
        }
    }
}

/// `eSymbolProcess`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Process {
    PureText,
    Line,
    Variable,
}

/// `CharIsVariableValid`
fn char_is_variable_valid(c: u8) -> bool {
    c.is_ascii_digit() || c.is_ascii_uppercase() || c.is_ascii_lowercase()
}

/// `StringToKeyword`
fn string_to_keyword(s: &[u8]) -> Option<Keyword> {
    Some(match s {
        b"define" => Keyword::Define,
        b"ifdef" => Keyword::Ifdef,
        b"else" => Keyword::Else,
        b"elseif" => Keyword::Elseif,
        b"endif" => Keyword::Endif,
        b"include" => Keyword::Include,
        _ => return None,
    })
}

/// `StringToOperator`
fn string_to_operator(s: &[u8]) -> Option<Operator> {
    match s {
        b"&&" => Some(Operator::And),
        b"||" => Some(Operator::Or),
        _ => None,
    }
}

struct Tokenizer<'a> {
    input: &'a [u8],
    pos: usize,
    process: Process,
    row_count: i32,
}

impl Tokenizer<'_> {
    fn end_of_input(&self) -> bool {
        self.pos >= self.input.len()
    }

    /// `GetNextString`: the next piece of input and its row. Changes
    /// `process` for the piece after it.
    fn next_string(&mut self) -> (Vec<u8>, i32) {
        let mut s = Vec::new();
        let row = self.row_count;
        loop {
            let c = self.input[self.pos];
            if c == b'\n' || c == b'\r' {
                self.row_count += 1;
            }

            if self.process != Process::PureText {
                if c == b' ' || c == b'\n' || c == b'\r' || c == b'\t' {
                    // Whitespace ends a word; a new line (or the end of a
                    // $variable) also returns to text.
                    if c == b'\n' || c == b'\r' || self.process == Process::Variable {
                        self.process = Process::PureText;
                    }
                    self.pos += 1;
                    return (s, row);
                } else if c == b'$' {
                    self.pos += 1;
                    if self.process == Process::Variable {
                        self.process = Process::PureText;
                        return (s, row);
                    }
                } else if !char_is_variable_valid(c) {
                    if self.process == Process::Variable {
                        // Not consumed: it starts the following text.
                        self.process = Process::PureText;
                        return (s, row);
                    }
                    self.pos += 1;
                    s.push(c);
                } else {
                    self.pos += 1;
                    s.push(c);
                }
            } else if c == b'@' {
                // Spaces and tabs before the '@' are dropped from the text.
                while matches!(s.last(), Some(b' ' | b'\t')) {
                    s.pop();
                }
                self.pos += 1;
                self.process = Process::Line;
                return (s, row);
            } else if c == b'$' {
                self.pos += 1;
                self.process = Process::Variable;
                return (s, row);
            } else {
                self.pos += 1;
                s.push(c);
            }

            if self.end_of_input() {
                return (s, row);
            }
        }
    }
}

/// `ParseStringToSymbol`
fn string_to_symbol(process: Process, s: Vec<u8>, row: i32) -> Option<Symbol> {
    let kind = match process {
        Process::PureText => SymbolKind::Text(s),
        Process::Variable | Process::Line => {
            if s.is_empty() || s[0] == b' ' || s[0] == b'\t' {
                return None;
            }
            if process == Process::Variable {
                SymbolKind::Variable(s)
            } else if let Some(k) = string_to_keyword(&s) {
                SymbolKind::Keyword(k)
            } else if let Some(op) = string_to_operator(&s) {
                SymbolKind::Operator(op)
            } else {
                SymbolKind::Variable(s)
            }
        }
    };
    Some(Symbol { kind, row })
}

/// Splits the input into symbols (first half of `Parse`).
fn tokenize(input: &[u8]) -> Vec<Symbol> {
    let mut t = Tokenizer { input, pos: 0, process: Process::PureText, row_count: 1 };
    let mut symbols = Vec::new();
    while !t.end_of_input() {
        let process = t.process;
        let (s, row) = t.next_string();
        symbols.extend(string_to_symbol(process, s, row));
    }
    symbols
}

// ---------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------

fn text(b: &[u8]) -> Cow<'_, str> {
    String::from_utf8_lossy(b)
}

struct Evaluator<'a, H: ?Sized> {
    symbols: Vec<Symbol>,
    /// `mSymbolIt`; the current symbol is `symbols[pos]` (none at the end).
    pos: usize,
    output: &'a mut Vec<u8>,
    dir: &'a [WChar],
    host: &'a mut H,
}

impl<H: PreprocessHost + ?Sized> Evaluator<'_, H> {
    fn end_of_symbols(&self) -> bool {
        self.pos >= self.symbols.len()
    }

    fn current(&self) -> Option<&Symbol> {
        self.symbols.get(self.pos)
    }

    /// `GetNextSymbol`
    fn next_symbol(&mut self) {
        if !self.end_of_symbols() {
            self.pos += 1;
        }
    }

    fn current_keyword_is_any(&self, keywords: &[Keyword]) -> bool {
        matches!(self.current(), Some(Symbol { kind: SymbolKind::Keyword(k), .. }) if keywords.contains(k))
    }

    /// `ParseSymbol` on the current symbol (never called at the end).
    fn parse_symbol(&mut self) -> bool {
        let symbol = self.symbols[self.pos].clone();
        match symbol.kind {
            SymbolKind::Text(t) => {
                self.output.extend_from_slice(&t);
                self.next_symbol();
                true
            }
            SymbolKind::Variable(name) => {
                match self.host.get_var(&name) {
                    Some(value) => self.output.extend_from_slice(value),
                    None => log::warn!("Parser warning: Variable '{}' on row {} was not found!", text(&name), symbol.row),
                }
                self.next_symbol();
                true
            }
            SymbolKind::Operator(_) => {
                log::error!("Parser failed: Unexpected operator on row {}!", symbol.row);
                self.next_symbol();
                true
            }
            SymbolKind::Keyword(k) => self.parse_keyword(k, symbol.row),
        }
    }

    /// `ParseBooleanDefineStatement`: `a`, `a && b && ...` or `a || b || ...`
    /// (mixing the operators is not supported, as in C++).
    fn parse_boolean_define_statement(&mut self, value: &mut bool) -> bool {
        *value = false;
        let mut prev_op = None;
        loop {
            let name = match self.current() {
                Some(Symbol { kind: SymbolKind::Variable(name), .. }) => name.clone(),
                Some(s) => {
                    log::error!("Parser failed: Invalid symbol type {} at row {}!", s.type_id(), s.row);
                    return false;
                }
                None => {
                    // Deviation: the C++ dereferences a null symbol here.
                    log::error!("Parser failed: Statement is missing a variable at the end of the input!");
                    return false;
                }
            };
            if self.host.get_var(&name).is_some() {
                if prev_op != Some(Operator::And) {
                    *value = true;
                }
            } else if prev_op == Some(Operator::And) {
                *value = false;
            }

            self.next_symbol();
            match self.current() {
                Some(Symbol { kind: SymbolKind::Operator(op), .. }) => {
                    prev_op = Some(*op);
                    self.next_symbol();
                }
                _ => break,
            }
        }
        true
    }

    /// The current symbol if it is a variable, for `@define` / `@include`.
    /// Deviation: at the end of the symbols the C++ dereferences null; here
    /// it is the same syntax error.
    fn current_variable(&self) -> Option<Vec<u8>> {
        match self.current() {
            Some(Symbol { kind: SymbolKind::Variable(name), .. }) => Some(name.clone()),
            _ => None,
        }
    }

    /// `ParseKeyword`
    fn parse_keyword(&mut self, keyword: Keyword, row: i32) -> bool {
        const BLOCK_END: [Keyword; 3] = [Keyword::Endif, Keyword::Else, Keyword::Elseif];
        match keyword {
            Keyword::Ifdef => {
                let mut statement_was_true = false;
                let mut has_found_correct_passage = false;
                let mut current_keyword = keyword;

                while current_keyword != Keyword::Endif && !self.end_of_symbols() {
                    self.next_symbol();
                    if (current_keyword == Keyword::Ifdef || current_keyword == Keyword::Elseif)
                        && !self.parse_boolean_define_statement(&mut statement_was_true)
                    {
                        return false;
                    }

                    if !has_found_correct_passage && (statement_was_true || current_keyword == Keyword::Else) {
                        // Take this passage: parse up to the next else/elseif/endif.
                        has_found_correct_passage = true;
                        while !self.end_of_symbols() && !self.current_keyword_is_any(&BLOCK_END) {
                            // Faithful: errors inside the passage are ignored.
                            self.parse_symbol();
                        }
                    } else {
                        // Skip to the next else/elseif/endif of this level.
                        let mut ifdef_count = 0;
                        while !self.end_of_symbols() && (!self.current_keyword_is_any(&BLOCK_END) || ifdef_count > 0) {
                            if let Some(Symbol { kind: SymbolKind::Keyword(k), .. }) = self.current() {
                                if *k == Keyword::Ifdef {
                                    ifdef_count += 1;
                                } else if *k == Keyword::Endif {
                                    ifdef_count -= 1;
                                }
                            }
                            self.next_symbol();
                        }
                    }
                    if let Some(Symbol { kind: SymbolKind::Keyword(k), .. }) = self.current() {
                        current_keyword = *k;
                    }
                }
                self.next_symbol();
            }
            Keyword::Define => {
                let mut parts = Vec::with_capacity(2);
                for _ in 0..2 {
                    self.next_symbol();
                    match self.current_variable() {
                        Some(v) => parts.push(v),
                        None => {
                            log::error!("Parser failed: Keyword define at row {row} does not meet syntax: '@define var value'!");
                            return false;
                        }
                    }
                }
                self.host.define(&parts[0], &parts[1]);
                self.next_symbol();
            }
            Keyword::Include => {
                self.next_symbol();
                let Some(file) = self.current_variable() else {
                    log::error!("Parser failed: Keyword include at row {row} does not meet syntax: '@include file.ext'!");
                    return false;
                };
                let file_w: Vec<WChar> = to_16char(&file);
                let path = if !self.dir.is_empty() { set_file_path(&file_w, self.dir) } else { file_w };

                match self.host.read_include(&path) {
                    Some(data) => self.output.extend_from_slice(&data),
                    None => {
                        log::error!(
                            "Parser failed: Could not find include file '{}' in path: '{}' at line {}!",
                            text(&file),
                            text(&to_8char(&path)),
                            row
                        );
                        return false;
                    }
                }
                self.next_symbol();
            }
            Keyword::Else | Keyword::Elseif | Keyword::Endif => {
                log::error!("Parser failed: Keyword {} at row {} is not valid!", keyword as i32, row);
                return false;
            }
        }
        true
    }
}

/// `cPreprocessParser::Parse` with the variables and files provided by
/// `host`: appends the processed `input` to `output`. `dir` is the
/// directory of `@include` files ("" for the current directory).
///
/// Returns false on a syntax error or a missing include file; `output`
/// then keeps what was produced up to the error.
pub fn parse<H: PreprocessHost + ?Sized>(input: &[u8], output: &mut Vec<u8>, dir: &[WChar], host: &mut H) -> bool {
    let symbols = tokenize(input);
    if symbols.is_empty() {
        // Deviation: the C++ goes on to dereference the end of its symbol
        // list (which an optimised build survives, returning true).
        log::error!("Parser failed: No symbols to parse!");
        return true;
    }

    let mut eval = Evaluator { symbols, pos: 0, output, dir, host };
    while !eval.end_of_symbols() {
        if !eval.parse_symbol() {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chars::widen;

    fn run(input: &str, vars: &[(&str, &str)]) -> (String, bool) {
        let mut container = VarContainer::new();
        for (k, v) in vars {
            container.add(k.as_bytes(), v.as_bytes());
        }
        let mut parser = PreprocessParser::new();
        let mut out = Vec::new();
        let ok = parser.parse(input.as_bytes(), &mut out, Some(&container), &[], |path| {
            (to_8char(path) == b"inc.glsl").then(|| b"INCLUDED".to_vec())
        });
        (String::from_utf8(out).unwrap(), ok)
    }

    #[test]
    fn symbols() {
        let s = tokenize(b"a $x$b\n@ifdef A && B\nc");
        let kinds: Vec<_> = s.iter().map(|s| (s.kind.clone(), s.row)).collect();
        assert_eq!(
            kinds,
            vec![
                (SymbolKind::Text(b"a ".to_vec()), 1),
                (SymbolKind::Variable(b"x".to_vec()), 1),
                (SymbolKind::Text(b"b\n".to_vec()), 1),
                (SymbolKind::Keyword(Keyword::Ifdef), 2),
                (SymbolKind::Variable(b"A".to_vec()), 2),
                (SymbolKind::Operator(Operator::And), 2),
                (SymbolKind::Variable(b"B".to_vec()), 2),
                (SymbolKind::Text(b"c".to_vec()), 3),
            ]
        );
    }

    #[test]
    fn variables_and_defines() {
        assert_eq!(run("x=$v;", &[("v", "1.5")]), ("x=1.5;".into(), true));
        assert_eq!(run("x=$v$y", &[("v", "7")]), ("x=7y".into(), true));
        assert_eq!(run("@define A 3\n$A", &[]), ("3".into(), true));
        assert_eq!(run("[$missing]", &[]), ("[]".into(), true));
    }

    #[test]
    fn conditionals() {
        let src = "@ifdef A\na\n@elseif B\nb\n@else\nc\n@endif\nend";
        assert_eq!(run(src, &[("A", "")]).0, "a\nend");
        assert_eq!(run(src, &[("B", "")]).0, "b\nend");
        assert_eq!(run(src, &[]).0, "c\nend");
        let and = "@ifdef A && B\nyes\n@endif\n";
        assert_eq!(run(and, &[("A", "")]).0, "");
        assert_eq!(run(and, &[("A", ""), ("B", "")]).0, "yes\n");
        let or = "@ifdef A || B\nyes\n@endif\n";
        assert_eq!(run(or, &[("B", "")]).0, "yes\n");
        let nested = "@ifdef X\n@ifdef A\n1\n@else\n2\n@endif\n@else\n3\n@endif\n";
        assert_eq!(run(nested, &[]).0, "3\n");
        assert_eq!(run(nested, &[("X", "")]).0, "2\n");
        // Indentation before '@' is dropped.
        assert_eq!(run("a\n  @ifdef A\nb\n  @endif\n", &[]).0, "a\n");
    }

    #[test]
    fn include_and_errors() {
        assert_eq!(run("<\n@include inc.glsl\n>", &[]), ("<\nINCLUDED>".into(), true));
        assert_eq!(run("<\n@include none.glsl\n>", &[]), ("<\n".into(), false));
        assert_eq!(run("a\n@endif\nb", &[]), ("a\n".into(), false));
        assert_eq!(run("@define A\nb", &[]), ("".into(), false));
        assert_eq!(run("", &[]), ("".into(), true));
        assert_eq!(run("@define", &[]), ("".into(), false));
        assert_eq!(run("@ifdef", &[]), ("".into(), false));
    }

    #[test]
    fn include_path() {
        let dir: Vec<WChar> = widen(b"shaders");
        let mut seen = Vec::new();
        let mut p = PreprocessParser::new();
        p.parse(b"@include a.glsl\n", &mut Vec::new(), None, &dir, |path| {
            seen = to_8char(path);
            Some(Vec::new())
        });
        assert_eq!(seen, b"shaders/a.glsl");
    }
}
