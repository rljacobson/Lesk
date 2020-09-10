#![allow(dead_code)]

use std::io::{Write, Read, BufWriter};
use std::fs::File;

use structopt::StructOpt;
use codespan_reporting::{diagnostic::Diagnostic, files::SimpleFiles};

use super::*;

static DEFAULT_OUTPUT_PATH: &str = "lex.yy.cpp";


//#[derive(Default)]
pub struct Specification<'s> {
  pub options : Options, //< maps option name (from the options_table) to its option value
  color_term  : bool,    //< terminal supports colors

  writer       : Box<dyn FnMut(&str)>, //< output stream
  code_files   : SimpleFiles<String, String>,       //< Source code database
  source       : String,               //< source text
  in_file      : String,

  conditions  : StrVec<'s>, //< "INITIAL" start condition etc. defined with %x name
  definitions : StrMap<'s>, //< map of {name} to regex
  inclusive   : Starts,     //< inclusive start conditions

  //library      : Library,      //< the regex library selected
  line           : &'s str,      //< current line read from input
  lineno         : usize,        //< current line number at input
  patterns       : StrVec<'s>,   //< regex patterns for each start condition
  rules          : RulesMap<'s>, //< <Start_i>regex_j action for Start i Rule j
  section_1      : Codes,        //< %{ user code %} in section 1 container
  section_2      : CodesMap,     //< lexer user code in section 2 container
  section_3      : Codes,        //< main user code in section 3 container
  section_init   : Codes,        //< %init{ init code %} in section 1 container
  section_struct : Codes,        //< %class{ class code %} in section 1 container
  section_top    : Codes,        //< %top{ user code %} in section 1 container

}

impl<'s> Default for Specification<'s> {
  fn default() -> Self {
    // This method:
    //    1. Parses the command line arguments, and
    //    2. Establishes the output stream.
    //    3. Read the source file from the input stream into a codespan structure.


    let mut new_spec = Self {
      options        : Options::from_args(), // Parses command line arguments
      color_term     : true,
      // todo        : writer to be replaced with Akama
      writer         : Box::new(|_|{}),       // a dummy initial value
      code_files     : SimpleFiles::new(),
      source         : String::default(),
      in_file        : String::default(),
      conditions     : StrVec::default(),
      definitions    : StrMap::default(),
      inclusive      : Starts::default(),
      //library      : Library::default(),
      line           : &"",
      lineno         : 0,
      patterns       : StrVec::default(),
      rules          : RulesMap::default(),
      section_1      : Codes::default(),
      section_2      : CodesMap::default(),
      section_3      : Codes::default(),
      section_init   : Codes::default(),
      section_struct : Codes::default(),
      section_top    : Codes::default(),

    };



    // Establish the output stream

    new_spec.writer = // the value of the if statement
    if let Some(path) = &new_spec.options.out_file{
      let f = File::create(&path)
                .expect(format!("Unable to create file: {}", &path).as_str());
      let mut buf_writer = BufWriter::new(f);

      // Write to both file and stdout.
      if new_spec.options.stdout {
        let mut std_out = BufWriter::new(std::io::stdout());

        Box::new(
          move |buf: &str| {
            let _ = std_out.write_all(buf.as_bytes());
            let _ = buf_writer.write_all(buf.as_bytes());
          }
        )
      }
      // Only write to file
      else {
        Box::new(
          move |buf: &str| {
            let _ = buf_writer.write_all(buf.as_bytes());
          }
        )
      }
    }
    // No filename supplied
    else {
      // Only write to STDOUT
      if new_spec.options.stdout {
        let mut std_out = BufWriter::new(std::io::stdout());

        Box::new(
          move |buf: &str| {
            let _ = std_out.write(buf.as_bytes());
          }
        )
      }
      // Only write to default output file `lex.yy.rs`
      else {
        let f = File::create(DEFAULT_OUTPUT_PATH)
        .expect(format!("Unable to create file: {}", DEFAULT_OUTPUT_PATH).as_str());
        let mut buf_writer = BufWriter::new(f);

        Box::new(
          move |buf: &str| {
            let _ = buf_writer.write_all(buf.as_bytes());
          }
        )
      }
    };


    // Read the source file

    // Read from STDIN
    if &new_spec.options.in_file == "STDIN" {
      // Both `new_source` and `new_file` will be consumed.
      let mut new_source = String::default();
      let mut in_file    = String::default();

      std::mem::swap(&mut new_spec.options.in_file, &mut in_file);
      let _ = std::io::stdin().read_to_string(&mut new_source);
      new_spec.code_files.add(in_file, new_source);
    }
    // Read from a file
    else {
      // Both `new_source` and `new_file` will be consumed.
      let mut new_source = String::default();
      let mut in_file    = String::default();

      std::mem::swap(&mut new_spec.options.in_file, &mut in_file);
      std::fs::File::open(&in_file)
        .expect(format!(
          "Could not read from file: {}",
          &in_file
        ).as_str())
        .read_to_string(&mut new_source)
        .unwrap_or_else(
          |x| { panic!("Could not read from file: {:?}", x.into_inner()); }
        );

      new_spec.code_files.add(in_file, new_source);
    }

    new_spec

  }
}

impl<'s> Specification<'s> {
  pub fn new() -> Self {
    Self::default()
  }


  pub fn parse(&mut self) {
    if self.source.is_empty(){
      eprintln!("Empty source file.");
      return;
    }

    self.conditions.push("INITIAL");
    self.inclusive.insert(0);
    self.lineno = 0;

    // If there were a choice of libraries...
    //set_library();
    self.parse_section_1();
  }


  pub fn parse_section_1(&mut self) {

  }

  /*
  void        parse_section_2();
  void        parse_section_3();
  void        include(const std::string& filename);
  void        write();
  // These will be replaced with a template library
  void        write_banner(const char *title);
  void        write_prelude();
  void        write_defines();
  void        write_class();
  void        write_section_top();
  void        write_section_class();
  void        write_section_init();
  void        write_perf_report();
  void        write_section_1();
  void        write_section_3();
  void        write_code(const Codes& codes);
  void        write_code(const Code& code);
  void        write_lexer();
  void        write_main();
  void        write_regex(const std::string *condition, const std::string& regex);
  void        write_namespace_open();
  void        write_namespace_close();
  void        write_namespace_scope();

  void        undot_namespace(std::string& s);
  void        stats();
  bool        get_line();
  bool        skip_comment(size_t& pos);
  bool        is(const char *s);
  bool        br(size_t pos, const char *s = NULL);
  bool        as(size_t& pos, const char *s);
  bool        ws(size_t& pos);
  bool        eq(size_t& pos);
  bool        nl(size_t& pos);
  bool        is_code();
  bool        is_topcode();
  bool        is_classcode();
  bool        is_initcode();
  std::string get_name(size_t& pos);
  std::string get_option(size_t& pos);
  std::string get_start(size_t& pos);
  std::string get_string(size_t& pos);
  bool        get_pattern(size_t& pos, std::string& pattern, std::string& regex);
  std::string get_namespace(size_t& pos);
  std::string get_code(size_t& pos);
  std::string escape_bs(const std::string& s);
  std::string upper_name(const std::string& s);
  std::string param_args(const std::string& s);
  bool        get_starts(size_t& pos, Starts& starts);
  void        abort(const char *message, const char *arg = NULL);
  void        error(const char *message, const char *arg = NULL, size_t at_lineno = 0);
  void        warning(const char *message, const char *arg = NULL, size_t at_lineno = 0);
  const char *SGR(const char *code) { return color_term ? code : ""; }

  */
}
