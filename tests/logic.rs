#[allow(dead_code)]
#[path = "../src/logic.rs"]
mod logic;

use logic::*;
use std::collections::BTreeSet;

const NOW: u64 = 1_800_000_000;
const EXE: &str = "ols-arm64-darwin";

fn record(age_secs: u64, version: &str) -> String {
    format_check_record(NOW - age_secs, version)
}

#[test]
fn check_version_returned_within_interval() {
    assert_eq!(
        fresh_check_version(&record(0, "dev-2026-06"), NOW),
        Some("dev-2026-06")
    );
    assert_eq!(
        fresh_check_version(&record(RELEASE_CHECK_INTERVAL_SECS - 1, "nightly"), NOW),
        Some("nightly")
    );
    assert_eq!(
        fresh_check_version(&format!("{}\n", record(60, "dev-2026-06")), NOW),
        Some("dev-2026-06")
    );
}

#[test]
fn check_is_stale_at_or_past_interval() {
    assert_eq!(
        fresh_check_version(&record(RELEASE_CHECK_INTERVAL_SECS, "dev-2026-06"), NOW),
        None
    );
    assert_eq!(fresh_check_version("0 dev-2026-06", NOW), None);
}

#[test]
fn check_is_stale_for_invalid_or_future_records() {
    assert_eq!(fresh_check_version("", NOW), None);
    assert_eq!(fresh_check_version("not-a-number dev-2026-06", NOW), None);
    assert_eq!(fresh_check_version("99999999999999999999999 x", NOW), None);
    assert_eq!(fresh_check_version(&NOW.to_string(), NOW), None);
    assert_eq!(
        fresh_check_version(&format_check_record(NOW + 60, "dev-2026-06"), NOW),
        None
    );
}

#[test]
fn version_dir_matches_tag_and_release_version() {
    assert_eq!(ols_version_dir("dev-2026-06"), "ols-dev-2026-06");
    assert_eq!(ols_version_dir(NIGHTLY_TAG), "ols-nightly");
}

#[test]
fn reusable_version_rules() {
    assert_eq!(
        reusable_version(Some("dev-2026-05"), None, NOW).as_deref(),
        Some("dev-2026-05")
    );
    assert_eq!(
        reusable_version(Some("dev-2026-05"), Some(&record(60, "dev-2026-06")), NOW).as_deref(),
        Some("dev-2026-05")
    );
    assert_eq!(
        reusable_version(None, Some(&record(60, "dev-2026-06")), NOW).as_deref(),
        Some("dev-2026-06")
    );
    assert_eq!(reusable_version(None, None, NOW), None);
    assert_eq!(
        reusable_version(
            None,
            Some(&record(RELEASE_CHECK_INTERVAL_SECS, "dev-2026-06")),
            NOW
        ),
        None
    );
    assert_eq!(
        reusable_version(None, Some(&record(60, NIGHTLY_TAG)), NOW),
        None
    );
    assert_eq!(
        reusable_version(Some(NIGHTLY_TAG), Some(&record(60, NIGHTLY_TAG)), NOW).as_deref(),
        Some(NIGHTLY_TAG)
    );
    assert_eq!(
        reusable_version(
            Some(NIGHTLY_TAG),
            Some(&record(RELEASE_CHECK_INTERVAL_SECS, NIGHTLY_TAG)),
            NOW
        ),
        None
    );
    assert_eq!(
        reusable_version(Some(NIGHTLY_TAG), Some(&record(60, "dev-2026-06")), NOW),
        None
    );
}

#[test]
fn recording_and_replacement_rules() {
    assert!(!should_record_check(Some("dev-2026-05")));
    assert!(should_record_check(None));
    assert!(should_record_check(Some(NIGHTLY_TAG)));
    assert!(must_replace_download(Some(NIGHTLY_TAG)));
    assert!(!must_replace_download(Some("dev-2026-06")));
    assert!(!must_replace_download(None));
}

#[test]
fn path_binary_is_outranked_by_explicit_pin() {
    assert!(use_path_binary(None));
    assert!(!use_path_binary(Some("dev-2026-06")));
    assert!(!use_path_binary(Some(NIGHTLY_TAG)));
}

#[test]
fn strip_extension_settings_removes_only_release_tag() {
    let mut settings = serde_json::json!({
        "release_tag": "nightly",
        "odin_command": "/usr/local/bin/odin",
    });
    strip_extension_settings(&mut settings);
    assert_eq!(
        settings,
        serde_json::json!({ "odin_command": "/usr/local/bin/odin" })
    );
}

#[test]
fn strip_extension_settings_ignores_non_objects() {
    for mut settings in [
        serde_json::Value::Null,
        serde_json::json!("nightly"),
        serde_json::json!([1, 2, 3]),
    ] {
        let before = settings.clone();
        strip_extension_settings(&mut settings);
        assert_eq!(settings, before);
    }
}

#[test]
fn initialization_defaults_never_override_user_options() {
    let defaults = merged_initialization_options(None);
    assert_eq!(
        defaults,
        serde_json::json!({
            "enable_hover": true,
            "enable_document_symbols": true,
            "enable_snippets": true,
            "enable_references": true,
            "enable_inlay_hints_params": true,
            "enable_inlay_hints_default_params": true,
        }),
        "changing the default set must be a deliberate, tested decision"
    );

    let user = serde_json::json!({
        "enable_hover": false,
        "collections": [{"name": "shared", "path": "/x"}],
    });
    let merged = merged_initialization_options(Some(user));
    assert_eq!(merged["enable_hover"], false);
    assert_eq!(merged["enable_snippets"], true);
    assert_eq!(merged["collections"][0]["name"], "shared");

    let passthrough = merged_initialization_options(Some(serde_json::json!(null)));
    assert_eq!(passthrough, serde_json::Value::Null);
}

#[test]
fn debug_output_names_are_derived_from_the_resolved_label() {
    assert_eq!(
        debug_output_name("run: 'main.odin'", ""),
        "debug_build-main_odin"
    );
    assert_eq!(
        debug_output_name("run: package 'src'", ""),
        "debug_build-package_src"
    );
    assert_eq!(
        debug_output_name("test: 'my_test'", ""),
        "debug_build-my_test"
    );
    assert_eq!(debug_output_name("test: 'src'", ""), "debug_build-src");
    assert_eq!(
        debug_output_name("run: 'main.odin'", ".exe"),
        "debug_build-main_odin.exe"
    );

    // A run and a test targeting the same relative dir must not collide.
    assert_ne!(
        debug_output_name("run: package 'src'", ""),
        debug_output_name("test: 'src'", "")
    );
    // Different targets must not collide.
    assert_ne!(
        debug_output_name("run: 'client.odin'", ""),
        debug_output_name("run: 'server.odin'", "")
    );

    // Defensive fallbacks: no "run: "/"test: " prefix, empty, or a label
    // that sanitizes to nothing must never produce a broken/empty name.
    assert_eq!(
        debug_output_name("package 'src'", ""),
        "debug_build-package_src"
    );
    assert_eq!(debug_output_name("", ""), "debug_build");
    assert_eq!(debug_output_name("run: ''", ""), "debug_build");
    assert_eq!(debug_output_name("test: ///", ".exe"), "debug_build.exe");
}

#[derive(Default)]
struct FakeHost {
    files: BTreeSet<String>,
    dirs: BTreeSet<String>,
    release: Option<Release>,
    download_error: Option<String>,
    download_produces_nothing: bool,
    fetched_tags: Vec<Option<String>>,
    downloaded_urls: Vec<String>,
    removed_dirs: Vec<String>,
    written_record: Option<String>,
    statuses: Vec<&'static str>,
}

impl FakeHost {
    fn with_release(version: &str) -> Self {
        FakeHost {
            release: Some(release(version)),
            ..FakeHost::default()
        }
    }

    fn add_download(&mut self, version: &str) {
        let dir = ols_version_dir(version);
        self.files.insert(format!("{dir}/{EXE}"));
        self.dirs.insert(dir);
    }

    fn fetch_count(&self) -> usize {
        self.fetched_tags.len()
    }
}

fn release(version: &str) -> Release {
    Release {
        version: version.to_string(),
        assets: vec![ReleaseAsset {
            name: format!("{EXE}.zip"),
            download_url: format!("https://example.com/{version}/{EXE}.zip"),
        }],
    }
}

impl Host for FakeHost {
    fn is_file(&self, path: &str) -> bool {
        self.files.contains(path)
    }

    fn list_ols_dirs(&self) -> Vec<String> {
        self.dirs.iter().cloned().collect()
    }

    fn fetch_release(&mut self, tag: Option<&str>) -> Result<Release, String> {
        self.fetched_tags.push(tag.map(str::to_string));
        self.release
            .clone()
            .ok_or_else(|| "network down".to_string())
    }

    fn download_and_install(
        &mut self,
        download_url: &str,
        version_dir: &str,
        binary_path: &str,
    ) -> Result<(), String> {
        self.downloaded_urls.push(download_url.to_string());
        if let Some(error) = &self.download_error {
            return Err(error.clone());
        }
        self.dirs.insert(version_dir.to_string());
        if !self.download_produces_nothing {
            self.files.insert(binary_path.to_string());
        }
        Ok(())
    }

    fn remove_dir(&mut self, dir: &str) {
        self.removed_dirs.push(dir.to_string());
        self.dirs.remove(dir);
        let prefix = format!("{dir}/");
        self.files.retain(|file| !file.starts_with(&prefix));
    }

    fn write_check_record(&mut self, contents: &str) {
        self.written_record = Some(contents.to_string());
    }

    fn set_status_checking(&mut self) {
        self.statuses.push("checking");
    }

    fn set_status_downloading(&mut self) {
        self.statuses.push("downloading");
    }
}

fn inputs<'a>(release_tag: Option<&'a str>, check_record: Option<&'a str>) -> ResolveInputs<'a> {
    ResolveInputs {
        cached_binary_path: None,
        release_tag,
        check_record,
        now_secs: Some(NOW),
        asset_stem: EXE,
        executable_name: EXE,
        separator: "/",
    }
}

#[test]
fn cached_path_reused_only_while_its_file_exists() {
    let mut host = FakeHost::with_release("dev-2026-06");
    host.add_download("dev-2026-05");
    let cached = format!("ols-dev-2026-05/{EXE}");

    let mut req = inputs(None, None);
    req.cached_binary_path = Some(&cached);
    assert_eq!(resolve_ols_binary(&mut host, &req), Ok(cached.clone()));
    assert_eq!(host.fetch_count(), 0);
    assert!(host.statuses.is_empty());

    host.remove_dir("ols-dev-2026-05");
    host.removed_dirs.clear();
    let resolved = resolve_ols_binary(&mut host, &req).unwrap();
    assert_eq!(resolved, format!("ols-dev-2026-06/{EXE}"));
    assert_eq!(host.fetch_count(), 1);
}

#[test]
fn pinned_monthly_uses_existing_download_without_network() {
    let mut host = FakeHost::default();
    host.add_download("dev-2026-05");

    let resolved = resolve_ols_binary(&mut host, &inputs(Some("dev-2026-05"), None)).unwrap();
    assert_eq!(resolved, format!("ols-dev-2026-05/{EXE}"));
    assert_eq!(host.fetch_count(), 0);
    assert_eq!(host.written_record, None);
}

#[test]
fn pinned_monthly_downloads_when_missing_and_writes_no_record() {
    let mut host = FakeHost::with_release("dev-2026-05");
    host.add_download("dev-2026-06");

    let resolved = resolve_ols_binary(&mut host, &inputs(Some("dev-2026-05"), None)).unwrap();
    assert_eq!(resolved, format!("ols-dev-2026-05/{EXE}"));
    assert_eq!(host.fetched_tags, vec![Some("dev-2026-05".to_string())]);
    assert_eq!(host.statuses, vec!["checking", "downloading"]);
    assert_eq!(host.removed_dirs, vec!["ols-dev-2026-06"]);
    assert_eq!(host.written_record, None);
}

#[test]
fn pinned_monthly_recovers_when_user_empties_the_directory() {
    let mut host = FakeHost::with_release("dev-2026-05");
    host.dirs.insert("ols-dev-2026-05".to_string());

    let resolved = resolve_ols_binary(&mut host, &inputs(Some("dev-2026-05"), None)).unwrap();
    assert_eq!(resolved, format!("ols-dev-2026-05/{EXE}"));
    assert_eq!(host.downloaded_urls.len(), 1);
}

#[test]
fn latest_with_fresh_record_reuses_exactly_the_recorded_release() {
    let mut host = FakeHost::default();
    host.add_download("dev-2026-05");
    host.add_download("dev-2026-06");

    let fresh = record(60, "dev-2026-06");
    let resolved = resolve_ols_binary(&mut host, &inputs(None, Some(&fresh))).unwrap();
    assert_eq!(resolved, format!("ols-dev-2026-06/{EXE}"));
    assert_eq!(host.fetch_count(), 0);
}

#[test]
fn latest_recovers_when_recorded_download_was_deleted() {
    let mut host = FakeHost::with_release("dev-2026-06");
    let fresh = record(60, "dev-2026-06");

    let resolved = resolve_ols_binary(&mut host, &inputs(None, Some(&fresh))).unwrap();
    assert_eq!(resolved, format!("ols-dev-2026-06/{EXE}"));
    assert_eq!(host.fetch_count(), 1);
    assert_eq!(host.written_record, Some(record(0, "dev-2026-06")));
}

#[test]
fn latest_with_stale_or_tampered_record_checks_github() {
    for bad_record in [
        None,
        Some(record(RELEASE_CHECK_INTERVAL_SECS, "dev-2026-06")),
        Some("garbage".to_string()),
        Some(NOW.to_string()),
        Some(format_check_record(NOW + 999, "dev-2026-06")),
    ] {
        let mut host = FakeHost::with_release("dev-2026-06");
        host.add_download("dev-2026-05");

        let resolved = resolve_ols_binary(&mut host, &inputs(None, bad_record.as_deref())).unwrap();
        assert_eq!(resolved, format!("ols-dev-2026-06/{EXE}"));
        assert_eq!(host.fetch_count(), 1, "record {bad_record:?} was trusted");
        assert_eq!(host.written_record, Some(record(0, "dev-2026-06")));
        assert!(host.removed_dirs.contains(&"ols-dev-2026-05".to_string()));
    }
}

#[test]
fn latest_skips_download_when_release_already_on_disk() {
    let mut host = FakeHost::with_release("dev-2026-06");
    host.add_download("dev-2026-06");

    let resolved = resolve_ols_binary(&mut host, &inputs(None, None)).unwrap();
    assert_eq!(resolved, format!("ols-dev-2026-06/{EXE}"));
    assert_eq!(host.fetch_count(), 1);
    assert!(host.downloaded_urls.is_empty());
    assert_eq!(host.statuses, vec!["checking"]);
    assert_eq!(host.written_record, Some(record(0, "dev-2026-06")));
}

#[test]
fn nightly_fresh_is_reused_and_stale_is_replaced() {
    let mut host = FakeHost::default();
    host.add_download(NIGHTLY_TAG);
    let fresh = record(60, NIGHTLY_TAG);
    let resolved = resolve_ols_binary(&mut host, &inputs(Some(NIGHTLY_TAG), Some(&fresh))).unwrap();
    assert_eq!(resolved, format!("ols-nightly/{EXE}"));
    assert_eq!(host.fetch_count(), 0);

    let mut host = FakeHost::with_release(NIGHTLY_TAG);
    host.add_download(NIGHTLY_TAG);
    let stale = record(RELEASE_CHECK_INTERVAL_SECS, NIGHTLY_TAG);
    let resolved = resolve_ols_binary(&mut host, &inputs(Some(NIGHTLY_TAG), Some(&stale))).unwrap();
    assert_eq!(resolved, format!("ols-nightly/{EXE}"));
    assert_eq!(host.fetched_tags, vec![Some(NIGHTLY_TAG.to_string())]);
    assert!(host.removed_dirs.contains(&"ols-nightly".to_string()));
    assert_eq!(host.downloaded_urls.len(), 1);
    assert_eq!(host.written_record, Some(record(0, NIGHTLY_TAG)));
}

#[test]
fn fetch_failure_falls_back_to_newest_intact_download() {
    let mut host = FakeHost::default();
    host.add_download("dev-2026-05");
    host.add_download("dev-2026-06");

    let resolved = resolve_ols_binary(&mut host, &inputs(None, None)).unwrap();
    assert_eq!(resolved, format!("ols-dev-2026-06/{EXE}"));

    let mut host = FakeHost::default();
    host.add_download("dev-2026-05");
    host.dirs.insert("ols-dev-2026-06".to_string());

    let resolved = resolve_ols_binary(&mut host, &inputs(None, None)).unwrap();
    assert_eq!(resolved, format!("ols-dev-2026-05/{EXE}"));
}

#[test]
fn fetch_failure_with_nothing_usable_errors_helpfully() {
    let mut host = FakeHost::default();
    let err = resolve_ols_binary(&mut host, &inputs(None, None)).unwrap_err();
    assert!(err.contains("Failed to download OLS language server"));

    let mut host = FakeHost::default();
    let err = resolve_ols_binary(&mut host, &inputs(Some("dev-9999-99"), None)).unwrap_err();
    assert!(err.contains("dev-9999-99"));
    assert!(err.contains("release_tag"));

    let mut host = FakeHost::default();
    host.dirs.insert("ols-dev-2026-06".to_string());
    assert!(resolve_ols_binary(&mut host, &inputs(None, None)).is_err());
}

#[test]
fn offline_pin_is_never_satisfied_by_a_different_version() {
    let mut host = FakeHost::default();
    host.add_download("dev-2026-06");

    let err = resolve_ols_binary(&mut host, &inputs(Some("dev-2026-05"), None)).unwrap_err();
    assert!(err.contains("dev-2026-05"));

    let mut host = FakeHost::default();
    host.add_download(NIGHTLY_TAG);
    host.add_download("dev-2026-06");
    let stale = record(RELEASE_CHECK_INTERVAL_SECS, NIGHTLY_TAG);
    let resolved = resolve_ols_binary(&mut host, &inputs(Some(NIGHTLY_TAG), Some(&stale))).unwrap();
    assert_eq!(resolved, format!("ols-nightly/{EXE}"));
}

#[test]
fn tampered_record_cannot_point_outside_the_work_dir() {
    assert_eq!(
        fresh_check_version(&record(60, "../../../usr/bin/evil"), NOW),
        None
    );
    assert_eq!(fresh_check_version(&record(60, "..\\evil"), NOW), None);
}

#[test]
fn release_tag_setting_is_validated() {
    let tag = |json: serde_json::Value| release_tag_from_settings(Some(&json));

    assert_eq!(
        tag(serde_json::json!({"release_tag": "dev-2026-06"})).as_deref(),
        Some("dev-2026-06")
    );
    assert_eq!(
        tag(serde_json::json!({"release_tag": "  nightly  "})).as_deref(),
        Some("nightly")
    );
    assert_eq!(tag(serde_json::json!({"release_tag": ""})), None);
    assert_eq!(tag(serde_json::json!({"release_tag": "   "})), None);
    assert_eq!(tag(serde_json::json!({"release_tag": 42})), None);
    assert_eq!(tag(serde_json::json!({"release_tag": null})), None);
    assert_eq!(tag(serde_json::json!({})), None);
    assert_eq!(release_tag_from_settings(None), None);
}

#[test]
fn download_failure_propagates_and_leaves_no_record() {
    let mut host = FakeHost::with_release("dev-2026-06");
    host.download_error = Some("failed to download file: connection reset".to_string());

    let err = resolve_ols_binary(&mut host, &inputs(None, None)).unwrap_err();
    assert!(err.contains("connection reset"));
    assert_eq!(host.written_record, None);
}

#[test]
fn download_that_yields_no_binary_is_rejected() {
    let mut host = FakeHost::with_release("dev-2026-06");
    host.download_produces_nothing = true;

    let err = resolve_ols_binary(&mut host, &inputs(None, None)).unwrap_err();
    assert!(err.contains("did not contain"));
    assert_eq!(host.written_record, None);
}

#[test]
fn release_without_matching_asset_errors() {
    let mut host = FakeHost::default();
    host.release = Some(Release {
        version: "dev-2026-06".to_string(),
        assets: vec![ReleaseAsset {
            name: "ols-source-only.tar.gz".to_string(),
            download_url: "https://example.com/src.tar.gz".to_string(),
        }],
    });

    let err = resolve_ols_binary(&mut host, &inputs(None, None)).unwrap_err();
    assert!(err.contains("no asset found"));
}

#[test]
fn clock_unavailable_still_resolves_but_never_records() {
    let mut host = FakeHost::with_release("dev-2026-06");
    let fresh = record(60, "dev-2026-06");
    let mut req = inputs(None, Some(&fresh));
    req.now_secs = None;

    let resolved = resolve_ols_binary(&mut host, &req).unwrap();
    assert_eq!(resolved, format!("ols-dev-2026-06/{EXE}"));
    assert_eq!(host.fetch_count(), 1);
    assert_eq!(host.written_record, None);
}

#[test]
fn pin_unpin_replay_never_serves_the_pinned_version() {
    let mut host = FakeHost::with_release("dev-2026-06");

    let resolved = resolve_ols_binary(&mut host, &inputs(None, None)).unwrap();
    assert_eq!(resolved, format!("ols-dev-2026-06/{EXE}"));
    let record_after_latest = host.written_record.clone().unwrap();

    host.release = Some(release("dev-2026-05"));
    let resolved = resolve_ols_binary(
        &mut host,
        &inputs(Some("dev-2026-05"), Some(&record_after_latest)),
    )
    .unwrap();
    assert_eq!(resolved, format!("ols-dev-2026-05/{EXE}"));
    assert_eq!(host.written_record, Some(record_after_latest.clone()));

    host.release = Some(release("dev-2026-06"));
    let resolved =
        resolve_ols_binary(&mut host, &inputs(None, Some(&record_after_latest))).unwrap();
    assert_eq!(resolved, format!("ols-dev-2026-06/{EXE}"));
    assert_eq!(host.list_ols_dirs(), vec!["ols-dev-2026-06".to_string()]);

    let latest_record = host.written_record.clone().unwrap();
    let fetches_before = host.fetch_count();
    let resolved = resolve_ols_binary(&mut host, &inputs(None, Some(&latest_record))).unwrap();
    assert_eq!(resolved, format!("ols-dev-2026-06/{EXE}"));
    assert_eq!(host.fetch_count(), fetches_before);
}

#[test]
fn windows_inputs_produce_backslash_paths_with_exe_suffix() {
    let stem = "ols-x86_64-pc-windows-msvc";
    let mut host = FakeHost::default();
    host.release = Some(Release {
        version: "dev-2026-06".to_string(),
        assets: vec![ReleaseAsset {
            name: format!("{stem}.zip"),
            download_url: format!("https://example.com/{stem}.zip"),
        }],
    });
    let req = ResolveInputs {
        cached_binary_path: None,
        release_tag: None,
        check_record: None,
        now_secs: Some(NOW),
        asset_stem: stem,
        executable_name: &format!("{stem}.exe"),
        separator: "\\",
    };

    let resolved = resolve_ols_binary(&mut host, &req).unwrap();
    assert_eq!(resolved, format!("ols-dev-2026-06\\{stem}.exe"));
    assert_eq!(host.written_record, Some(record(0, "dev-2026-06")));

    let resolved_again = resolve_ols_binary(&mut host, &req).unwrap();
    assert_eq!(resolved_again, resolved);
    assert_eq!(host.downloaded_urls.len(), 1);
}

#[test]
fn nightly_roundtrip_respects_the_setting_at_every_step() {
    let mut host = FakeHost::with_release(NIGHTLY_TAG);

    let resolved = resolve_ols_binary(&mut host, &inputs(Some(NIGHTLY_TAG), None)).unwrap();
    assert_eq!(resolved, format!("ols-nightly/{EXE}"));
    let nightly_record = host.written_record.clone().unwrap();

    host.release = Some(release("dev-2026-06"));
    let resolved = resolve_ols_binary(&mut host, &inputs(None, Some(&nightly_record))).unwrap();
    assert_eq!(resolved, format!("ols-dev-2026-06/{EXE}"));
    assert_eq!(host.list_ols_dirs(), vec!["ols-dev-2026-06".to_string()]);
}
