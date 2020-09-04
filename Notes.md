# Questions

1. Could we not parse directly to `State`'s instead of passing through `Move`'s and `Edge`'s first? 
2. Must we have both `Char` and `Ranges`? Can we use a structure that can also be used by
 `Position`? 
3. What's with the weird opcode format?
4. What is with `trim_lazy`?

# Architecture ToDo's

1. Lots of loops are used to add ranges of values to Ranges objects. These should be replaced with
 pairs.

2. Possibly elaborate on the `Group`'s.
