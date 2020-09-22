use structopt::StructOpt;

use OptionField::*; // Enum defined below


const DEFAULT_TAB_WIDTH: u8 = 2u8;

pub type OptionSet = Vec<OptionField>;

pub enum OptionValue<'a> {
  String(InputType<'a>),
  Bool(bool),
  Number(u8)
}

pub enum OptionKind {
  String(OptionField),
  Bool(OptionField),
  NegatedBool(OptionField),
  Number(OptionField),
  Legacy(OptionField),
  Unimplemented(OptionField),
}


#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, )]
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
  Debug(bool),
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

  #[structopt(short = "L", long)]
  /// suppress #line directives in scanner
  pub noline: bool,

  #[structopt(short = "P", long)]
  /// use NAME as prefix of the FlexLexer class name and its members
  pub prefix: Option<String>,

  #[structopt(long)]
  /// initialize input to std::cin instead of stdin
  pub nostdinit: bool,

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

  #[structopt(long)]
  /// do not call global yywrap() on EOF, requires option --flex
  pub noyywrap: bool,

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

  #[structopt(short = "s", long)]
  /// disable the default rule in scanner that echoes unmatched text
  pub nodefault: bool,

  #[structopt(short, long)]
  /// report summary of scanner statistics to stdout
  pub verbose: bool,

  #[structopt(short = "w", long)]
  /// do not generate warnings
  pub nowarn: bool,
  // endregion

  // region Obsolete or Unsettable

  #[structopt(name = "c++", skip)]
  /// n/a
  pub cpp: bool,

  #[structopt(skip)]
  /// n/a
  pub lex_compat: bool,

  // todo: It's not clear what to do about these *interactive synonyms.
  // #[structopt(skip)]
  // default
  // pub never_interactive: bool,

  #[structopt(skip)]
  /// n/a
  pub nounistd: bool,

  #[structopt(skip)]
  /// n/a
  pub posix_compat: bool,

  #[structopt(skip)]
  /// n/a
  pub stack: bool,

  #[structopt(skip)]
  /// default
  pub warn: bool,

  #[structopt(skip)]
  /// Compute the line number while parsing - default
  pub yylineno: bool,

  #[structopt(skip)]
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

        Batch(v)            => { self.batch              = v;             }
        Bison(v)            => { self.bison              = v;             }
        BisonBridge(v)      => { self.bison_bridge       = v;             }
        BisonCc(v)          => { self.bison_cc           = v;             }
        BisonCcNamespace(v) => { self.bison_cc_namespace = Some(v);       }
        BisonCcParser(v)    => { self.bison_cc_parser    = Some(v);       }
        BisonComplete(v)    => { self.bison_complete     = v;             }
        BisonLocations(v)   => { self.bison_locations    = v;             }
        CaseInsensitive(v)  => { self.case_insensitive   = v;             }
        Class(v)            => { self.class              = Some(v);       }
        Cpp(v)              => { self.cpp                = v;             }
        Debug(v)            => { self.debug              = v;             }
        Default(v)          => { self.nodefault          = v;             }
        Dotall(v)           => { self.dotall             = v;             }
        Exception(v)        => { self.exception          = Some(v);       }
        Fast(v)             => { self.fast               = v;             }
        Find(v)             => { self.find               = v;             }
        Flex(v)             => { self.flex               = v;             }
        Freespace(v)        => { self.freespace          = v;             }
        Full(v)             => { self.full               = v;             }
        GraphsFile(v)       => { self.graphs_file        = Some(Some(v)); }
        Include(v)          => { self.include            = Some(v);       }
        Interactive(v)      => { self.interactive        = v;             }
        Lex(v)              => { self.lex                = Some(v);       }
        LexCompat(v)        => { self.lex_compat         = v;             }
        Lexer(v)            => { self.lexer              = Some(v);       }
        Line(v)             => {self.line                = v              }
        Main(v)             => { self.main               = v;             }
        Namespace(v)        => { self.namespace          = Some(v);       }
        OutFile(v)          => { self.out_file           = Some(v);       }
        Pattern(v)          => { self.pattern            = Some(v);       }
        PerfReport(v)       => { self.perf_report        = v;             }
        PosixCompat(v)      => { self.posix_compat       = v;             }
        Prefix(v)           => { self.prefix             = Some(v);       }
        Reentrant(v)        => { self.reentrant          = v;             }
        RegexpFile(v)       => { self.regexp_file        = Some(Some(v)); }
        Reject(v)           => {self.reject              = v              }
        Stack(v)            => { self.stack              = v;             }
        Stdinit(v)          => { self.nostdinit          = v;             }
        Stdout(v)           => { self.stdout             = v;             }
        TablesFile(v)       => { self.tables_file        = Some(Some(v)); }
        Tabs(v)             => { self.tabs               = v;             }
        TokenType(v)        => { self.token_type         = Some(v);       }
        Unicode(v)          => { self.unicode            = v;             }
        Unistd(v)           => { self.nounistd           = v;             }
        Verbose(v)          => { self.verbose            = v;             }
        Warn(v)             => { self.warn               = v;             }
        Yy(v)               => { self.yy                 = v;             }
        Yyclass(v)          => { self.yyclass            = Some(v);       }
        Yylineno(v)         => { self.yylineno           = v;             }
        Yymore(v)           => { self.yymore             = v;             }
        Yywrap(v)           => { self.noyywrap           = v;             }

      } // end match
    } // end for
  }
}

// todo: Is this the right representation? Struct better?
// todo: figure out which will be implemented, which are unsettable, and which are legacy.
static OPTIONS: phf::Map<&'static str, OptionField> = phf_map! {
  "caseless"           => OptionKind::Bool(CaseInsensitive),
  "case-insensitive"   => OptionKind::Bool(CaseInsensitive(true)),
  "caseful"            => OptionKind::Bool(CaseInsensitive(false)),
  "case-sensitive"     => OptionKind::Bool(CaseInsensitive(false)),
  "7bit"               => OptionKind::Legacy(Legacy("7bit")),
  "8bit"               => OptionKind::Legacy(Legacy("8bit")),
  "align"              => OptionKind::Legacy(Legacy("align")),
  "always-interactive" => OptionKind::Bool(Interactive(true)),
  "array"              => OptionKind::Legacy(Legacy("array")),
  "backup"             => OptionKind::Legacy(Legacy("backup")),
  "batch"              => OptionKind::Bool(Batch(true)),
  "bison"              => OptionKind::Bool(Bison(false)),
  "bison_bridge"       => OptionKind::Bool(BisonBridge(false)),
  "bison_cc"           => OptionKind::Bool(BisonCc(false)),
  "bison_cc_namespace" => OptionKind::String(BisonCcNamespace("")),
  "bison_cc_parser"    => OptionKind::String(BisonCcParser("")),
  "bison_complete"     => OptionKind::Bool(BisonComplete(false)),
  "bison_locations"    => OptionKind::Bool(BisonLocations(false)),
  "c++"                => OptionKind::Bool(Cpp(false)),
  "class"              => OptionKind::String(Class("")),
  "ctorarg"            => OptionKind::Legacy(Legacy("ctorarg")),
  "debug"              => OptionKind::Bool(Debug(true)),
  "default"            => OptionKind::Bool(Default(false)),
  "dotall"             => OptionKind::Bool(Dotall(false)),
  "ecs"                => OptionKind::Legacy(Legacy("ecs")),
  "exception"          => OptionKind::String(Exception("")),
  "extra-type"         => OptionKind::Legacy(Legacy("extra-type")),
  "fast"               => OptionKind::Bool(Fast(false)),
  "find"               => OptionKind::Bool(Find(false)),
  "flex"               => OptionKind::Bool(Flex(true)),
  "freespace"          => OptionKind::Bool(Freespace(false)),
  "full"               => OptionKind::Bool(Full(false)),
  "graphs_file"        => OptionKind::String(GraphsFile("")),
  "header_file"        => OptionKind::Legacy(Legacy("header_file")),
  "include"            => OptionKind::String(Include("")),
  "indent"             => OptionKind::Legacy(Legacy("indent")),
  "input"              => OptionKind::Legacy(Legacy("input")),
  "interactive"        => OptionKind::Bool(Interactive(false)),
  "lex"                => OptionKind::String(Lex("")),
  "lex-compat"         => OptionKind::Bool(LexCompat(false)),
  "lexer"              => OptionKind::String(Lexer("")),
  "line"               => OptionKind::Bool(Line(true)),
  "main"               => OptionKind::Bool(Main(true)),
  "matcher"            => OptionKind::Legacy(Legacy("matcher")),
  "meta-ecs"           => OptionKind::Legacy(Legacy("meta-ecs")),
  "namespace"          => OptionKind::String(Namespace("")),
  "never-interactive"  => OptionKind::Bool(Interactive(false)),
  "outfile"            => OptionKind::String(OutFile("")),
  "params"             => OptionKind::Legacy(Legacy("params")),
  "pattern"            => OptionKind::String(Pattern("")),
  "perf-report"        => OptionKind::Bool(PerfReport(true)),
  "permissive"         => OptionKind::Legacy(Legacy("permissive")),
  "pointer"            => OptionKind::Legacy(Legacy("pointer")),
  "posix-compat"       => OptionKind::Legacy(Legacy("posix-compat")),
  "prefix"             => OptionKind::String(Prefix("")),
  "read"               => OptionKind::Legacy(Legacy("read")),
  "reentrant"          => OptionKind::Bool(Reentrant(true)),
  "regexp_file"        => OptionKind::String(RegexpFile("")),
  "reject"             => OptionKind::Bool(Reject(true)),
  "stack"              => OptionKind::Bool(Stack(true)),
  "stdinit"            => OptionKind::Bool(Stdinit(false)),
  "stdout"             => OptionKind::Bool(Stdout(true)),
  "tables-file"        => OptionKind::String(TablesFile("")),
  "tables-verify"      => OptionKind::Legacy(Legacy("tables-verify")),
  "tablesext"          => OptionKind::Legacy(Legacy("tablesext")),
  "tabs"               => OptionKind::Number(Tabs(DEFAULT_TAB_WIDTH)),
  "token_eof"          => OptionKind::Legacy(Legacy("token_eof")),
  "token_type"         => OptionKind::String(TokenType("")),
  "unicode"            => OptionKind::Bool(Unicode(false)),
  "unistd"             => OptionKind::Bool(Unistd(false)),
  "unput"              => OptionKind::Legacy(Legacy("unput")),
  "verbose"            => OptionKind::Bool(Verbose(true)),
  "warn"               => OptionKind::Bool(Warn(true)),
  "yy"                 => OptionKind::Bool(Yy(false)),
  "yy_pop_state"       => OptionKind::Legacy(Legacy("yy_pop_state")),
  "yy_push_state"      => OptionKind::Legacy(Legacy("yy_push_state")),
  "yy_scan_buffer"     => OptionKind::Legacy(Legacy("yy_scan_buffer")),
  "yy_scan_bytes"      => OptionKind::Legacy(Legacy("yy_scan_bytes")),
  "yy_scan_string"     => OptionKind::Legacy(Legacy("yy_scan_string")),
  "yy_top_state"       => OptionKind::Legacy(Legacy("yy_top_state")),
  "yyalloc"            => OptionKind::Legacy(Legacy("yyalloc")),
  "yyclass"            => OptionKind::String(Yyclass("")),
  "yyfree"             => OptionKind::Legacy(Legacy("yyfree")),
  "yyget_column"       => OptionKind::Legacy(Legacy("yyget_column")),
  "yyget_debug"        => OptionKind::Legacy(Legacy("yyget_debug")),
  "yyget_extra"        => OptionKind::Legacy(Legacy("yyget_extra")),
  "yyget_in"           => OptionKind::Legacy(Legacy("yyget_in")),
  "yyget_leng"         => OptionKind::Legacy(Legacy("yyget_leng")),
  "yyget_lineno"       => OptionKind::Legacy(Legacy("yyget_lineno")),
  "yyget_lloc"         => OptionKind::Legacy(Legacy("yyget_lloc")),
  "yyget_lval"         => OptionKind::Legacy(Legacy("yyget_lval")),
  "yyget_out"          => OptionKind::Legacy(Legacy("yyget_out")),
  "yyget_text"         => OptionKind::Legacy(Legacy("yyget_text")),
  "yylineno"           => OptionKind::Bool(Yylineno(true)),
  "yyltype"            => OptionKind::Legacy(Legacy("yyltype")),
  "yymore"             => OptionKind::Bool(Yymore(true)),
  "yyrealloc"          => OptionKind::Legacy(Legacy("yyrealloc")),
  "yyset_column"       => OptionKind::Legacy(Legacy("yyset_column")),
  "yyset_debug"        => OptionKind::Legacy(Legacy("yyset_debug")),
  "yyset_extra"        => OptionKind::Legacy(Legacy("yyset_extra")),
  "yyset_in"           => OptionKind::Legacy(Legacy("yyset_in")),
  "yyset_lineno"       => OptionKind::Legacy(Legacy("yyset_lineno")),
  "yyset_lloc"         => OptionKind::Legacy(Legacy("yyset_lloc")),
  "yyset_lval"         => OptionKind::Legacy(Legacy("yyset_lval")),
  "yyset_out"          => OptionKind::Legacy(Legacy("yyset_out")),
  "yystype"            => OptionKind::Legacy(Legacy("yystype")),
  "yywrap"             => OptionKind::Bool(Yywrap(false)),
};

