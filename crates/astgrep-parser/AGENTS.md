# astgrep-parser

Tree-sitter language adapters, optimizer modules, and parser registry.

## Structure

```
src/
├── lib.rs                    # LanguageParserRegistry — maps Language→parser, file extension detection
├── registry.rs               # ConfigurableParserRegistry with per-language ParserConfig (timeout, size limits, recovery)
├── adapters.rs               # AstAdapter trait + AdapterContext — converts language ASTs → UniversalNode
├── base_adapter.rs           # BaseAdapter shared implementation
├── java.rs                   # JavaParser (tree-sitter 0.23.5)
├── javascript.rs             # JavaScriptParser (tree-sitter 0.25, handles .js/.jsx/.ts/.tsx)
├── javascript_optimizer.rs   # JS-specific AST optimizations
├── python.rs                 # PythonParser (tree-sitter 0.25)
├── sql.rs                    # SqlParser (tree-sitter-sequel 0.3.11, NOT tree-sitter-sql)
├── bash.rs                   # BashParser (tree-sitter 0.25, handles .sh/.bash/.zsh)
├── xml.rs                    # XmlParser
├── c.rs / c_simple.rs        # C parser adapters (no Language enum variant yet)
├── csharp.rs                 # C# adapter (no Language enum variant yet)
├── kotlin.rs                 # Kotlin adapter (no Language enum variant yet)
├── ruby.rs                   # Ruby adapter (no Language enum variant yet)
├── swift.rs                  # Swift adapter (no Language enum variant yet)
├── php.rs / php_optimizer.rs # PHP adapter + optimizer (no Language enum variant yet)
├── tree_sitter_parser.rs     # Generic tree-sitter integration utilities
├── script_discovery/         # Auto-detect language from file content/extension
│   ├── mod.rs
│   ├── discovery.rs          # Main discovery logic
│   ├── detection.rs          # Heuristic detection
│   └── extensions.rs         # Extension mapping tables
└── language_discovery/       # Test case auto-creation from discovered languages
    ├── test_case_creation.rs
    └── content_analysis.rs
```

## Where to Look

| Task | File | Key Type |
|------|------|----------|
| Add new language | `src/{lang}.rs` + update `Language` enum in astgrep-core | Implement `LanguageParser` trait |
| Register parser | `src/lib.rs` → `register_default_parsers()` | `LanguageParserRegistry` |
| Configure parser limits | `src/registry.rs` | `ParserConfig` (timeout, max_file_size, recovery) |
| Adapt AST nodes | `src/adapters.rs` | `AstAdapter` trait, `AdapterContext` |
| File extension → Language | `src/lib.rs` → `detect_language()` + `astgrep_core::Language::from_extension()` | — |
| Language detection heuristics | `src/language_discovery/detection.rs` | Content-based fallback |

## Conventions

- Each language module exports a `{Language}Parser` struct implementing `LanguageParser` trait
- Optimizer modules (`javascript_optimizer.rs`, `php_optimizer.rs`) normalize ASTs for better pattern matching
- Modules for languages NOT in `Language` enum (C, C#, Kotlin, Ruby, Swift, PHP) exist as tree-sitter adapters — they can parse but aren't wired into the full analysis pipeline
- `AdapterContext` carries `line_map` (byte offset → line:col) for location resolution

## Anti-Patterns

- Do NOT use `tree-sitter-sql` — SQL uses `tree-sitter-sequel` (version 0.3.11)
- Do NOT forget to update `register_default_parsers()` in `lib.rs` when adding a new Language enum variant
- Do NOT bypass `LanguageParserRegistry` — always go through it for parser access
- Do NOT hardcode language detection — use `detect_language()` or `Language::from_extension()`
