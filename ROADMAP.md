# Roadmap

## Planned Next Steps

| # | Feature | Notes |
|---|---------|-------|
| 1 | ~~Error reporting~~ | Done — spans, no panics, source snippets |
| 2 | ~~Immutability (`let` vs `let mut`)~~ | Done — `let` immutable by default; `let mut` for mutable |
| 3 | `for` loop + `break`/`continue` | Range-based: `for i in 0..n { }` |
| 4 | Modulo `%` + compound assignment (`+=` etc.) | Small additions |
| 5 | ~~Integer type `int` (i64)~~ | Done — `42` → int, `42.0` → num (dot distinguishes) |
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
