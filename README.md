# 🔨 Odin Language Support for Zed

This project provides Odin programming language support, featuring syntax highlighting and code navigation via Tree-sitter, Language Server capabilities like autocompletion and diagnostics, and full debugging support.

- Tree Sitter: [tree-sitter-odin](https://github.com/tree-sitter-grammars/tree-sitter-odin)
- Language Server: [@DanielGavin/ols](https://github.com/DanielGavin/ols)
- Debug Adapters: LLDB (Built-in)

---

## Language Server

This extension automatically downloads the latest OLS (Odin Language Server) monthly build. To keep startup fast and avoid GitHub rate limits, it checks for updates at most once every 24 hours.

### Using a Custom OLS Binary

If you want to use a locally built binary, you can override the automatic download. `arguments` and `env` are passed to OLS whichever way the binary is resolved:

```json
{
  "lsp": {
    "ols": {
      "binary": {
        "path": "/path/to/your/ols",
        "arguments": [],
        "env": {}
      }
    }
  }
}
```

### Pinning an OLS Release

To pin a specific OLS release — or opt into the rolling nightly builds — set `release_tag` to any tag from the [OLS releases page](https://github.com/DanielGavin/ols/releases):

```json
{
  "lsp": {
    "ols": {
      "settings": {
        "release_tag": "dev-2026-06"
      }
    }
  }
}
```

Monthly tags (like `dev-2026-06`) are downloaded once and never re-checked. The special `nightly` tag is re-downloaded when a newer nightly build is available (checked at most once every 24 hours).

### Binary Resolution Order

The extension searches for the OLS binary in the following priority order:

1. **Custom binary path** - If configured in settings (see above), it is always used
2. **System PATH** - Checks if `ols` is available in your system PATH (skipped when `release_tag` is set — an explicit pin outranks implicit PATH discovery)
3. **Cached binary** - Uses a previously downloaded version if it matches the pinned tag, or if it is exactly the release found by a check less than 24 hours old
4. **GitHub download** - Downloads the configured `release_tag` (or the latest release) from [DanielGavin/ols](https://github.com/DanielGavin/ols/releases)

When GitHub is unreachable, the latest flow falls back to the newest intact download on disk. A pinned `release_tag` is only ever satisfied by that exact version — if its download is missing and GitHub is unreachable, the extension reports an error instead of silently running a different version.

---

## Configuration

### Out-of-the-Box Defaults

OLS ships with every feature disabled, so the extension enables a conservative set by default:

```jsonc
{
  "enable_hover": true,
  "enable_document_symbols": true,
  "enable_snippets": true,
  "enable_references": true,
  "enable_inlay_hints_params": true,
  "enable_inlay_hints_default_params": true
}
```

Inlay hints only appear if you also enable them in Zed. Formatting is deliberately **not** enabled by default — Zed formats on save out of the box, and silently reformatting existing codebases would be unwelcome; see the Formatting section below to opt in.

Your own settings always win: any key you set in `initialization_options` (including setting one of the above to `false`) overrides the default, and a project `ols.json` overrides both.

#### Configure via Zed Settings (Recommended)

Add OLS configuration directly in your Zed `settings.json`. This approach works project-wide and doesn't require additional files:

```jsonc
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
            "name": "shared",
            "path": "/path/to/shared"
          }
        ]
      }
    }
  }
}
```

#### Use `ols.json` in Workspace Root

Alternatively, create an `ols.json` file at the root of your workspace.For more configuration options, see the [OLS documentation](https://github.com/DanielGavin/ols#configuration).

### Collections Need to Be Passed to Tasks Too

`collections` (whether set via `initialization_options` or `ols.json`) only teach OLS how to resolve imports like `import "project:foo"` for indexing, hover, and go-to-definition. The `odin` compiler itself has no config file; it only learns about a collection from a `-collection:name=path` flag on the command line. Since this extension's built-in `run:`/`test:`/`build:`/`check:` tasks are static and don't know about your `ols.json`, running one on code that uses a custom collection fails with `Unknown library collection: '<name>'`, even though OLS resolved the same import correctly.

To fix this, add a `.zed/tasks.json` to your project overriding the tasks you use with the matching `-collection:` flag, e.g.:

```json
[
  {
    "label": "run: package '$ZED_RELATIVE_DIR'",
    "command": "odin",
    "args": ["run", "$ZED_DIRNAME", "-collection:project=."],
    "tags": ["odin-main"]
  }
]
```

A worktree-local task with the same label overrides the one this extension provides, so you only need to redefine the ones you actually run.

---

## Formatting with odinfmt

The simplest way to get formatting is through OLS: set `"enable_format": true` in your `initialization_options` (it is off by default so that format-on-save never surprises an existing codebase). This works for most setups, but some issues — most commonly reported by **Vim mode** users (formatting glitches that persist regardless of OLS settings) — are only fixed by bypassing the language server and running [odinfmt](https://github.com/DanielGavin/ols) as an external formatter:

```jsonc
{
  "languages": {
    "Odin": {
      "formatter": {
        "external": {
          "command": "odinfmt",
          "arguments": ["-stdin"]
        }
      },
      "format_on_save": "on"
    }
  }
}
```

### You already have odinfmt

Since OLS release `dev-2025-12`, the release zips bundle a prebuilt `odinfmt` executable — you don't need to build it yourself. Because this extension downloads those zips, odinfmt is already on your machine, next to the ols binary:

| OS | Path |
| --- | --- |
| macOS | `~/Library/Application Support/Zed/extensions/work/odin/ols-<release>/odinfmt-<arch>-darwin` |
| Linux | `~/.local/share/zed/extensions/work/odin/ols-<release>/odinfmt-<arch>-unknown-linux-gnu` |
| Windows | `%LOCALAPPDATA%\Zed\extensions\work\odin\ols-<release>\odinfmt-x86_64-pc-windows-msvc.exe` |

For example, on an Apple Silicon Mac running the `dev-2026-06` release, the `command` above would be:

```
~/Library/Application Support/Zed/extensions/work/odin/ols-dev-2026-06/odinfmt-arm64-darwin
```

> [!NOTE]
> The folder name contains the release version, and old folders are deleted when the extension updates — so a hard-coded path breaks on the next monthly release. Either pin the version with `release_tag` (see above) so the path stays stable, or copy the binary somewhere permanent / onto your `PATH` and use plain `"command": "odinfmt"`.

odinfmt reads its style options from an `odinfmt.json` in your project (e.g. `character_width`, `tabs`, `brace_style`, `sort_imports`) — see the [Odinfmt configurations](https://github.com/DanielGavin/ols#odinfmt-configurations).

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

Debugging uses **CodeLLDB**, which Zed's debugger installs automatically.

### Starting a session

Use the run icon in the gutter next to `main :: proc()` or any `@(test)` procedure and pick the debug variant, or open the debug panel and choose one of the detected `odin run` / `odin test` scenarios. The extension rebuilds the package with `odin build -debug -out:debug_build` (adding `-build-mode:test` for tests) and launches the result under the debugger — breakpoints and stepping work with no configuration.

### Odin-aware variable display

A bundled LLDB formatter is injected into every session, so the Variables panel shows Odin types natively:

| Type | Displayed as |
| --- | --- |
| `string` | the text, truncated past 4096 characters; zero-value strings show `""` |
| `cstring` | the text (via LLDB's C-string support) |
| `[]T`, `[dynamic]T` | element list, chunked for very large slices |
| `map[K]V` | `len = N, cap = M`, expanding to one `["key"] = value` row per entry |
| `union` | all variants with the active one marked `*`; nil unions show `nil`; `#no_nil` unions supported |
| `Maybe(T)` | the wrapped value directly, or `nil` |
| `bit_set` | Odin literal syntax: `{.Active, .Dirty}`, rune ranges as `{'b', 'd'}` |

Corrupted values degrade safely: a slice or map with a garbage length or a nil data pointer shows its raw fields instead of freezing the panel or spilling Python errors into the console.

### Custom debug scenarios

For anything beyond run/test — attaching to a process, custom binaries, arguments — add a `.zed/debug.json` to your project:

```json
[
  {
    "adapter": "CodeLLDB",
    "label": "Debug my_app",
    "request": "launch",
    "program": "$ZED_WORKTREE_ROOT/my_app",
    "cwd": "$ZED_WORKTREE_ROOT"
  }
]
```

Build the `program` with `-debug` so it carries debug info. On Windows, CodeLLDB debugs Odin binaries via LLDB's PDB support; the experience is solid for breakpoints and stepping, though some type rendering can be more limited than on macOS/Linux.

---
