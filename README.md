# Lesk lexer generator for Rust

## In the beginning...

In the beginning was lex, the standard lexical analyzer (lexer) generator on Unix. The beginning was 1975, and Alfred Aho had only just invented the mathematical machinery that powers lex, while Steve Johnson had just invented yacc. 

> As should be obvious from the above, the outside of Lex is patterned on Yacc and the inside on Aho's string matching routines.  Therefore, both S. C.  Johnson and A. V. Aho are really originators of much of Lex, as well as debuggers of it.  Many thanks are due to both.
>
> ​			—*Lex - A Lexical Analyzer Generator*, M. E. Lesk and E. Schmidt

Michael Lesk worked with a young intern named Eric Schmidt to produce lex. The haze of time has obscured how clever the name of Lesk's lex really is. 

Since that time, there have been countless literal and spiritual descendants of lex, most notably
 the mostly compatible flex, the *f* presumably standing for *fast*. Despite the multiplicity of competing tools, lex and flex have been a mainstay of the compiler writer's toolbox for nearly 50 years. I speculate that the maintenance of old code bases and familiarity with these tools have perpetuated their use. 

## The Genesis of Lesk

### Whence

[RE-flex](https://www.genivia.com/doc/reflex/html/index.html) is a modern reimagining of flex which provides source-file compatibility with flex (and lex), among many other features, written by Robert van Engelen in C++. (RE-flex is actually much more, a sort of swiss army knife for regex engines.)

Lesk is a rewrite of (some of) RE-flex in Rust. While the Rust ecosystem has several lexer tools, none of them are nearly as sophisticated as flex. It is a conspicuous hole in the ecosystem that Lesk intends to fill. Consider this my contribution to the "rewrite everything in Rust" hysteria. 

### Why

But why do this? For several reasons:

1. None of the Rust regex libraries are tailored to scanning.
2. Rust's memory and concurrency safety guarantees are attractive to many people.
3. My personal motivation: I want to write programs larger than 1000 LOC in Rust in order to
 understand the software engineering differences Rust has from other languages I know, and issues
  of software architecture do not readily present themselves in smaller code bases. Software engineering is much more than understanding syntax. Lesk will be 15,000-20,000 LOC, enough to be nontrivial.
4. Without intending to denigrate any other software developer, many of whom have been many times more successful in the discipline than I ever will be, it has been my experience that there is an acute lack of clean and readable code bases for such tools. 

### Relationship to RE-flex

I have no affiliation or involvement with RE-flex. Lesk is a bit more than "inspired" by RE-flex, as it takes algorithms straight from RE-flex's source code, none of which should be confused as my own work. At the same time, none of the RE-flex *code* as such is included in Lesk. Some of the comments have been copied and pasted verbatim in order to continue to benefit from Robert van Engelen's further technical progress with RE-flex. Lesk does not pretend to be as sophisticated as RE-flex, nor does it aspire to ever be so. Rather, my aspirations are to write a well-architected, readable piece of software that also happens to be useful to someone.

## Progress

The list below is more of a brain dump. As a task list, it leaves out far too much to be very useful. Most of the work will be thinking about design, not writing code.

* [x] Parse regexes
* [ ] Generate DFA tables
* [ ] Add compression options to the DFA generator
* [ ] Write a matcher
* [ ] Write a scanner
* [ ] Bootstrap a scanner/parser for flex spec files

# Notes

This is the initial outline of an algorithm for a standard table-driven lexer. The vision,
if it comes to pass, is to rewrite lex/flex using modern technology. What would these
programs look like if they were written today? How would their architecture be different?
I also want users of lex and flex to be immediately comfortable with the modern tool. Porting a
 complex grammar to Lesk shouldn't be impossible.

One of the surprises is that with a modern software ecosystem a significant portion of the
code in lex/flex is unnecessary. The lexer generator—let's also call this hypothetical modern
tool Lesk—doesn't need to provide the user with a stack to keep track of lexing modes, as a
stack is an off-the-shelf data structure in almost any modern language. The biggest savings in
effort is in not having to handle buffering and swapping out input sources. The vast majority
of users will just read the whole source file into memory, and those that don't want to
can provide their own adapter source, as a source is implemented with appropriate generics.

In fact, more generally, with the generics of modern languages, it should be far
easier to integrate the lexer —and just the lexer—into a wider variety of projects.

With modern templating tools like [Handlebars](https://handlebarsjs.com/), 
[Askama](https://github.com/djc/askama), and 
[liquid ](https://github.com/cobalt-org/liquid-rust),
developing and maintaining the code that is generated is far easier. Coroutines and
closures could potentially make faster lexers, though this theory has yet to be tested.

