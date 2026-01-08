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
    cached_binary_path: Option<String>,
}

const GITHUB_REPO: &str = "DanielGavin/ols";

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

    fn find_existing_ols_binary(&self) -> Option<String> {
        let entries = fs::read_dir(".").ok()?;
        let (platform, arch) = zed::current_platform();
        let binary_name = self.ols_binary_name(platform, arch)?;
        let executable_name = format!("{}{}", binary_name, Self::exe_suffix(platform));
        let separator = Self::path_separator(platform);

        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name_str = file_name.to_str()?;
            if name_str.starts_with("ols-") && entry.path().is_dir() {
                let binary_path = entry.path().join(&executable_name);
                if binary_path.is_file() {
                    let full_path = format!("{}{}{}", name_str, separator, executable_name);
                    return Some(full_path);
                }
            }
        }

        None
    }

    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<String> {
        let language_server = language_server_id.as_ref();
        if let Some(path) = LspSettings::for_worktree(language_server, worktree)
            .ok()
            .and_then(|settings| settings.binary)
            .and_then(|binary| binary.path)
        {
            return Ok(path);
        }

        if let Some(path) = worktree.which(language_server) {
            return Ok(path);
        }

        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).is_ok_and(|stat| stat.is_file()) {
                return Ok(path.to_string());
            }
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let release = match zed::latest_github_release(
            GITHUB_REPO,
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        ) {
            Ok(release) => release,
            Err(e) => {
                if let Some(path) = self.find_existing_ols_binary() {
                    self.cached_binary_path = Some(path.clone());
                    return Ok(path);
                }

                return Err(format!(
                    "Failed to download OLS language server: {}\n\n\
                    To resolve this issue, you can connect to the internet and restart Zed or Manually install OLS.",
                    e
                ));
            }
        };

        let (platform, arch) = zed::current_platform();
        let file_name = self
            .ols_binary_name(platform, arch)
            .ok_or_else(|| format!("Unsupported platform {:?}", arch))?;
        let asset_name = format!("{}.zip", file_name);

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no asset found matching {:?}", asset_name))?;

        let version_dir = format!("ols-{}", release.version);
        let separator = Self::path_separator(platform);
        let binary_path = format!(
            "{}{}{}{}",
            version_dir,
            separator,
            file_name,
            Self::exe_suffix(platform)
        );

        if !fs::metadata(&binary_path).is_ok_and(|stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            zed::download_file(
                &asset.download_url,
                &version_dir,
                zed::DownloadedFileType::Zip,
            )
            .map_err(|e| format!("failed to download file: {e}"))?;

            zed::make_file_executable(&binary_path)?;

            // Cleanup older versions
            let entries =
                fs::read_dir(".").map_err(|e| format!("failed to list working directory {e}"))?;
            for entry in entries {
                let entry = entry.map_err(|e| format!("failed to load directory entry {e}"))?;
                if entry.file_name().to_str() != Some(&version_dir) {
                    fs::remove_dir_all(entry.path()).ok();
                }
            }
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
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
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        let ols_binary_path = self.language_server_binary_path(language_server_id, worktree)?;
        Ok(zed::Command {
            command: ols_binary_path,
            args: Default::default(),
            env: Default::default(),
        })
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<serde_json::Value>> {
        let settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.initialization_options.clone());
        Ok(settings)
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<serde_json::Value>> {
        let settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.settings.clone())
            .unwrap_or_default();
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
        let out_name = format!("debug_build{}", Self::exe_suffix(platform));
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
                "odin debug test build".into()
            } else {
                "odin debug build".into()
            },
            command: build_task.command.clone(),
            args: build_args,
            env: build_task.env.clone(),
            cwd: build_task.cwd.clone(),
        };

        // Config is Null - the actual launch config comes from run_dap_locator
        let config = serde_json::to_string(&serde_json::Value::Null).ok()?;

        // Remove the prefix from the task label, since 'debug: ' will be prepended by default if no prefix is provided
        let base_label = resolved_label
            .strip_prefix("run: ")
            .or_else(|| resolved_label.strip_prefix("test: "))
            .unwrap_or(&resolved_label) // Both sides are now &str
            .to_string();

        // Add a special prefix to clarify debugging tests
        let label = if is_test {
            format!("debug test: {}", base_label)
        } else {
            base_label
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
