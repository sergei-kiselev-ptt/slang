# CLAUDE.md

## Project Overview

**slang** (working name, may change) is a custom programming language designed for building native desktop applications. The ultimate goal is a simple, cross-platform language with GUI components as part of its standard library.

### Design Goals
- Simple syntax requiring only general software engineering knowledge
- Cross-platform: MacOS, Linux (Arch), Windows
- Fast feedback loop: REPL now, hot-reload later
- Native, self-contained, small binaries (no Electron)

## Architecture

```
Source Code
    ↓
  Lexer (lexer.rs)
    ↓
  Tokens
    ↓
  Parser (parser.rs)
    ↓
  AST (Expr enum)
    ↓
  Cranelift JIT (jit.rs)
    ↓
  Native Machine Code
    ↓
  Execute
```

### Backend Options (for future native compilation)
- **Cranelift**: Current choice for JIT, can also do AOT compilation
- **QBE**: Simpler alternative (~10k lines vs LLVM's millions), "70% of LLVM performance in 10% of code"
- **LLVM**: Most optimized, but complex

## Current State

### Implemented
- **Lexer**: Full tokenization with comprehensive tests
- **Parser**: Recursive descent parser producing AST
- **AST**: Expression nodes (Literal, Unary, Binary, Variable, Assign)
- **JIT Compiler**: Cranelift-based, compiles expressions to native code
- **REPL**: Interactive mode with JIT compilation and variable persistence

### JIT Features
- Compiles each expression to native machine code
- Proper type system: `Number(f64)` and `Bool(bool)` as separate types
- Type inference for expressions
- Persistent variable storage across REPL evaluations
- ~1-5ms compilation latency per expression

### Grammar (Expression-only)
```
<expression>  ::= <assignment>
<assignment>  ::= <IDENTIFIER> "=" <assignment> | <logical_or>
<logical_or>  ::= <logical_and> ( "||" <logical_and> )*
<logical_and> ::= <equality> ( "&&" <equality> )*
<equality>    ::= <comparison> ( ( "==" | "!=" ) <comparison> )*
<comparison>  ::= <term> ( ( ">" | ">=" | "<" | "<=" ) <term> )*
<term>        ::= <factor> ( ( "+" | "-" ) <factor> )*
<factor>      ::= <unary> ( ( "*" | "/" ) <unary> )*
<unary>       ::= ( "-" | "+" | "!" ) <unary> | <primary>
<primary>     ::= <NUMBER> | <IDENTIFIER> | "true" | "false" | "(" <expression> ")"
```

### Supported Operations
- Arithmetic: `+`, `-`, `*`, `/`
- Comparison: `>`, `>=`, `<`, `<=`
- Equality: `==`, `!=`
- Logical: `&&`, `||`, `!`
- Assignment: `=` (right-associative, chainable: `x = y = 1`)
- Grouping: `()`

### Value Types
- `Number(f64)` - floating point numbers
- `Bool(bool)` - true/false

## Project Structure

```
src/
├── lib.rs       # Library exports
├── main.rs      # CLI entry point
├── lexer.rs     # Tokenization
├── parser.rs    # AST construction
├── jit.rs       # Cranelift JIT compiler
├── ir.rs        # Stack-based IR (legacy, may remove)
├── repl.rs      # REPL + tree-walking interpreter (legacy)
└── grammar.bnf  # Formal grammar spec
examples/
└── main.sl      # Example program
```

## Development

### Commands
```bash
cargo build              # Build
cargo test               # Run tests (120+ tests)
cargo run -- --repl      # Start JIT REPL
cargo run -- <file.sl>   # Execute file (uses old interpreter)
```

### REPL Usage
```
> 2 + 3
5
> x = 10
10
> y = 20
20
> x + y
30
> x > y
false
> flag = true
true
> !flag
false
```

### Testing
- Lexer: 33+ tests
- IR compiler: 12 tests
- JIT: 21 tests (numbers, bools, arithmetic, comparisons, logical ops, variables)

## Coding Conventions

- Use `Result<T, String>` for JIT compilation errors
- Use `JitValue` enum for runtime values
- Follow existing patterns in codebase
- Add tests for new functionality

## Planned Next Steps

### Extend Language
1. Add statements: `let` (explicit declaration), `print`
2. Add control flow: `if`/`else`, `while`
3. Add functions

### Native Compilation
1. Add AOT compilation mode using Cranelift
2. Or explore QBE as simpler backend

## Deferred
- Structured error reporting with spans
- Hot reload
- GUI components
- FFI
- Standard library
- String literals
