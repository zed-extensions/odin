# 🔨 Odin Language Support for Zed

This project provides Odin programming language support, featuring syntax highlighting and code navigation via Tree-sitter, Language Server capabilities like autocompletion and diagnostics, and full debugging support.

- Tree Sitter: [tree-sitter-odin](https://github.com/tree-sitter-grammars/tree-sitter-odin)
- Language Server: [@DanielGavin/ols](https://github.com/DanielGavin/ols)
- Debug Adapters: LLDB (Built-in)

---

## Language Server

This extension automatically updates to the latest OLS (Odin Language Server) nightly build on each startup.

### Using a Custom OLS Binary

If you want to use a specific OLS version or a locally built binary, you can override the automatic download:

```json
{
  "lsp": {
    "ols": {
      "binary": {
        "path": "/path/to/your/ols",
        "arguments": []
      }
    }
  }
}
```

---

## Configuration

You can customize OLS behavior if needed:

### Configure via Zed Settings (Recommended)

Add OLS configuration directly in your Zed `settings.json`. This approach works project-wide and doesn't require additional files:

```json
{
  "lsp": {
    "ols": {
      "initialization_options": {
        "enable_hover": true,
        "enable_snippets": true,
        "enable_procedure_snippet": true,
        "enable_completion_matching": true,
        "enable_references": true,
        "enable_document_symbols": true,
        "enable_format": true,
        "enable_document_links": true,
        "collections": [
          {
            "name": "core",
            "path": "/path/to/Odin/core"
          }
        ]
      }
    }
  }
}
```

**Note**: Replace `/path/to/Odin/core` with the actual path to your Odin installation's core library.

### Use `ols.json` in Workspace Root

Alternatively, create an `ols.json` file at the root of your workspace:

```json
{
  "$schema": "https://raw.githubusercontent.com/DanielGavin/ols/master/misc/ols.schema.json",
  "collections": [
    {
      "name": "core",
      "path": "/path/to/Odin/core"
    }
  ],
  "enable_hover": true,
  "enable_snippets": true,
  "enable_procedure_snippet": true,
  "enable_completion_matching": true,
  "enable_references": true,
  "enable_document_symbols": true,
  "enable_format": true,
  "enable_document_links": true
}
```

### Key Configuration Options

- **`enable_hover`**: Shows detailed documentation when hovering over symbols
- **`enable_snippets`**: Provides code templates and snippets
- **`enable_procedure_snippet`**: Auto-adds parentheses after function names for better UX
- **`enable_completion_matching`**: Enables type-aware completions that match argument types
- **`enable_references`**: Allows finding all references to symbols
- **`enable_document_symbols`**: Enables better outline/symbol navigation
- **`enable_format`**: Enables code formatting support via `odinfmt`
- **`enable_document_links`**: Enables clickable documentation links
- **`collections`**: Defines library paths that OLS indexes for completions (critical for core library support)

### Finding Your Odin Core Path

To find your Odin installation path:

```bash
# macOS/Linux
which odin
# Then navigate to ../core from the Odin binary location

# Or check ODIN_ROOT environment variable
echo $ODIN_ROOT
```

For more configuration options, see the [OLS documentation](https://github.com/DanielGavin/ols#configuration).

---

## Snippets

You can define custom code snippets to speed up your Odin development workflow.

### Creating Snippets

1. Open the command palette (`Cmd/Ctrl+Shift+P`)
2. Run `snippets: configure snippets`
3. Create or edit `odin.json` in the snippets directory
4. Add your snippets in JSON format

Example snippet:

```json
{
  "Main procedure": {
    "prefix": "main",
    "body": [
      "package main",
      "",
      "import \"core:fmt\"",
      "",
      "main :: proc() {",
      "\t$0",
      "}"
    ],
    "description": "Creates a main package with imports"
  }
}
```

For detailed information about creating and using snippets, see [Zed's snippet documentation](https://zed.dev/docs/snippets).

---

## Debugging

This extension supports debugging Odin applications using **LLDB**.

---
