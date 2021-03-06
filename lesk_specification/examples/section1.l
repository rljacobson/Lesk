%{
// C++ example taken from the Flex documentation
#include <iostream>
using namespace std;
int mylineno = 0;
%}

%option noyywrap

  #include <iostream>
  /* using namespace\* std;*/
  int mylineno = 0;

%init{

  {
    string  "[^\n\"]+"
    }

ws      [ \t]+
}

%top{
alpha   [A-Za-z]
dig     [0-9]
name    ({alpha}|{dig}|\$)({alpha}|{dig}|[_.\-/$])*
num1    [-+]?{dig}+\.?([eE][-+]?{dig}+)?
num2    [-+]?{dig}*\.{dig}+([eE][-+]?{dig}+)?
number  {num1}|{num2}
}
