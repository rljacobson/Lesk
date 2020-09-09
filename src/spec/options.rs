use std::path::PathBuf;
use structopt::StructOpt;


/*
static OPTIONS: [&str; 86] = [
  "array",
  "always_interactive",
  "batch",
  "bison",
  "bison_bridge",
  "bison_cc",
  "bison_cc_namespace",
  "bison_cc_parser",
  "bison_complete",
  "bison_locations",
  "case_insensitive",
  "class",
  "ctorarg",
  "debug",
  "default",
  "dotall",
  "exception",
  "extra_type",
  "fast",
  "find",
  "flex",
  "freespace",
  "full",
  "graphs_file",
  "header_file",
  "include",
  "indent",
  "input",
  "interactive",
  "lex",
  "lex_compat",
  "lexer",
  "main",
  "matcher",
  "namespace",
  "never_interactive",
  "noarray",
  "nocase_insensitive",
  "nodebug",
  "nodefault",
  "nodotall",
  "nofreespace",
  "noindent",
  "noinput",
  "noline",
  "nomain",
  "nopointer",
  "nostack",
  "nostdinit",
  "nounicode",
  "nounistd",
  "nounput",
  "nowarn",
  "noyylineno",
  "noyymore",
  "noyywrap",
  "outfile",
  "params",
  "pattern",
  "permissive",
  "pointer",
  "perf_report",
  "posix_compat",
  "prefix",
  "reentrant",
  "regexp_file",
  "stack",
  "stdinit",
  "stdout",
  "tables_file",
  "tabs",
  "token_eof",
  "token_type",
  "unicode",
  "unput",
  "verbose",
  "warn",
  "yy",
  "yyclass",
  "yylineno",
  "yymore",
  "yywrap",
  "YYLTYPE",
  "YYSTYPE",
  "7bit",
  "8bit",
];
*/


//#[derive(Debug, StructOpt)]

#[derive(Debug, StructOpt)]
#[structopt(name = "Lesk", about = "A lexer generator.")]
pub struct Options {
  // Scanner

  #[structopt(short="+", long)]
  /// Generate Flex-compatible C++ scanner
  flex: bool,

  #[structopt(short="a", long)]
  /// Dot in patterns match newline
  dotall: bool,

  /// Generate scanner for batch input by buffering the entire input
  #[structopt(short="B", long)]
  batch: bool,

  #[structopt(short, long)]
  /// Generate full scanner with FSM opcode tables
  full: bool,

  #[structopt(short="F", long)]
  /// Generate fast scanner with FSM code
  fast: bool,

  #[structopt(short="i", long)]
  /// Ignore case in patterns
  case_insensitive: bool,

  // todo: option alias
  #[structopt(short="I", long)]
  /// Generate interactive scanner
  interactive: bool, // --always-interactive

  // todo: selectable regex engine
  //#[structopt(short, long)]
  //#[parse(type [= path::to::parser::fn])]
  // /// Which regex backend to use
  //matcher: RegexEngine,

  #[structopt(long)]
  /// use custom pattern class NAME for custom matcher option -m
  pattern: Option<String>,

  #[structopt(long, parse(from_os_str))]
  /// include header FILE.h for custom matcher option -m
  include: Option<PathBuf>,

  #[structopt(short="S", long)]
  /// generate search engine to find matches, ignores unmatched input
  find: bool,

  #[structopt(short="T", long, default_value="4")]
  /// set default tab size to N (1,2,4,8) for indent/dedent matching
  tabs: u8,

  #[structopt(short="u", long)]
  /// match Unicode . (dot), \\p, \\s, \\w, etc and group UTF-8 bytes
  unicode: bool,

  #[structopt(short="x", long)]
  /// ignore space in patterns
  freespace: bool,



  // Generated files

  #[structopt(short, long, parse(from_os_str), default_value="lex.yy.cpp")]
  /// specify output FILE instead of lex.yy.cpp
  pub(crate) outfile: PathBuf,

  #[structopt(short="t", long)]
  /// write scanner on stdout instead of lex.yy.cpp
  stdout: bool,

  #[structopt(long)]
  /// write the scanner's DFA in Graphviz format to FILE.gv
  graphs_file: Option<Option<PathBuf>>,

  //#[structopt(short, long, parse(from_os_str))]
  ///// write a C++ header FILE.h in addition to the scanner
  //header_file: Option<Option<PathBuf>>,

  #[structopt(long)]
  /// write the scanner's regular expression patterns to FILE.txt
  regexp_file: Option<Option<PathBuf>>,

  #[structopt(long)]
  /// write the scanner's FSM opcode tables or FSM code to FILE.cpp
  tables_file: Option<Option<PathBuf>>,




  // Generated code

  #[structopt(long)]
  /// use C++ namespace NAME for the generated scanner class, with multiple
  /// namespaces specified as NAME1.NAME2.NAME3 ...
  namespace: Option<String>,

  #[structopt(long)]
  /// use lexer class NAME instead of Lexer or yyFlexLexer
  lexer: Option<String>,

  #[structopt(long)]
  /// use lex function NAME instead of lex or yylex
  lex: Option<String>,

  #[structopt(long)]
  /// declare a user-defined scanner class NAME
  class: Option<String>,

  #[structopt(long)]
  /// generate Flex-compatible scanner with user-defined class NAME
  yyclass: Option<String>,

  #[structopt(long)]
  /// generate main() to invoke lex() or yylex() once
  main: bool,

  #[structopt(short="L", long)]
  /// suppress #line directives in scanner
  noline: bool,

  #[structopt(short="P", long)]
  /// use NAME as prefix of the FlexLexer class name and its members
  prefix: Option<String>,

  #[structopt(long)]
  /// initialize input to std::cin instead of stdin
  nostdinit: bool,

  #[structopt(long)]
  /// generate global yylex() scanner, yytext, yyleng, yylineno
  bison: bool,

  #[structopt(long)]
  /// generate reentrant yylex() scanner for bison pure parser
  bison_bridge: bool,

  #[structopt(long)]
  /// generate bison C++ interface code for bison lalr1.cc skeleton
  bison_cc: bool,

  #[structopt(long)]
  /// use namespace NAME with bison lalr1.cc skeleton
  bison_cc_namespace: Option<String>,

  #[structopt(long)]
  /// use parser class NAME with bison lalr1.cc skeleton
  bison_cc_parser: Option<String>,

  #[structopt(long)]
  /// use bison complete-symbols feature, implies bison-cc
  bison_complete: bool,

  #[structopt(long)]
  /// include bison yylloc support
  bison_locations: bool,

  #[structopt(short="R", long)]
  /// generate Flex-compatible yylex() reentrant scanner functions
  reentrant: bool,

  #[structopt(short, long)]
  /// same as --flex and --bison, also generate global yyin, yyout
  yy: bool,

  #[structopt(long)]
  /// do not call global yywrap() on EOF, requires option --flex
  noyywrap: bool,

  #[structopt(long)]
  /// use exception VALUE to throw in the default rule of the scanner
  exception: Option<String>,

  #[structopt(long)]
  /// use NAME as the return type of lex() and yylex() instead of int
  token_type: Option<String>,



  // Debugging

  #[structopt(short, long)]
  /// enable debug mode in scanner
  debug: bool,

  #[structopt(short, long)]
  /// scanner reports detailed performance statistics to stderr
  perf_report: bool,

  #[structopt(short="s", long)]
  /// disable the default rule in scanner that echoes unmatched text
  nodefault: bool,

  #[structopt(short, long)]
  /// report summary of scanner statistics to stdout
  verbose: bool,

  #[structopt(short="w", long)]
  /// do not generate warnings
  nowarn: bool,



  // Obsolete or unsettable

  #[structopt(name="c++", skip)]
  /// n/a
  cpp: bool,

  #[structopt(skip)]
  /// n/a
  lex_compat: bool,

  #[structopt(skip)]
  /// default
  never_interactive: bool,

  #[structopt(skip)]
  /// n/a
  nounistd: bool,

  #[structopt(skip)]
  /// n/a
  posix_compat: bool,

  #[structopt(skip)]
  /// n/a
  stack: bool,

  #[structopt(skip)]
  /// default
  warn: bool,

  #[structopt(skip)]
  /// default
  yylineno: bool,

  #[structopt(skip)]
  /// default
  yymore: bool,

}
