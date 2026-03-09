# CLAUDE.md

## Project Overview

**slang** (working name, may change) is a custom programming language designed for building native desktop applications. The ultimate goal is a simple, cross-platform language with GUI components as part of its standard library.

### Design Goals
- Simple syntax requiring only general software engineering knowledge
- Cross-platform: MacOS, Linux (Arch), Windows
- Fast feedback loop: hot-reload later
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
  QBE Compiler (qbe.rs)
    ↓
  QBE IR (.qbe file)
    ↓
  QBE → Assembly → Native Binary
```

### Backend
- **QBE**: Current backend. ~10k lines, "70% of LLVM performance in 10% of code". Handles both the IR generation and assembly output.
- Run pipeline: `cargo run <file.sl>` → `.build/main.qbe` → `qbe` → `.build/out.s` → `cc` → `.build/out`

## Current State

### Implemented
- **Lexer**: Full tokenization with comprehensive tests; `Span` (line, col, len) on every token
- **Parser**: Recursive descent parser producing AST; newlines as statement separators (insignificant inside expressions); returns `Result<_, ParseError>` with source spans — no panics
- **AST**: `Literal`, `Unary`, `Binary`, `Variable`, `Assign`, `Let`, `If`, `While`, `Print`, `FuncDef`, `Call`; each node carries or delegates a `Span`
- **QBE Compiler**: Compiles to QBE IR, produces native binaries; type-checked (`ResType::Number | Bool | Void`); errors include source location
- **Error reporting**: Parse and compile errors display file, line:col, source line snippet, and caret underline

### Grammar
```
<program>     ::= ( <func_def> | <expression> <NEWLINE>* )*
<func_def>    ::= "func" <IDENTIFIER> "(" <params>? ")" "->" <type> "{" <expression>* "}"
<params>      ::= <param> ( "," <param> )*
<param>       ::= <IDENTIFIER> ":" <type>
<type>        ::= "num" | "bool"
<expression>  ::= <assignment>
<assignment>  ::= <IDENTIFIER> "=" <assignment> | <logical_or>
<logical_or>  ::= <logical_and> ( "||" <logical_and> )*
<logical_and> ::= <equality> ( "&&" <equality> )*
<equality>    ::= <comparison> ( ( "==" | "!=" ) <comparison> )*
<comparison>  ::= <term> ( ( ">" | ">=" | "<" | "<=" ) <term> )*
<term>        ::= <factor> ( ( "+" | "-" ) <factor> )*
<factor>      ::= <unary> ( ( "*" | "/" ) <unary> )*
<unary>       ::= ( "-" | "+" | "!" ) <unary> | <primary>
<primary>     ::= <NUMBER> | <IDENTIFIER> | "true" | "false"
                | "(" <expression> ")"
                | "if" <expression> "{" <expression> "}" ( "else" "{" <expression> "}" )?
                | "while" <expression> "{" <expression>* "}"
                | "print" <expression>
                | "let" <IDENTIFIER> ":" <type> "=" <expression>
                | <IDENTIFIER> "(" <args>? ")"
<args>        ::= <expression> ( "," <expression> )*
```

### Supported Operations
- Arithmetic: `+`, `-`, `*`, `/`
- Comparison: `>`, `>=`, `<`, `<=`
- Equality: `==`, `!=`
- Logical: `&&`, `||`, `!`
- Declaration: `let x: type = expr` (declares and initializes; shadows if name already exists)
- Assignment: `=` (reassigns a declared variable; error if undeclared; returns `Void` — not usable as expression)
- Grouping: `()`
- Control flow: `if`/`else` (condition must be `bool`), `while` (condition must be `bool`)
- Output: `print` (prints value, returns it)
- Functions: `func name(param: type) -> type { body }`, called as `name(arg)`

### Value Types
- `Number(f64)` - floating point numbers
- `Bool(bool)` - true/false
- `Void` - internal only; returned by assignment; not usable as a value anywhere

## Project Structure

```
src/
├── lib.rs       # Library exports
├── main.rs      # CLI entry point
├── lexer.rs     # Tokenization
├── parser.rs    # AST construction
├── qbe.rs       # QBE IR compiler
└── grammar.bnf  # Formal grammar spec
examples/
└── main.sl      # Example program
```

## Development

### Commands
```bash
cargo build              # Build
cargo test               # Run tests
cargo run -- <file.sl>   # Compile file to QBE IR
bash run.sh              # Full pipeline: compile → qbe → cc → run
```

### run.sh pipeline
```bash
cargo run ./examples/main.sl && qbe -o .build/out.s .build/main.qbe && cc .build/out.s -o .build/out && ./.build/out
```

### Testing
- Lexer tests: tokenization of all token types
- Parser tests: AST construction, error cases (unclosed blocks, wrong syntax, missing types)
- QBE compiler tests: literals, arithmetic, comparisons, logical ops, variables, let, if, while, print, functions

## Coding Conventions

- Use `Result<T, anyhow::Error>` for QBE compilation errors; `QbeError::new(msg, span)` for user-facing errors with location, `QbeError::no_span(msg)` for internal errors
- Use `ResType` enum (`Number`, `Bool`, `Void`) for compile-time type tracking
- Labels in QBE IR use `@label_N` naming with counter-based unique IDs
- Function parameters in QBE use `%p_<name>` to avoid collisions with `%t<N>` temporaries
- Functions are top-level only; all sigs are registered before any body is compiled (enables forward calls)
- Function body: last expression is the implicit return value
- Parser errors: use `ParseError` with `Span`; all parse methods return `Result<_, ParseError>`
- Follow existing patterns in codebase
- Add tests for new functionality

## Planned Next Steps

| # | Feature | Notes |
|---|---------|-------|
| 1 | ~~Error reporting~~ | Done — spans, no panics, source snippets |
| 2 | Immutability (`let` vs `let mut`) | `let` immutable by default; `let mut` for mutable |
| 3 | `for` loop + `break`/`continue` | Range-based: `for i in 0..n { }` |
| 4 | Modulo `%` + compound assignment (`+=` etc.) | Small additions |
| 5 | Integer type `int` (i64) | `42` → int, `42.0` → num (dot distinguishes) |
| 6 | Strings (basic) | Heap-allocated, `print`, concatenation |
| 7 | Type inference for `let` | `let x = 42` infers `int` |
| 8 | Arrays | `let xs: [int] = [1, 2, 3]`, indexing, `.len` |
| 9 | Structs | `struct Foo { x: num }`, field access `f.x` |
| 10 | Enums + `match` | Tagged unions, exhaustiveness checking |
| 11 | Modules/imports | `import "file.sl"` |

## Deferred
- Hot reload
- GUI components
- FFI / C interop
- Garbage collection / memory management strategy
- Closures / first-class functions
- Generics
- Concurrency
- Package manager
