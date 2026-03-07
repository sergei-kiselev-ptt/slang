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
- **Lexer**: Full tokenization with comprehensive tests
- **Parser**: Recursive descent parser producing AST; newlines as statement separators (insignificant inside expressions)
- **AST**: `Literal`, `Unary`, `Binary`, `Variable`, `Assign`, `If`, `While`
- **QBE Compiler**: Compiles to QBE IR, produces native binaries
- **REPL**: Tree-walking interpreter (for quick experimentation)

### Grammar
```
<program>     ::= ( <expression> <NEWLINE>* )*
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
```

### Supported Operations
- Arithmetic: `+`, `-`, `*`, `/`
- Comparison: `>`, `>=`, `<`, `<=`
- Equality: `==`, `!=`
- Logical: `&&`, `||`, `!`
- Assignment: `=` (right-associative, chainable: `x = y = 1`)
- Grouping: `()`
- Control flow: `if`/`else`, `while`

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
├── qbe.rs       # QBE IR compiler
├── repl.rs      # REPL + tree-walking interpreter
└── grammar.bnf  # Formal grammar spec
examples/
└── main.sl      # Example program
```

## Development

### Commands
```bash
cargo build              # Build
cargo test               # Run tests (49 tests)
cargo run -- <file.sl>   # Compile file to QBE IR
cargo run -- --repl      # Start tree-walking REPL
bash run.sh              # Full pipeline: compile → qbe → cc → run
```

### run.sh pipeline
```bash
cargo run ./examples/main.sl && qbe -o .build/out.s .build/main.qbe && cc .build/out.s -o .build/out && ./.build/out
```

### Testing
- Lexer: 33 tests
- QBE compiler: 16 tests (literals, arithmetic, comparisons, logical ops, variables, if, while)

## Coding Conventions

- Use `Result<T, anyhow::Error>` for QBE compilation errors
- Use `ResType` enum (`Number`, `Bool`) for compile-time type tracking
- Labels in QBE IR use `@label_N` naming with counter-based unique IDs
- Follow existing patterns in codebase
- Add tests for new functionality

## Planned Next Steps

1. `print` statement
2. Functions
3. `let` for explicit variable declaration

## Deferred
- Structured error reporting with spans
- Hot reload
- GUI components
- FFI
- Standard library
- String literals
