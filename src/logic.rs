pub const RELEASE_CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;
pub const LAST_RELEASE_CHECK_FILE: &str = ".ols-last-release-check";
pub const NIGHTLY_TAG: &str = "nightly";

pub fn format_check_record(checked_at_secs: u64, version: &str) -> String {
    format!("{checked_at_secs} {version}")
}

pub fn fresh_check_version(record: &str, now_secs: u64) -> Option<&str> {
    let mut parts = record.split_whitespace();
    let checked_at: u64 = parts.next()?.parse().ok()?;
    let version = parts.next().filter(|v| !v.contains(['/', '\\']))?;
    (now_secs.checked_sub(checked_at)? < RELEASE_CHECK_INTERVAL_SECS).then_some(version)
}

pub fn release_tag_from_settings(settings: Option<&serde_json::Value>) -> Option<String> {
    settings?
        .get("release_tag")?
        .as_str()
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_string)
}

pub fn ols_version_dir(version: &str) -> String {
    format!("ols-{version}")
}

pub fn reusable_version(
    release_tag: Option<&str>,
    check_record: Option<&str>,
    now_secs: u64,
) -> Option<String> {
    let checked = || check_record.and_then(|record| fresh_check_version(record, now_secs));
    match release_tag {
        Some(tag) if tag != NIGHTLY_TAG => Some(tag.to_string()),
        Some(_) => checked().filter(|v| *v == NIGHTLY_TAG).map(str::to_string),
        None => checked().filter(|v| *v != NIGHTLY_TAG).map(str::to_string),
    }
}

pub fn should_record_check(release_tag: Option<&str>) -> bool {
    release_tag.is_none() || release_tag == Some(NIGHTLY_TAG)
}

pub fn must_replace_download(release_tag: Option<&str>) -> bool {
    release_tag == Some(NIGHTLY_TAG)
}

pub fn use_path_binary(release_tag: Option<&str>) -> bool {
    release_tag.is_none()
}

pub fn strip_extension_settings(settings: &mut serde_json::Value) {
    if let Some(settings) = settings.as_object_mut() {
        settings.remove("release_tag");
    }
}

pub fn merged_initialization_options(user: Option<serde_json::Value>) -> serde_json::Value {
    let mut options = serde_json::json!({
        "enable_hover": true,
        "enable_document_symbols": true,
        "enable_snippets": true,
        "enable_references": true,
        "enable_inlay_hints_params": true,
        "enable_inlay_hints_default_params": true,
    });
    match user {
        Some(serde_json::Value::Object(user)) => {
            let defaults = options.as_object_mut().unwrap();
            for (key, value) in user {
                defaults.insert(key, value);
            }
            options
        }
        Some(other) => other,
        None => options,
    }
}

pub fn debug_output_name(resolved_label: &str, exe_suffix: &str) -> String {
    let target = resolved_label
        .strip_prefix("run: ")
        .or_else(|| resolved_label.strip_prefix("test: "))
        .unwrap_or(resolved_label);

    let mut sanitized = String::with_capacity(target.len());
    let mut last_was_underscore = false;
    for c in target.chars() {
        let c = if c.is_alphanumeric() { c } else { '_' };
        if c == '_' && last_was_underscore {
            continue;
        }
        last_was_underscore = c == '_';
        sanitized.push(c);
    }
    let trimmed = sanitized.trim_matches('_');

    if trimmed.is_empty() {
        format!("debug_build{exe_suffix}")
    } else {
        format!("debug_build-{trimmed}{exe_suffix}")
    }
}

#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Clone, Debug)]
pub struct ReleaseAsset {
    pub name: String,
    pub download_url: String,
}

pub trait Host {
    fn is_file(&self, path: &str) -> bool;
    fn list_ols_dirs(&self) -> Vec<String>;
    fn fetch_release(&mut self, tag: Option<&str>) -> Result<Release, String>;
    fn download_and_install(
        &mut self,
        download_url: &str,
        version_dir: &str,
        binary_path: &str,
    ) -> Result<(), String>;
    fn remove_dir(&mut self, dir: &str);
    fn write_check_record(&mut self, contents: &str);
    fn set_status_checking(&mut self);
    fn set_status_downloading(&mut self);
}

pub struct ResolveInputs<'a> {
    pub cached_binary_path: Option<&'a str>,
    pub release_tag: Option<&'a str>,
    pub check_record: Option<&'a str>,
    pub now_secs: Option<u64>,
    pub asset_stem: &'a str,
    pub executable_name: &'a str,
    pub separator: &'a str,
}

impl ResolveInputs<'_> {
    fn binary_path_in(&self, dir: &str) -> String {
        format!("{dir}{}{}", self.separator, self.executable_name)
    }
}

pub fn resolve_ols_binary(host: &mut dyn Host, inputs: &ResolveInputs) -> Result<String, String> {
    if let Some(path) = inputs.cached_binary_path.filter(|path| host.is_file(path)) {
        return Ok(path.to_string());
    }

    let now = inputs.now_secs.unwrap_or(0);
    if let Some(version) = reusable_version(inputs.release_tag, inputs.check_record, now) {
        let path = inputs.binary_path_in(&ols_version_dir(&version));
        if host.is_file(&path) {
            return Ok(path);
        }
    }

    host.set_status_checking();
    let release = match host.fetch_release(inputs.release_tag) {
        Ok(release) => release,
        Err(error) => {
            return offline_fallback(host, inputs)
                .ok_or_else(|| fetch_failed_message(inputs.release_tag, &error));
        }
    };

    let asset_name = format!("{}.zip", inputs.asset_stem);
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == asset_name)
        .ok_or_else(|| format!("no asset found matching {asset_name:?}"))?;

    let version_dir = ols_version_dir(&release.version);
    let binary_path = inputs.binary_path_in(&version_dir);

    if must_replace_download(inputs.release_tag) {
        host.remove_dir(&version_dir);
    }
    if !host.is_file(&binary_path) {
        host.set_status_downloading();
        host.download_and_install(&asset.download_url, &version_dir, &binary_path)?;
        if !host.is_file(&binary_path) {
            return Err(format!(
                "downloaded OLS release {} but it did not contain {:?}",
                release.version, binary_path
            ));
        }
    }

    for dir in host.list_ols_dirs() {
        if dir != version_dir {
            host.remove_dir(&dir);
        }
    }
    if should_record_check(inputs.release_tag) {
        if let Some(now) = inputs.now_secs {
            host.write_check_record(&format_check_record(now, &release.version));
        }
    }
    Ok(binary_path)
}

fn offline_fallback(host: &dyn Host, inputs: &ResolveInputs) -> Option<String> {
    match inputs.release_tag {
        Some(pinned_tag) => {
            let path = inputs.binary_path_in(&ols_version_dir(pinned_tag));
            host.is_file(&path).then_some(path)
        }
        None => newest_existing_binary(host, inputs),
    }
}

fn newest_existing_binary(host: &dyn Host, inputs: &ResolveInputs) -> Option<String> {
    let mut dirs = host.list_ols_dirs();
    dirs.sort();
    dirs.into_iter().rev().find_map(|dir| {
        let path = inputs.binary_path_in(&dir);
        host.is_file(&path).then_some(path)
    })
}

fn fetch_failed_message(release_tag: Option<&str>, error: &str) -> String {
    match release_tag {
        Some(tag) => format!(
            "Failed to download OLS release {tag:?}: {error}\n\n\
            OLS release tags look like \"dev-2026-06\" or \"nightly\"; check the \
            `release_tag` in your `lsp.ols.settings` and your internet connection.",
        ),
        None => format!(
            "Failed to download OLS language server: {error}\n\n\
            To resolve this issue, you can connect to the internet and restart Zed or Manually install OLS.",
        ),
    }
}
