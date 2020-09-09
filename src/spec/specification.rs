use super::*;
use structopt::StructOpt;
use std::io::{Write, Read};

//#[derive(Default)]
pub struct Specification<'s, 'r> {
  pub options : Options,    //< maps option name (from the options_table) to its option value
  color_term  : bool,       //< terminal supports colors

  output : Option<Box<dyn FnMut(&[u8])>>, //< output stream
  input  : Option<Box<dyn FnMut(&[u8])>>, //< input specification
  //out_handle : Option<Box<dyn Write>>,
  //in_handle  : Option<Box<dyn Read >>,

  conditions  : StrVec<'s>, //< "INITIAL" start condition etc. defined with %x name
  definitions : StrMap<'s>, //< map of {name} to regex
  inclusive   : Starts,     //< inclusive start conditions

  //infile       : String,       //< input file name
  //library      : Library,      //< the regex library selected
  line           : &'s str,      //< current line read from input
  linelen        : usize,        //< current line length
  lineno         : usize,        //< current line number at input
  patterns       : StrVec<'s>,   //< regex patterns for each start condition
  rules          : RulesMap<'r>, //< <Start_i>regex_j action for Start i Rule j
  section_1      : Codes,        //< %{ user code %} in section 1 container
  section_2      : CodesMap,     //< lexer user code in section 2 container
  section_3      : Codes,        //< main user code in section 3 container
  section_init   : Codes,        //< %init{ init code %} in section 1 container
  section_struct : Codes,        //< %class{ class code %} in section 1 container
  section_top    : Codes,        //< %top{ user code %} in section 1 container
}

impl<'s, 'r> Default for Specification {
  fn default() -> Self {
    Self {
      options        : Options::from_args(),
      output         : None,
      input          : None,
      //out_handle     : None,
      //in_handle      : None,
      color_term     : true,
      conditions     : StrVec::default(),
      definitions    : StrMap::default(),
      inclusive      : Starts::default(),
      //infile       : String::default(),
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
    }
  }
}

impl<'s, 'r> Specification<'s, 'r> {
  pub fn new() -> Self {
    Self {
      options: Options::from_args(),
      ..Self::default()
    }
  }

  pub fn parse(&mut self) {
    self.conditions.push("INITIAL");
    self.inclusive.insert(0);
    self.lineno = 0;

    // If there were a choice of libraries...
    //set_library();

    fn write_factory(mut out: Box<dyn Write>){
      move | buf | {
        out.write_all(buf)
      };
    }


    if let Some(path) =  self.options.outfile {
      let mut file = std::io::Stdout();

      self.out = Some(
        move | buf | {
          std_out.write_all(buf);
        }
      );
    } else {
      let mut std_out = std::io::Stdout();

      self.out = Some(
        move | buf | {
          std_out.write_all(buf);
        }
      );
    }
  }

  /*
  void        parse_section_1();
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
