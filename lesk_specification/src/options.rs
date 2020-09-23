#[macro_use]
use phf::{map, Map};
use structopt::StructOpt;

use crate::parser::InputType;
use OptionField::*; // Enum defined below


const DEFAULT_TAB_WIDTH: u8 = 2u8;

pub type OptionSet = Vec<OptionField>;

pub enum OptionValue<'a> {
  String(InputType<'a>),
  Bool(bool),
  Number(u8)
}

pub enum OptionKind {
  String(fn(String) -> OptionField),
  Bool(fn(bool) -> OptionField),
  NegatedBool(fn(bool) -> OptionField),
  Number(fn(u8) -> OptionField),
  Legacy,
  Unimplemented,
}


#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum OptionField {
  // Scanner
  InFile(String),

  // Cannot be updated
  Batch(bool),
  CaseInsensitive(bool),
  Dotall(bool),
  Fast(bool),
  Find(bool),
  Flex(bool),
  Freespace(bool),
  Full(bool),
  GraphsFile(String),
  Include(String),
  Interactive(bool),
  OutFile(String),
  Pattern(String),
  RegexpFile(String),
  Stdout(bool),
  TablesFile(String),
  Tabs(u8),
  Unicode(bool),

  // Generator
  Bison(bool),
  BisonBridge(bool),
  BisonCc(bool),
  BisonCcNamespace(String),
  BisonCcParser(String),
  BisonComplete(bool),
  BisonLocations(bool),
  Class(String),
  Exception(String),
  Lex(String),
  Lexer(String),
  Line(bool),
  Main(bool),
  Namespace(String),
  Prefix(String),
  Reentrant(bool),
  Stdinit(bool),
  TokenType(String),
  Yy(bool),
  Yyclass(String),
  Yywrap(bool),

  // Debugging and Benchmarking
  Debug_(bool),
  Default(bool),
  Warn(bool),
  PerfReport(bool),
  Verbose(bool),
  Reject(bool),

  // Obsolete / Unsettable
  // Included for completeness
  Cpp(bool),
  LexCompat(bool),
  Unistd(bool),
  PosixCompat(bool),
  Stack(bool),
  Yylineno(bool),
  Yymore(bool),

  Legacy(&'static str),
}


// todo: switch to argh (https://docs.rs/argh/0.1.3/argh/)
#[derive(Debug, StructOpt)]
#[structopt(name = "Lesk", about = "A lexer generator.")]
pub struct Options {
  // region Scanner

  /// Generate scanner for batch input by buffering the entire input
  #[structopt(short = "B", long)]
  pub batch: bool,

  #[structopt(short = "i", long)]
  /// Ignore case in patterns
  pub case_insensitive: bool,

  #[structopt(short = "a", long)]
  /// Dot in patterns match newline
  pub dotall: bool,

  #[structopt(short = "F", long)]
  /// Generate fast scanner with FSM code
  pub fast: bool,

  #[structopt(short = "S", long)]
  /// generate search engine to find matches, ignores unmatched input
  pub find: bool,

  #[structopt(short = "+", long, required_if("noyywrap", "true"))]
  /// Generate Flex-compatible C++ scanner
  pub flex: bool,

  #[structopt(short = "x", long)]
  /// ignore space in patterns
  pub freespace: bool,

  #[structopt(short, long)]
  /// Generate full scanner with FSM opcode tables
  pub full: bool,

  #[structopt(name = "FILE", default_value = "STDIN")]
  /// The scanner specification file
  pub in_file: String,

  #[structopt(long)]
  /// include header FILE.h for custom matcher option -m
  pub include: Option<String>,

  // todo: option alias
  #[structopt(short = "I", long)]
  /**
  Generate interactive scanner. Related options:
    always-interactive
    never_interactive
  */
  pub interactive: bool, //

  // todo: selectable regex engine
  //#[structopt(short, long)]
  //#[parse(type [= path::to::parser::fn])]
  // /// Which regex backend to use
  //matcher: RegexEngine,

  #[structopt(long)]
  /// use custom pattern class NAME for custom matcher option -m
  pub pattern: Option<String>,

  #[structopt(short = "T", long, default_value = "2")]
  /// set default tab size to N (2,4,8) for indent/dedent matching
  pub tabs: u8,

  #[structopt(short = "u", long)]
  /// match Unicode . (dot), \\p, \\s, \\w, etc and group UTF-8 bytes
  pub unicode: bool,

  // endregion

  //region Generated Files

  #[structopt(long)]
  /// write the scanner's DFA in Graphviz format to FILE.gv
  pub graphs_file: Option<Option<String>>,

  //#[structopt(short, long, parse(from_os_str))]
  ///// write a C++ header FILE.h in addition to the scanner
  //header_file: Option<Option<String>>,

  #[structopt(short, long)]
  /// specify output FILE instead of lex.yy.cpp
  pub out_file: Option<String>,

  #[structopt(long)]
  /// write the scanner's regular expression patterns to FILE.txt
  pub regexp_file: Option<Option<String>>,

  #[structopt(long)]
  /// override Lesk's decision as to whether you use the options, either by setting them (e.g.,
  /// %option reject) to indicate the feature is indeed used
  pub reject: bool,

  #[structopt(short = "t", long)]
  /// write scanner on stdout instead of lex.yy.cpp
  pub stdout: bool,

  #[structopt(long)]
  /// write the scanner's FSM opcode tables or FSM code to FILE.cpp
  pub tables_file: Option<Option<String>>,
  // endregion

  // region Generated Code

  #[structopt(long)]
  /// use C++ namespace NAME for the generated scanner class, with multiple
  /// namespaces specified as NAME1.NAME2.NAME3 ...
  pub namespace: Option<String>,

  #[structopt(long)]
  /// use lexer class NAME instead of Lexer or yyFlexLexer
  pub lexer: Option<String>,

  #[structopt(long)]
  /// use lex function NAME instead of lex or yylex
  pub lex: Option<String>,

  #[structopt(long)]
  /// declare a user-defined scanner class NAME
  pub class: Option<String>,

  #[structopt(long)]
  /// generate Flex-compatible scanner with user-defined class NAME
  pub yyclass: Option<String>,

  #[structopt(long)]
  /// generate main() to invoke lex() or yylex() once
  pub main: bool,

  #[structopt(short = "L", long="noline")]
  /// suppress #line directives in scanner
  pub line: bool,

  #[structopt(short = "P", long)]
  /// use NAME as prefix of the FlexLexer class name and its members
  pub prefix: Option<String>,

  #[structopt(long="nostdinit")]
  /// initialize input to std::cin instead of stdin
  pub stdinit: bool,

  #[structopt(long)]
  /// generate global yylex() scanner, yytext, yyleng, yylineno
  pub bison: bool,

  #[structopt(long)]
  /// generate reentrant yylex() scanner for bison pure parser
  pub bison_bridge: bool,

  #[structopt(long)]
  /// generate bison C++ interface code for bison lalr1.cc skeleton
  pub bison_cc: bool,

  #[structopt(long)]
  /// use namespace NAME with bison lalr1.cc skeleton
  pub bison_cc_namespace: Option<String>,

  #[structopt(long)]
  /// use parser class NAME with bison lalr1.cc skeleton
  pub bison_cc_parser: Option<String>,

  #[structopt(long)]
  /// use bison complete-symbols feature, implies bison-cc
  pub bison_complete: bool,

  #[structopt(long)]
  /// include bison yylloc support
  pub bison_locations: bool,

  #[structopt(short = "R", long)]
  /// generate Flex-compatible yylex() reentrant scanner functions
  pub reentrant: bool,

  #[structopt(short, long)]
  /// same as --flex and --bison, also generate global yyin, yyout
  pub yy: bool,

  #[structopt(long="noyywrap")]
  /// do not call global yywrap() on EOF, requires option --flex
  pub yywrap: bool,

  #[structopt(long)]
  /// use exception VALUE to throw in the default rule of the scanner
  pub exception: Option<String>,

  #[structopt(long)]
  /// use NAME as the return type of lex() and yylex() instead of int
  pub token_type: Option<String>,
  // endregion

  // region Debugging

  #[structopt(short, long)]
  /// enable debug mode in scanner
  pub debug: bool,

  #[structopt(short, long)]
  /// scanner reports detailed performance statistics to stderr
  pub perf_report: bool,

  #[structopt(short = "s", long="nodefault")]
  /// disable the default rule in scanner that echoes unmatched text
  pub default: bool,

  #[structopt(short, long)]
  /// report summary of scanner statistics to stdout
  pub verbose: bool,

  #[structopt(short = "w", long="nowarn")]
  /// do not generate warnings
  pub warn: bool,
  // endregion

  // region Obsolete or Unsettable

  #[structopt(long)]
  /// n/a
  pub cpp: bool,

  #[structopt(long)]
  /// n/a
  pub lex_compat: bool,

  // todo: It's not clear what to do about these *interactive synonyms.
  // #[structopt(skip)]
  // default
  // pub never_interactive: bool,

  #[structopt(long="nounistd")]
  /// n/a
  pub unistd: bool,

  #[structopt(long)]
  /// n/a
  pub posix_compat: bool,

  #[structopt(long)]
  /// n/a
  pub stack: bool,

  #[structopt(long)]
  /// Compute the line number while parsing - default
  pub yylineno: bool,

  #[structopt(long)]
  /// default
  pub yymore: bool,

  // endregion
}


impl Options {
  /// Update the values of self with those of other. The `OptionSet` `other` is consumed.
  pub fn update(&mut self, other: OptionSet) {
    for field in other {
      match field {
        // Scanner
        InFile(_) => { /* in_file cannot change. */ }
        Legacy(_) => { /* pass */ }

        Batch(v) => { self.batch = v; }
        Bison(v) => { self.bison = v; }
        BisonBridge(v) => { self.bison_bridge = v; }
        BisonCc(v) => { self.bison_cc = v; }
        BisonCcNamespace(v) => { self.bison_cc_namespace = Some(v); }
        BisonCcParser(v) => { self.bison_cc_parser = Some(v); }
        BisonComplete(v) => { self.bison_complete = v; }
        BisonLocations(v) => { self.bison_locations = v; }
        CaseInsensitive(v) => { self.case_insensitive = v; }
        Class(v) => { self.class = Some(v); }
        Cpp(v) => { self.cpp = v; }
        Debug_(v) => { self.debug = v; }
        Default(v) => { self.default = v; }
        Dotall(v) => { self.dotall = v; }
        Exception(v) => { self.exception = Some(v); }
        Fast(v) => { self.fast = v; }
        Find(v) => { self.find = v; }
        Flex(v) => { self.flex = v; }
        Freespace(v) => { self.freespace = v; }
        Full(v) => { self.full = v; }
        GraphsFile(v) => { self.graphs_file = Some(Some(v)); }
        Include(v) => { self.include = Some(v); }
        Interactive(v) => { self.interactive = v; }
        Lex(v) => { self.lex = Some(v); }
        LexCompat(v) => { self.lex_compat = v; }
        Lexer(v) => { self.lexer = Some(v); }
        Line(v) => { self.line = v }
        Main(v) => { self.main = v; }
        Namespace(v) => { self.namespace = Some(v); }
        OutFile(v) => { self.out_file = Some(v); }
        Pattern(v) => { self.pattern = Some(v); }
        PerfReport(v) => { self.perf_report = v; }
        PosixCompat(v) => { self.posix_compat = v; }
        Prefix(v) => { self.prefix = Some(v); }
        Reentrant(v) => { self.reentrant = v; }
        RegexpFile(v) => { self.regexp_file = Some(Some(v)); }
        Reject(v) => { self.reject = v }
        Stack(v) => { self.stack = v; }
        Stdinit(v) => { self.stdinit = v; }
        Stdout(v) => { self.stdout = v; }
        TablesFile(v) => { self.tables_file = Some(Some(v)); }
        Tabs(v) => { self.tabs = v; }
        TokenType(v) => { self.token_type = Some(v); }
        Unicode(v) => { self.unicode = v; }
        Unistd(v) => { self.unistd = v; }
        Verbose(v) => { self.verbose = v; }
        Warn(v) => { self.warn = v; }
        Yy(v) => { self.yy = v; }
        Yyclass(v) => { self.yyclass = Some(v); }
        Yylineno(v) => { self.yylineno = v; }
        Yymore(v) => { self.yymore = v; }
        Yywrap(v) => { self.yywrap = v; }
      } // end match
    } // end for
  }
}

// todo: Is this the right representation? Struct better?
// todo: figure out which will be implemented, which are unsettable, and which are legacy.
pub static OPTIONS: phf::Map<&'static str, OptionKind> = phf_map! {

  "caseless"           => OptionKind::Bool(CaseInsensitive),
  "case-insensitive"   => OptionKind::Bool(CaseInsensitive),
  "caseful"            => OptionKind::NegatedBool(CaseInsensitive),
  "case-sensitive"     => OptionKind::NegatedBool(CaseInsensitive),
  "7bit"               => OptionKind::Legacy,
  "8bit"               => OptionKind::Legacy,
  "align"              => OptionKind::Legacy,
  "always-interactive" => OptionKind::Bool(Interactive),
  "array"              => OptionKind::Legacy,
  "backup"             => OptionKind::Legacy,
  "batch"              => OptionKind::Bool(Batch),
  "bison"              => OptionKind::Bool(Bison),
  "bison_bridge"       => OptionKind::Bool(BisonBridge),
  "bison_cc"           => OptionKind::Bool(BisonCc),
  "bison_cc_namespace" => OptionKind::String(BisonCcNamespace),
  "bison_cc_parser"    => OptionKind::String(BisonCcParser),
  "bison_complete"     => OptionKind::Bool(BisonComplete),
  "bison_locations"    => OptionKind::Bool(BisonLocations),
  "c++"                => OptionKind::Bool(Cpp),
  "class"              => OptionKind::String(Class),
  "ctorarg"            => OptionKind::Legacy,
  "debug"              => OptionKind::Bool(Debug_),
  "default"            => OptionKind::Bool(Default),
  "dotall"             => OptionKind::Bool(Dotall),
  "ecs"                => OptionKind::Legacy,
  "exception"          => OptionKind::String(Exception),
  "extra-type"         => OptionKind::Legacy,
  "fast"               => OptionKind::Bool(Fast),
  "find"               => OptionKind::Bool(Find),
  "flex"               => OptionKind::Bool(Flex),
  "freespace"          => OptionKind::Bool(Freespace),
  "full"               => OptionKind::Bool(Full),
  "graphs_file"        => OptionKind::String(GraphsFile),
  "header_file"        => OptionKind::Legacy,
  "include"            => OptionKind::String(Include),
  "indent"             => OptionKind::Legacy,
  "input"              => OptionKind::Legacy,
  "interactive"        => OptionKind::Bool(Interactive),
  "lex"                => OptionKind::String(Lex),
  "lex-compat"         => OptionKind::Bool(LexCompat),
  "lexer"              => OptionKind::String(Lexer),
  "line"               => OptionKind::Bool(Line),
  "main"               => OptionKind::Bool(Main),
  "matcher"            => OptionKind::Legacy,
  "meta-ecs"           => OptionKind::Legacy,
  "namespace"          => OptionKind::String(Namespace),
  "never-interactive"  => OptionKind::NegatedBool(Interactive),
  "outfile"            => OptionKind::String(OutFile),
  "params"             => OptionKind::Legacy,
  "pattern"            => OptionKind::String(Pattern),
  "perf-report"        => OptionKind::Bool(PerfReport),
  "permissive"         => OptionKind::Legacy,
  "pointer"            => OptionKind::Legacy,
  "posix-compat"       => OptionKind::Legacy,
  "prefix"             => OptionKind::String(Prefix),
  "read"               => OptionKind::Legacy,
  "reentrant"          => OptionKind::Bool(Reentrant),
  "regexp_file"        => OptionKind::String(RegexpFile),
  "reject"             => OptionKind::Bool(Reject),
  "stack"              => OptionKind::Bool(Stack),
  "stdinit"            => OptionKind::Bool(Stdinit),
  "stdout"             => OptionKind::Bool(Stdout),
  "tables-file"        => OptionKind::String(TablesFile),
  "tables-verify"      => OptionKind::Legacy,
  "tablesext"          => OptionKind::Legacy,
  "tabs"               => OptionKind::Number(Tabs),
  "token_eof"          => OptionKind::Legacy,
  "token_type"         => OptionKind::String(TokenType),
  "unicode"            => OptionKind::Bool(Unicode),
  "unistd"             => OptionKind::Bool(Unistd),
  "unput"              => OptionKind::Legacy,
  "verbose"            => OptionKind::Bool(Verbose),
  "warn"               => OptionKind::Bool(Warn),
  "yy"                 => OptionKind::Bool(Yy),
  "yy_pop_state"       => OptionKind::Legacy,
  "yy_push_state"      => OptionKind::Legacy,
  "yy_scan_buffer"     => OptionKind::Legacy,
  "yy_scan_bytes"      => OptionKind::Legacy,
  "yy_scan_string"     => OptionKind::Legacy,
  "yy_top_state"       => OptionKind::Legacy,
  "yyalloc"            => OptionKind::Legacy,
  "yyclass"            => OptionKind::String(Yyclass),
  "yyfree"             => OptionKind::Legacy,
  "yyget_column"       => OptionKind::Legacy,
  "yyget_debug"        => OptionKind::Legacy,
  "yyget_extra"        => OptionKind::Legacy,
  "yyget_in"           => OptionKind::Legacy,
  "yyget_leng"         => OptionKind::Legacy,
  "yyget_lineno"       => OptionKind::Legacy,
  "yyget_lloc"         => OptionKind::Legacy,
  "yyget_lval"         => OptionKind::Legacy,
  "yyget_out"          => OptionKind::Legacy,
  "yyget_text"         => OptionKind::Legacy,
  "yylineno"           => OptionKind::Bool(Yylineno),
  "yyltype"            => OptionKind::Legacy,
  "yymore"             => OptionKind::Bool(Yymore),
  "yyrealloc"          => OptionKind::Legacy,
  "yyset_column"       => OptionKind::Legacy,
  "yyset_debug"        => OptionKind::Legacy,
  "yyset_extra"        => OptionKind::Legacy,
  "yyset_in"           => OptionKind::Legacy,
  "yyset_lineno"       => OptionKind::Legacy,
  "yyset_lloc"         => OptionKind::Legacy,
  "yyset_lval"         => OptionKind::Legacy,
  "yyset_out"          => OptionKind::Legacy,
  "yystype"            => OptionKind::Legacy,
  "yywrap"             => OptionKind::Bool(Yywrap),
};

