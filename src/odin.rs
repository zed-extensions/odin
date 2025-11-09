use std::fs;
use zed::{
    BuildTaskDefinition, BuildTaskDefinitionTemplatePayload, BuildTaskTemplate, DebugAdapterBinary,
    DebugRequest, DebugScenario, LanguageServerId, LaunchRequest, StartDebuggingRequestArguments,
    StartDebuggingRequestArgumentsRequest, TaskTemplate, Worktree,
};
use zed_extension_api::{self as zed, serde_json, settings::LspSettings, DebugConfig, Result};

struct OdinExtension {
    cached_binary_path: Option<String>,
}

const GITHUB_REPO: &str = "DanielGavin/ols";
const ADAPTER_NAME_LLDB: &str = "Odin-LLDB";
const ADAPTER_NAME_GDB: &str = "Odin-GDB";

impl OdinExtension {
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
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                return Ok(path.to_string());
            }
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let release = zed::latest_github_release(
            GITHUB_REPO,
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: true,
            },
        )?;

        let (platform, arch) = zed::current_platform();

        let arch: &str = match arch {
            zed::Architecture::Aarch64 => "arm64",
            zed::Architecture::X8664 => "x86_64",
            zed::Architecture::X86 => return Err("Unsupported platform x86".into()),
        };

        let os: &str = match platform {
            zed::Os::Mac => "darwin",
            zed::Os::Linux => "unknown-linux-gnu",
            zed::Os::Windows => "pc-windows-msvc",
        };

        let file_name = format!("ols-{arch}-{os}");
        let asset_name = format!("{file_name}.zip");

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no asset found matching {:?}", asset_name))?;

        let version_dir = format!("ols-{}", release.version);
        let binary_path = format!(
            "{version_dir}/{file_name}{extension}",
            extension = match platform {
                zed::Os::Windows => ".exe",
                _ => "",
            },
        );

        if !fs::metadata(&binary_path).map_or(false, |stat| stat.is_file()) {
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

    fn get_dap_binary(
        &mut self,
        adapter_name: String,
        config: zed::DebugTaskDefinition,
        user_provided_debug_adapter_path: Option<String>,
        worktree: &Worktree,
    ) -> Result<DebugAdapterBinary, String> {
        match adapter_name.as_str() {
            ADAPTER_NAME_LLDB => {
                // Priority order for finding lldb-dap:
                // 1. User-provided path (from debug.json or UI)
                // 2. Settings configuration (lsp.lldb-dap.binary.path)
                // 3. System PATH (lldb-dap)
                // 4. Fallback (lldb-vscode)
                let debugger_path = user_provided_debug_adapter_path
                    .or_else(|| {
                        LspSettings::for_worktree("lldb-dap", worktree)
                            .ok()
                            .and_then(|settings| settings.binary)
                            .and_then(|binary| binary.path)
                    })
                    .or_else(|| worktree.which("lldb-dap"))
                    .or_else(|| worktree.which("lldb-vscode"))
                    .ok_or("lldb-dap not found. Please install LLDB or configure the path in settings.")?;

                Ok(DebugAdapterBinary {
                    command: Some(debugger_path),
                    arguments: vec![],
                    envs: vec![],
                    cwd: None,
                    connection: None,
                    request_args: StartDebuggingRequestArguments {
                        configuration: config.config,
                        request: StartDebuggingRequestArgumentsRequest::Launch,
                    },
                })
            }
            ADAPTER_NAME_GDB => {
                // Priority order for finding gdb:
                // 1. User-provided path (from debug.json or UI)
                // 2. Settings configuration (lsp.gdb.binary.path)
                // 3. System PATH (gdb)
                let debugger_path = user_provided_debug_adapter_path
                    .or_else(|| {
                        LspSettings::for_worktree("gdb", worktree)
                            .ok()
                            .and_then(|settings| settings.binary)
                            .and_then(|binary| binary.path)
                    })
                    .or_else(|| worktree.which("gdb"))
                    .ok_or(
                        "gdb not found. Please install GDB or configure the path in settings.",
                    )?;

                Ok(DebugAdapterBinary {
                    command: Some(debugger_path),
                    // GDB requires --interpreter=dap flag for DAP support
                    arguments: vec!["--interpreter=dap".to_string()],
                    envs: vec![],
                    cwd: None,
                    connection: None,
                    request_args: StartDebuggingRequestArguments {
                        configuration: config.config,
                        request: StartDebuggingRequestArgumentsRequest::Launch,
                    },
                })
            }
            _ => Err(format!("Unknown debug adapter: {}", adapter_name)),
        }
    }

    fn dap_request_kind(
        &mut self,
        adapter_name: String,
        config: serde_json::Value,
    ) -> Result<StartDebuggingRequestArgumentsRequest, String> {
        match adapter_name.as_str() {
            ADAPTER_NAME_LLDB | ADAPTER_NAME_GDB => {}
            _ => return Err(format!("Unknown debug adapter: {}", adapter_name)),
        }

        let request_type = config.get("request").and_then(|v| v.as_str()).unwrap();

        match request_type {
            "launch" => Ok(StartDebuggingRequestArgumentsRequest::Launch),
            "attach" => Ok(StartDebuggingRequestArgumentsRequest::Attach),
            _ => Err(format!("Unknown debug request type: {}", request_type)),
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
        // Only handle Odin tasks
        if build_task.command != "odin" {
            return None;
        }

        // Check if this is a "run" command
        if build_task.args.is_empty() || build_task.args[0] != "run" {
            return None;
        }

        let cwd = build_task.cwd.clone();
        let env = build_task.env.clone();

        // Convert "odin run" to "odin build" with -debug flag
        let mut build_args = build_task.args.clone();
        build_args[0] = "build".to_string();

        // Check if this is file mode
        let is_file_mode = build_args.contains(&"-file".to_string());

        // Determine output binary name and add to build args
        let output_name = if is_file_mode {
            // For file mode: use the file stem as binary name
            "$ZED_STEM".to_string()
        } else {
            // For package mode: use a fixed debug binary name
            "odin_debug".to_string()
        };

        // Add -out flag to control output name
        build_args.push(format!("-out:{}", output_name));

        // Add -debug flag if not present
        if !build_args.contains(&"-debug".into()) {
            build_args.push("-debug".into());
        }

        // Create the build task template
        let build_template = BuildTaskTemplate {
            label: "odin build -debug".into(),
            command: build_task.command.clone(),
            args: build_args,
            env,
            cwd,
        };

        // Config is Null - the actual launch config comes from run_dap_locator
        let config = serde_json::Value::Null;
        let Ok(config) = serde_json::to_string(&config) else {
            return None;
        };

        // Remove 'run: ' from the task label, since 'debug: ' will be prepended by default
        let label = resolved_label
            .clone()
            .strip_prefix("run: ")
            .unwrap_or(&resolved_label)
            .to_string();

        Some(DebugScenario {
            adapter: debug_adapter_name,
            label: label,
            config,
            tcp_connection: None,
            build: Some(BuildTaskDefinition::Template(
                BuildTaskDefinitionTemplatePayload {
                    template: build_template,
                    locator_name: Some(locator_name.into()),
                },
            )),
        })
    }

    fn run_dap_locator(
        &mut self,
        _locator_name: String,
        build_task: TaskTemplate,
    ) -> Result<DebugRequest, String> {
        // Only handle Odin build tasks
        if build_task.command != "odin"
            || build_task.args.is_empty()
            || build_task.args[0] != "build"
        {
            return Err("Not an Odin build task".to_string());
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
        let program = format!("{}/{}", cwd, output_name);

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
