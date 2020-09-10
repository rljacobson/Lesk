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

  #[structopt(name="FILE", default_value="STDIN")]
  /// The scanner specification file
  pub in_file: String,

  #[structopt(short="+", long)]
  /// Generate Flex-compatible C++ scanner
  pub flex: bool,

  #[structopt(short="a", long)]
  /// Dot in patterns match newline
  pub dotall: bool,

  /// Generate scanner for batch input by buffering the entire input
  #[structopt(short="B", long)]
  pub batch: bool,

  #[structopt(short, long)]
  /// Generate full scanner with FSM opcode tables
  pub full: bool,

  #[structopt(short="F", long)]
  /// Generate fast scanner with FSM code
  pub fast: bool,

  #[structopt(short="i", long)]
  /// Ignore case in patterns
  pub case_insensitive: bool,

  // todo: option alias
  #[structopt(short="I", long)]
  /// Generate interactive scanner
  pub interactive: bool, // --always-interactive

  // todo: selectable regex engine
  //#[structopt(short, long)]
  //#[parse(type [= path::to::parser::fn])]
  // /// Which regex backend to use
  //matcher: RegexEngine,

  #[structopt(long)]
  /// use custom pattern class NAME for custom matcher option -m
  pub pattern: Option<String>,

  #[structopt(long)]
  /// include header FILE.h for custom matcher option -m
  pub include: Option<String>,

  #[structopt(short="S", long)]
  /// generate search engine to find matches, ignores unmatched input
  pub find: bool,

  #[structopt(short="T", long, default_value="4")]
  /// set default tab size to N (1,2,4,8) for indent/dedent matching
  pub tabs: u8,

  #[structopt(short="u", long)]
  /// match Unicode . (dot), \\p, \\s, \\w, etc and group UTF-8 bytes
  pub unicode: bool,

  #[structopt(short="x", long)]
  /// ignore space in patterns
  pub freespace: bool,



  // Generated files

  #[structopt(short, long)]
  /// specify output FILE instead of lex.yy.cpp
  pub out_file: Option<String>,

  #[structopt(short="t", long)]
  /// write scanner on stdout instead of lex.yy.cpp
  pub stdout: bool,

  #[structopt(long)]
  /// write the scanner's DFA in Graphviz format to FILE.gv
  pub graphs_file: Option<Option<String>>,

  //#[structopt(short, long, parse(from_os_str))]
  ///// write a C++ header FILE.h in addition to the scanner
  //header_file: Option<Option<String>>,

  #[structopt(long)]
  /// write the scanner's regular expression patterns to FILE.txt
  pub regexp_file: Option<Option<String>>,

  #[structopt(long)]
  /// write the scanner's FSM opcode tables or FSM code to FILE.cpp
  pub tables_file: Option<Option<String>>,




  // Generated code

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

  #[structopt(short="L", long)]
  /// suppress #line directives in scanner
  pub noline: bool,

  #[structopt(short="P", long)]
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

  #[structopt(short="R", long)]
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



  // Debugging

  #[structopt(short, long)]
  /// enable debug mode in scanner
  pub debug: bool,

  #[structopt(short, long)]
  /// scanner reports detailed performance statistics to stderr
  pub perf_report: bool,

  #[structopt(short="s", long)]
  /// disable the default rule in scanner that echoes unmatched text
  pub nodefault: bool,

  #[structopt(short, long)]
  /// report summary of scanner statistics to stdout
  pub verbose: bool,

  #[structopt(short="w", long)]
  /// do not generate warnings
  pub nowarn: bool,



  // Obsolete or unsettable

  #[structopt(name="c++", skip)]
  /// n/a
  pub cpp: bool,

  #[structopt(skip)]
  /// n/a
  pub lex_compat: bool,

  #[structopt(skip)]
  /// default
  pub never_interactive: bool,

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
  /// default
  pub yylineno: bool,

  #[structopt(skip)]
  /// default
  pub yymore: bool,

}
