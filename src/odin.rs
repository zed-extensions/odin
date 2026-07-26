use base64::{engine::general_purpose, Engine as _};
use std::fs;
use zed::{
    BuildTaskDefinition, BuildTaskDefinitionTemplatePayload, BuildTaskTemplate, DebugRequest,
    DebugScenario, LanguageServerId, LaunchRequest, TaskTemplate, Worktree,
};
use zed_extension_api::{
    self as zed,
    lsp::{Completion, CompletionKind, Symbol, SymbolKind},
    serde_json,
    settings::LspSettings,
    Architecture, CodeLabel, CodeLabelSpan, DebugConfig, Os, Result,
};

struct OdinExtension {
    cached_binary: Option<CachedBinary>,
}

struct CachedBinary {
    release_tag: Option<String>,
    path: String,
}

mod logic;
use logic::{
    debug_output_name, merged_initialization_options, release_tag_from_settings,
    resolve_ols_binary, strip_extension_settings, use_path_binary, Host, Release, ReleaseAsset,
    ResolveInputs, LAST_RELEASE_CHECK_FILE,
};

const GITHUB_REPO: &str = "DanielGavin/ols";

const ODIN_SCRIPT: &str = include_str!("../resources/lldb/odin.py");

impl OdinExtension {
    fn exe_suffix(platform: Os) -> &'static str {
        match platform {
            Os::Windows => ".exe",
            _ => "",
        }
    }

    fn path_separator(platform: Os) -> &'static str {
        match platform {
            Os::Windows => "\\",
            _ => "/",
        }
    }

    fn ols_binary_name(&self, platform: Os, arch: Architecture) -> Option<String> {
        let arch: &str = match arch {
            zed::Architecture::Aarch64 => "arm64",
            zed::Architecture::X8664 => "x86_64",
            zed::Architecture::X86 => return None, // Not supported
        };

        let os: &str = match platform {
            zed::Os::Mac => "darwin",
            zed::Os::Linux => "unknown-linux-gnu",
            zed::Os::Windows => "pc-windows-msvc",
        };

        let binary_name = format!("ols-{arch}-{os}");
        Some(binary_name)
    }

    fn unix_time_now() -> Option<u64> {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|duration| duration.as_secs())
    }

    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<String> {
        let lsp_settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree).ok();

        if let Some(path) = lsp_settings
            .as_ref()
            .and_then(|settings| settings.binary.as_ref())
            .and_then(|binary| binary.path.clone())
        {
            return Ok(path);
        }

        let release_tag =
            release_tag_from_settings(lsp_settings.as_ref().and_then(|s| s.settings.as_ref()));

        if use_path_binary(release_tag.as_deref()) {
            if let Some(path) = worktree.which(language_server_id.as_ref()) {
                return Ok(path);
            }
        }

        let (platform, arch) = zed::current_platform();
        let asset_stem = self
            .ols_binary_name(platform, arch)
            .ok_or_else(|| format!("Unsupported platform {:?}", arch))?;
        let executable_name = format!("{}{}", asset_stem, Self::exe_suffix(platform));
        let check_record = fs::read_to_string(LAST_RELEASE_CHECK_FILE).ok();

        let cached_binary_path = self
            .cached_binary
            .as_ref()
            .filter(|cached| cached.release_tag.as_deref() == release_tag.as_deref())
            .map(|cached| cached.path.clone());

        let inputs = ResolveInputs {
            cached_binary_path: cached_binary_path.as_deref(),
            release_tag: release_tag.as_deref(),
            check_record: check_record.as_deref(),
            now_secs: Self::unix_time_now(),
            asset_stem: &asset_stem,
            executable_name: &executable_name,
            separator: Self::path_separator(platform),
        };

        let mut host = ZedHost { language_server_id };
        let path = resolve_ols_binary(&mut host, &inputs)?;
        self.cached_binary = Some(CachedBinary {
            release_tag,
            path: path.clone(),
        });
        Ok(path)
    }
}

struct ZedHost<'a> {
    language_server_id: &'a LanguageServerId,
}

impl Host for ZedHost<'_> {
    fn is_file(&self, path: &str) -> bool {
        fs::metadata(path).is_ok_and(|stat| stat.is_file())
    }

    fn list_ols_dirs(&self) -> Vec<String> {
        let Ok(entries) = fs::read_dir(".") else {
            return Vec::new();
        };
        entries
            .flatten()
            .filter(|entry| entry.path().is_dir())
            .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
            .filter(|name| name.starts_with("ols-"))
            .collect()
    }

    fn fetch_release(&mut self, tag: Option<&str>) -> Result<Release, String> {
        let release = match tag {
            Some(tag) => zed::github_release_by_tag_name(GITHUB_REPO, tag),
            None => zed::latest_github_release(
                GITHUB_REPO,
                zed::GithubReleaseOptions {
                    require_assets: true,
                    pre_release: false,
                },
            ),
        }?;
        Ok(Release {
            version: release.version,
            assets: release
                .assets
                .into_iter()
                .map(|asset| ReleaseAsset {
                    name: asset.name,
                    download_url: asset.download_url,
                })
                .collect(),
        })
    }

    fn download_and_install(
        &mut self,
        download_url: &str,
        version_dir: &str,
        binary_path: &str,
    ) -> Result<(), String> {
        zed::download_file(download_url, version_dir, zed::DownloadedFileType::Zip)
            .map_err(|e| format!("failed to download file: {e}"))?;
        zed::make_file_executable(binary_path)
    }

    fn remove_dir(&mut self, dir: &str) {
        fs::remove_dir_all(dir).ok();
    }

    fn write_check_record(&mut self, contents: &str) {
        fs::write(LAST_RELEASE_CHECK_FILE, contents).ok();
    }

    fn set_status_checking(&mut self) {
        zed::set_language_server_installation_status(
            self.language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
    }

    fn set_status_downloading(&mut self) {
        zed::set_language_server_installation_status(
            self.language_server_id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );
    }
}

impl OdinExtension {
    fn is_integer_type(type_str: &str) -> bool {
        matches!(
            type_str,
            // Basic signed integers
            "int" | "i8" | "i16" | "i32" | "i64" | "i128" |
            // Basic unsigned integers
            "uint" | "u8" | "u16" | "u32" | "u64" | "u128" | "uintptr" |
            // Integer aliases
            "byte" | "rune" |
            // Little-endian integers
            "i16le" | "i32le" | "i64le" | "i128le" |
            "u16le" | "u32le" | "u64le" | "u128le" |
            // Big-endian integers
            "i16be" | "i32be" | "i64be" | "i128be" |
            "u16be" | "u32be" | "u64be" | "u128be"
        )
    }

    fn create_label(code: String, filter_len: usize) -> CodeLabel {
        let code_len = code.len();
        CodeLabel {
            code,
            spans: vec![CodeLabelSpan::code_range(0..code_len)],
            filter_range: (0..filter_len).into(),
        }
    }

    fn create_label_with_span(
        code: String,
        span_range: std::ops::Range<usize>,
        filter_len: usize,
    ) -> CodeLabel {
        CodeLabel {
            code,
            spans: vec![CodeLabelSpan::code_range(span_range)],
            filter_range: (0..filter_len).into(),
        }
    }
}

impl zed::Extension for OdinExtension {
    fn new() -> Self {
        Self {
            cached_binary: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        let binary_settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|settings| settings.binary);
        let args = binary_settings
            .as_ref()
            .and_then(|binary| binary.arguments.clone())
            .unwrap_or_default();
        let env = binary_settings
            .and_then(|binary| binary.env)
            .map(|env| env.into_iter().collect())
            .unwrap_or_default();

        let ols_binary_path = self.language_server_binary_path(language_server_id, worktree)?;
        Ok(zed::Command {
            command: ols_binary_path,
            args,
            env,
        })
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<serde_json::Value>> {
        let user_options = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.initialization_options.clone());
        Ok(Some(merged_initialization_options(user_options)))
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<serde_json::Value>> {
        let mut settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.settings.clone())
            .unwrap_or_default();
        strip_extension_settings(&mut settings);
        Ok(Some(settings))
    }

    fn label_for_completion(
        &self,
        _language_server_id: &LanguageServerId,
        completion: Completion,
    ) -> Option<CodeLabel> {
        use CompletionKind::*;

        let kind = completion.kind?;
        let label = &completion.label;
        let filter_len = label.len();

        match kind {
            Struct => {
                let code = match &completion.detail {
                    Some(detail) if detail.starts_with('[') || detail.starts_with("distinct") => {
                        format!("{} :: {}", label, detail)
                    }
                    _ => format!("{} :: struct", label),
                };
                Some(Self::create_label(code, filter_len))
            }

            Enum => {
                let code = match &completion.detail {
                    // OLS sends union type info in detail field (e.g., "union { int, f32 }")
                    // We can detect and display it correctly here
                    Some(detail) if detail.contains("union") => {
                        format!("{} :: union", label)
                    }
                    Some(detail) if Self::is_integer_type(detail) => {
                        format!("{} :: enum {}", label, detail)
                    }
                    _ => format!("{} :: enum", label),
                };
                Some(Self::create_label(code, filter_len))
            }

            Variable | Field => {
                let type_name = completion.detail.unwrap_or_else(|| "type".to_string());
                Some(Self::create_label(
                    format!("{}: {}", label, type_name),
                    filter_len,
                ))
            }

            Constant => {
                let value = completion.detail.unwrap_or_else(|| "value".to_string());
                Some(Self::create_label(
                    format!("{} :: {}", label, value),
                    filter_len,
                ))
            }

            EnumMember => {
                let code = format!(".{}", label);
                Some(Self::create_label_with_span(
                    code,
                    1..label.len() + 1,
                    filter_len,
                ))
            }

            Property => {
                let code = format!(".{}", label);
                Some(Self::create_label_with_span(
                    code,
                    1..label.len() + 1,
                    filter_len,
                ))
            }

            Keyword => Some(CodeLabel {
                code: label.clone(),
                spans: vec![CodeLabelSpan::literal(
                    label.clone(),
                    Some("keyword".to_string()),
                )],
                filter_range: (0..filter_len).into(),
            }),

            Module => {
                let code = format!("package {}", label);
                Some(Self::create_label_with_span(
                    code,
                    8..label.len() + 8,
                    filter_len,
                ))
            }

            _ => None,
        }
    }

    fn label_for_symbol(
        &self,
        _language_server_id: &LanguageServerId,
        symbol: Symbol,
    ) -> Option<CodeLabel> {
        // NOTE: Symbol navigation has limited type information compared to completions.
        // The LSP Symbol type only provides 'name' and 'kind', without detailed type info.

        use SymbolKind::*;

        let name = &symbol.name;
        let filter_len = name.len();

        match symbol.kind {
            Function => Some(Self::create_label(format!("{} :: proc", name), filter_len)),
            Variable => Some(Self::create_label(format!("{}: type", name), filter_len)),
            Struct => Some(Self::create_label(
                format!("{} :: struct", name),
                filter_len,
            )),
            // OLS sends both enums and unions as Enum kind (cannot distinguish in symbols)
            Enum => Some(Self::create_label(format!("{} :: enum", name), filter_len)),
            // Struct and union fields
            Field => Some(Self::create_label(format!("{}: type", name), filter_len)),
            _ => None,
        }
    }

    fn dap_config_to_scenario(&mut self, config: DebugConfig) -> Result<DebugScenario, String> {
        let mut config_map = serde_json::Map::new();
        match &config.request {
            DebugRequest::Launch(launch) => {
                config_map.insert("request".to_string(), serde_json::json!("launch"));
                config_map.insert("program".to_string(), serde_json::json!(&launch.program));

                if let Some(ref cwd) = launch.cwd {
                    config_map.insert("cwd".to_string(), serde_json::json!(cwd));
                }

                if !launch.args.is_empty() {
                    config_map.insert("args".to_string(), serde_json::json!(&launch.args));
                }

                if !launch.envs.is_empty() {
                    config_map.insert("env".to_string(), serde_json::json!(&launch.envs));
                }
            }
            DebugRequest::Attach(attach) => {
                config_map.insert("request".to_string(), serde_json::json!("attach"));
                config_map.insert("pid".to_string(), serde_json::json!(&attach.process_id));
            }
        }

        if let Some(stop_on_entry) = config.stop_on_entry {
            config_map.insert("stopOnEntry".to_string(), serde_json::json!(stop_on_entry));
        }

        let config_value = serde_json::Value::Object(config_map);
        let config_json = serde_json::to_string(&config_value)
            .map_err(|e| format!("Failed to serialize debug config: {}", e))?;

        Ok(DebugScenario {
            adapter: config.adapter,
            label: config.label,
            config: config_json,
            tcp_connection: None,
            build: None,
        })
    }

    fn dap_locator_create_scenario(
        &mut self,
        locator_name: String,
        build_task: TaskTemplate,
        resolved_label: String,
        debug_adapter_name: String,
    ) -> Option<DebugScenario> {
        let is_run = build_task.command == "odin" && build_task.args.first() == Some(&"run".into());
        let is_test =
            build_task.command == "odin" && build_task.args.first() == Some(&"test".into());

        if !is_run && !is_test {
            return None;
        }

        // Convert "odin run" to "odin build" with -debug flag
        let mut build_args = build_task.args.clone();
        build_args[0] = "build".to_string();

        // Add -out flag to control output name
        let (platform, _) = zed::current_platform();
        let build_target = build_task.args.get(1).map(String::as_str).unwrap_or("");
        let out_name = debug_output_name(build_target, Self::exe_suffix(platform));
        build_args.push(format!("-out:{}", out_name));

        // Add -debug flag if not present
        if !build_args.contains(&"-debug".into()) {
            build_args.push("-debug".into());
        }

        if is_test {
            build_args.push("-build-mode:test".into())
        }

        // Create the build task template
        let build_template = BuildTaskTemplate {
            label: if is_test {
                "odin debug test".into()
            } else {
                "odin debug build".into()
            },
            command: build_task.command.clone(),
            args: build_args,
            env: build_task.env.clone(),
            cwd: build_task.cwd.clone(),
        };

        let mut config_map = serde_json::Map::new();

        let encoded_script = general_purpose::STANDARD.encode(ODIN_SCRIPT);
        let exec_command = format!(
            "script import base64, types; odin = types.SimpleNamespace(); exec(base64.b64decode('{}').decode(), odin.__dict__); odin.__dict__['__lldb_init_module'](lldb.debugger, {{}})",
            encoded_script
        );

        config_map.insert(
            "preRunCommands".to_string(),
            serde_json::json!(vec![exec_command]),
        );

        let config = serde_json::to_string(&config_map).ok()?;

        // Update the task labels. The resulting label will be displayed as-is in
        // the F4 Debug menu and will have "Debug: " prepended to the label when
        // shown in the test gutter.
        let label = if is_run {
            resolved_label
                .strip_prefix("run: ")
                .unwrap_or(&resolved_label)
                .to_string()
        } else {
            resolved_label
                .strip_prefix("test: ")
                .map(|suffix| format!("test {}", suffix))
                .unwrap_or_else(|| resolved_label.clone())
        };

        Some(DebugScenario {
            adapter: debug_adapter_name,
            label,
            config,
            tcp_connection: None,
            build: Some(BuildTaskDefinition::Template(
                BuildTaskDefinitionTemplatePayload {
                    template: build_template,
                    locator_name: Some(locator_name),
                },
            )),
        })
    }

    fn run_dap_locator(
        &mut self,
        _locator_name: String,
        build_task: TaskTemplate,
    ) -> Result<DebugRequest, String> {
        // Only handle Odin build and test tasks
        if build_task.command != "odin"
            || build_task.args.is_empty()
            || !(build_task.args[0] == "build" || build_task.args[0] == "test")
        {
            return Err("Not an Odin build or test task".to_string());
        }

        // Extract the binary name from the -out: flag
        let output_name = build_task
            .args
            .iter()
            .find_map(|arg| arg.strip_prefix("-out:"))
            .ok_or_else(|| "Failed to extract output binary name from build task".to_string())?
            .to_string();

        // Construct absolute path to the binary, since lldb-dap requires absolute paths
        let cwd = build_task.cwd.as_ref().ok_or("No cwd in build task")?;
        let (platform, _) = zed::current_platform();
        let separator = Self::path_separator(platform);
        let program = format!("{}{}{}", cwd, separator, output_name);

        let request = LaunchRequest {
            program,
            cwd: build_task.cwd,
            args: vec![],
            envs: build_task.env.into_iter().collect(),
        };

        Ok(DebugRequest::Launch(request))
    }
}

zed::register_extension!(OdinExtension);
