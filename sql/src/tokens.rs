//! Port of `kquery/sql/src/main/kotlin/Tokens.kt`.
//!
//! The token vocabulary: a `TokenType` (Kotlin's marker `interface TokenType`)
//! implemented by three enums — `Literal`, `Keyword`, and `Symbol` — plus the
//! `Token` value type.
//!
//! ## Translation notes
//! - Kotlin's marker `interface TokenType` implemented by three `enum class`es
//!   becomes a Rust `enum TokenType` wrapping the three sub-enums (per the
//!   interface-hierarchy → enum rule, §3.1; here the implementors are themselves
//!   enums, so each is wrapped in a variant).
//! - Kotlin enum *constants* are `SCREAMING_CASE`; Rust enum *variants* are
//!   `PascalCase` (Rust convention, §3.0). `Keyword.SELECT` → `Keyword::Select`,
//!   `Symbol.LEFT_PAREN` → `Symbol::LeftParen`, etc. The canonical upper-case
//!   spelling is preserved as the string returned by `Keyword::name` and matched
//!   by `Keyword::text_of`.
//! - Kotlin builds its keyword / symbol lookup tables by reflection
//!   (`values().associateBy(...)`). Rust has no enum reflection, so the
//!   `define_keywords!` / `define_symbols!` macros declare the enum *and* its
//!   string lookup tables from a single list — one source of truth, no
//!   reflection, no external crate.

use std::fmt;

/// Marker for the kind of a token. Kotlin: `interface TokenType` implemented by
/// `Literal`, `Keyword`, and `Symbol`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenType {
    Keyword(Keyword),
    Symbol(Symbol),
    Literal(Literal),
}

/// Literal token kinds. Kotlin: `enum class Literal : TokenType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Literal {
    Long,
    Double,
    String,
    Identifier,
}

impl Literal {
    /// Kotlin: `Literal.isNumberStart`.
    pub fn is_number_start(ch: char) -> bool {
        ch.is_ascii_digit() || ch == '.'
    }

    /// Kotlin: `Literal.isIdentifierStart`.
    pub fn is_identifier_start(ch: char) -> bool {
        ch.is_alphabetic() || ch == '`'
    }

    /// Kotlin: `Literal.isIdentifierPart`.
    pub fn is_identifier_part(ch: char) -> bool {
        ch.is_alphabetic() || ch.is_ascii_digit() || ch == '_'
    }

    /// Kotlin: `Literal.isCharsStart`.
    pub fn is_chars_start(ch: char) -> bool {
        ch == '\'' || ch == '"'
    }
}

/// Declares the `Keyword` enum together with its `name` / `text_of` lookup,
/// from a single `Variant => "CANONICAL"` list. Replaces Kotlin's reflective
/// `values().associateBy(Keyword::name)`.
macro_rules! define_keywords {
    ($($variant:ident => $text:literal),+ $(,)?) => {
        /// SQL keyword tokens. Kotlin: `enum class Keyword : TokenType`.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Keyword {
            $($variant),+
        }

        impl Keyword {
            /// The canonical upper-case spelling. Kotlin: `Keyword.name`.
            pub fn name(&self) -> &'static str {
                match self {
                    $(Keyword::$variant => $text),+
                }
            }

            /// Look up a keyword by text, case-insensitively. Kotlin:
            /// `Keyword.textOf(text)`.
            pub fn text_of(text: &str) -> Option<Keyword> {
                match text.to_uppercase().as_str() {
                    $($text => Some(Keyword::$variant),)+
                    _ => None,
                }
            }
        }
    };
}

define_keywords! {
    // common
    Schema => "SCHEMA", Database => "DATABASE", Table => "TABLE", Column => "COLUMN",
    View => "VIEW", Index => "INDEX", Trigger => "TRIGGER", Procedure => "PROCEDURE",
    Tablespace => "TABLESPACE", Function => "FUNCTION", Sequence => "SEQUENCE", Cursor => "CURSOR",
    From => "FROM", To => "TO", Of => "OF", If => "IF", On => "ON", For => "FOR",
    While => "WHILE", Do => "DO", No => "NO", By => "BY", With => "WITH", Without => "WITHOUT",
    True => "TRUE", False => "FALSE", Temporary => "TEMPORARY", Temp => "TEMP", Comment => "COMMENT",

    // create
    Create => "CREATE", Replace => "REPLACE", Before => "BEFORE", After => "AFTER",
    Instead => "INSTEAD", Each => "EACH", Row => "ROW", Statement => "STATEMENT",
    Execute => "EXECUTE", Bitmap => "BITMAP", Nosort => "NOSORT", Reverse => "REVERSE",
    Compile => "COMPILE",

    // alter
    Alter => "ALTER", Add => "ADD", Modify => "MODIFY", Rename => "RENAME", Enable => "ENABLE",
    Disable => "DISABLE", Validate => "VALIDATE", User => "USER", Identified => "IDENTIFIED",

    // truncate
    Truncate => "TRUNCATE",

    // drop
    Drop => "DROP", Cascade => "CASCADE",

    // insert
    Insert => "INSERT", Into => "INTO", Values => "VALUES",

    // update
    Update => "UPDATE", Set => "SET",

    // delete
    Delete => "DELETE",

    // select
    Select => "SELECT", Distinct => "DISTINCT", Limit => "LIMIT", As => "AS", Case => "CASE",
    When => "WHEN", Else => "ELSE", Then => "THEN", End => "END", Left => "LEFT", Right => "RIGHT",
    Full => "FULL", Inner => "INNER", Outer => "OUTER", Cross => "CROSS", Join => "JOIN",
    Use => "USE", Using => "USING", Natural => "NATURAL", Where => "WHERE", Order => "ORDER",
    Asc => "ASC", Desc => "DESC", Group => "GROUP", Having => "HAVING", Union => "UNION",

    // others
    Declare => "DECLARE", Grant => "GRANT", Fetch => "FETCH", Revoke => "REVOKE", Close => "CLOSE",
    Cast => "CAST", New => "NEW", Escape => "ESCAPE", Lock => "LOCK", Some => "SOME",
    Leave => "LEAVE", Iterate => "ITERATE", Repeat => "REPEAT", Until => "UNTIL", Open => "OPEN",
    Out => "OUT", Inout => "INOUT", Over => "OVER", Advise => "ADVISE", Siblings => "SIBLINGS",
    Loop => "LOOP", Explain => "EXPLAIN", Default => "DEFAULT", Except => "EXCEPT",
    Intersect => "INTERSECT", Minus => "MINUS", Password => "PASSWORD", Local => "LOCAL",
    Global => "GLOBAL", Storage => "STORAGE", Data => "DATA", Coalesce => "COALESCE",

    // types
    Char => "CHAR", Character => "CHARACTER", Varying => "VARYING", Varchar => "VARCHAR",
    Varchar2 => "VARCHAR2", Integer => "INTEGER", Int => "INT", Smallint => "SMALLINT",
    Decimal => "DECIMAL", Dec => "DEC", Numeric => "NUMERIC", Float => "FLOAT", Real => "REAL",
    Double => "DOUBLE", Precision => "PRECISION", Date => "DATE", Time => "TIME",
    Interval => "INTERVAL", Boolean => "BOOLEAN", Blob => "BLOB",

    // conditionals
    And => "AND", Or => "OR", Xor => "XOR", Is => "IS", Not => "NOT", Null => "NULL", In => "IN",
    Between => "BETWEEN", Like => "LIKE", Any => "ANY", All => "ALL", Exists => "EXISTS",

    // functions
    Avg => "AVG", Max => "MAX", Min => "MIN", Sum => "SUM", Count => "COUNT", Greatest => "GREATEST",
    Least => "LEAST", Round => "ROUND", Trunc => "TRUNC", Position => "POSITION",
    Extract => "EXTRACT", Length => "LENGTH", CharLength => "CHAR_LENGTH", Substring => "SUBSTRING",
    Substr => "SUBSTR", Instr => "INSTR", Initcap => "INITCAP", Upper => "UPPER", Lower => "LOWER",
    Trim => "TRIM", Ltrim => "LTRIM", Rtrim => "RTRIM", Both => "BOTH", Leading => "LEADING",
    Trailing => "TRAILING", Translate => "TRANSLATE", Convert => "CONVERT", Lpad => "LPAD",
    Rpad => "RPAD", Decode => "DECODE", Nvl => "NVL",

    // constraints
    Constraint => "CONSTRAINT", Unique => "UNIQUE", Primary => "PRIMARY", Foreign => "FOREIGN",
    Key => "KEY", Check => "CHECK", References => "REFERENCES",
}

/// Declares the `Symbol` enum together with its `text` / `text_of` / `is_symbol`
/// helpers, from a single `Variant => "text"` list. Replaces Kotlin's reflective
/// `values().associateBy(Symbol::text)` and `symbolStartSet`.
macro_rules! define_symbols {
    ($($variant:ident => $text:literal),+ $(,)?) => {
        /// SQL symbol/operator tokens. Kotlin: `enum class Symbol(val text) : TokenType`.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Symbol {
            $($variant),+
        }

        impl Symbol {
            /// Every symbol variant, used for the `is_symbol` character scan.
            pub const ALL: &'static [Symbol] = &[$(Symbol::$variant),+];

            /// The symbol's text. Kotlin: `Symbol.text`.
            pub fn text(&self) -> &'static str {
                match self {
                    $(Symbol::$variant => $text),+
                }
            }

            /// Look up a symbol by its exact text. Kotlin: `Symbol.textOf(text)`.
            pub fn text_of(text: &str) -> Option<Symbol> {
                match text {
                    $($text => Some(Symbol::$variant),)+
                    _ => None,
                }
            }

            /// Whether `ch` can appear in a symbol (Kotlin's `symbolStartSet`
            /// membership). Kotlin: `Symbol.isSymbol`.
            pub fn is_symbol(ch: char) -> bool {
                Self::ALL.iter().any(|s| s.text().contains(ch))
            }

            /// Kotlin: `Symbol.isSymbolStart` (delegates to `isSymbol`).
            pub fn is_symbol_start(ch: char) -> bool {
                Self::is_symbol(ch)
            }
        }
    };
}

define_symbols! {
    LeftParen => "(", RightParen => ")", LeftBrace => "{", RightBrace => "}",
    LeftBracket => "[", RightBracket => "]", Semi => ";", Comma => ",", Dot => ".",
    DoubleDot => "..", Plus => "+", Sub => "-", Star => "*", Slash => "/", Question => "?",
    Eq => "=", Gt => ">", Lt => "<", Bang => "!", Tilde => "~", Caret => "^", Percent => "%",
    Colon => ":", DoubleColon => "::", ColonEq => ":=", LtEq => "<=", GtEq => ">=",
    LtEqGt => "<=>", LtGt => "<>", BangEq => "!=", BangGt => "!>", BangLt => "!<",
    Amp => "&", Bar => "|", DoubleAmp => "&&", DoubleBar => "||", DoubleLt => "<<",
    DoubleGt => ">>", At => "@", Pound => "#",
}

/// A scanned token. Kotlin: `data class Token(text, type, endOffset)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub text: String,
    pub token_type: TokenType,
    pub end_offset: usize,
}

impl Token {
    pub fn new(text: impl Into<String>, token_type: TokenType, end_offset: usize) -> Self {
        Self { text: text.into(), token_type, end_offset }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Kotlin prints e.g. `Token("SELECT", Keyword.SELECT, 6)`.
        let (category, name) = match &self.token_type {
            TokenType::Keyword(k) => ("Keyword", k.name().to_string()),
            TokenType::Symbol(s) => ("Symbol", format!("{s:?}")),
            TokenType::Literal(l) => ("Literal", format!("{l:?}")),
        };
        write!(f, "Token(\"{}\", {category}.{name}, {})", self.text, self.end_offset)
    }
}
