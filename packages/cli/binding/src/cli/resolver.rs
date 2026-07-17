use std::{env, ffi::OsStr, iter, sync::Arc};

use rustc_hash::FxHashMap;
use vite_path::AbsolutePath;
use vite_str::Str;
use vite_task::config::user::{
    AutoTracking, EnabledCacheConfig, GlobWithBase, InputBase, UserCacheConfig, UserInputEntry,
};

use super::{
    help::should_prepend_vitest_run,
    types::{CliOptions, ResolvedSubcommand, ResolvedUniversalViteConfig, SynthesizableSubcommand},
};

/// Resolves synthesizable subcommands to concrete programs and arguments.
/// Used by both direct CLI execution and CommandHandler.
pub struct SubcommandResolver {
    cli_options: Option<CliOptions>,
    workspace_path: Arc<AbsolutePath>,
}

impl std::fmt::Debug for SubcommandResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubcommandResolver")
            .field("has_cli_options", &self.cli_options.is_some())
            .field("workspace_path", &self.workspace_path)
            .finish()
    }
}

impl SubcommandResolver {
    pub fn new(workspace_path: Arc<AbsolutePath>) -> Self {
        Self { cli_options: None, workspace_path }
    }

    pub fn with_cli_options(mut self, cli_options: CliOptions) -> Self {
        self.cli_options = Some(cli_options);
        self
    }

    fn cli_options(&self) -> anyhow::Result<&CliOptions> {
        self.cli_options
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("CLI options not available (running without NAPI?)"))
    }

    pub(crate) async fn resolve_universal_vite_config(
        &self,
    ) -> anyhow::Result<ResolvedUniversalViteConfig> {
        let cli_options = self.cli_options()?;
        let workspace_path_str = self
            .workspace_path
            .as_path()
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("workspace path is not valid UTF-8"))?;
        let vite_config_json =
            (cli_options.resolve_universal_vite_config)(workspace_path_str.to_string()).await?;

        Ok(serde_json::from_str(&vite_config_json).inspect_err(|_| {
            tracing::error!("Failed to parse vite config: {vite_config_json}");
        })?)
    }

    /// Resolve a synthesizable subcommand to a concrete program, args, cache config, and envs.
    pub(super) async fn resolve(
        &self,
        subcommand: SynthesizableSubcommand,
        resolved_vite_config: Option<&ResolvedUniversalViteConfig>,
        envs: &Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>,
    ) -> anyhow::Result<ResolvedSubcommand> {
        self.resolve_inner(subcommand, resolved_vite_config, envs).await
    }

    async fn resolve_inner(
        &self,
        subcommand: SynthesizableSubcommand,
        resolved_vite_config: Option<&ResolvedUniversalViteConfig>,
        envs: &Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>,
    ) -> anyhow::Result<ResolvedSubcommand> {
        match subcommand {
            SynthesizableSubcommand::Lint { mut args } => {
                let cli_options = self.cli_options()?;
                let resolved = (cli_options.lint)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("lint JS path is not valid UTF-8"))?;
                let owned_resolved_vite_config;
                let resolved_vite_config = if let Some(config) = resolved_vite_config {
                    config
                } else {
                    owned_resolved_vite_config = self.resolve_universal_vite_config().await?;
                    &owned_resolved_vite_config
                };

                if let (Some(_), Some(config_file)) =
                    (&resolved_vite_config.lint, &resolved_vite_config.config_file)
                {
                    args.insert(0, "-c".to_string());
                    args.insert(1, config_file.clone());
                }

                // When `GITHUB_ACTIONS=true`, oxlint auto-switches to the GitHub
                // reporter and omits the success summary ("Found 0 warnings and 0
                // errors"). Force the default reporter so a clean `vp lint` always
                // prints a visible summary — same approach as `vp check`.
                // Respect explicit user overrides (`--format` / `-f`, `--silent`).
                inject_lint_default_format(&mut args);

                Ok(ResolvedSubcommand {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from("--disable-warning=MODULE_TYPELESS_PACKAGE_JSON"))
                        .chain(iter::once(Str::from(js_path_str)))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    cache_config: UserCacheConfig::with_config(EnabledCacheConfig {
                        env: Some(Box::new([Str::from("OXLINT_TSGOLINT_PATH")])),
                        untracked_env: None,
                        input: None,
                        output: None,
                    }),
                    envs: merge_resolved_envs_with_version(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Fmt { mut args } => {
                let cli_options = self.cli_options()?;
                let resolved = (cli_options.fmt)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("fmt JS path is not valid UTF-8"))?;
                let owned_resolved_vite_config;
                let resolved_vite_config = if let Some(config) = resolved_vite_config {
                    config
                } else {
                    owned_resolved_vite_config = self.resolve_universal_vite_config().await?;
                    &owned_resolved_vite_config
                };

                if let (Some(_), Some(config_file)) =
                    (&resolved_vite_config.fmt, &resolved_vite_config.config_file)
                {
                    args.insert(0, "-c".to_string());
                    args.insert(1, config_file.clone());
                }

                Ok(ResolvedSubcommand {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from(js_path_str))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    cache_config: UserCacheConfig::with_config(EnabledCacheConfig {
                        env: None,
                        untracked_env: None,
                        input: None,
                        output: None,
                    }),
                    envs: merge_resolved_envs_with_version(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Build { args } => {
                let cli_options = self.cli_options()?;
                let resolved = (cli_options.vite)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("vite JS path is not valid UTF-8"))?;

                Ok(ResolvedSubcommand {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from(js_path_str))
                        .chain(iter::once(Str::from("build")))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    // No synthetic cache config: vite reports its inputs/outputs/
                    // envs to the runner via `@voidzero-dev/vite-task-client`.
                    // All fields `None` keep caching enabled with auto input and
                    // auto output inference (the latter drives output restoration);
                    // vite's `ignoreInput`/`ignoreOutput`/`getEnv`/`getEnvs` refine
                    // the fingerprint at runtime.
                    cache_config: UserCacheConfig::with_config(EnabledCacheConfig {
                        env: None,
                        untracked_env: None,
                        input: None,
                        output: None,
                    }),
                    envs: merge_resolved_envs_with_version(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Test { args } => {
                let cli_options = self.cli_options()?;
                let resolved = (cli_options.test)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("test JS path is not valid UTF-8"))?;
                let prepend_run = should_prepend_vitest_run(&args);
                let vitest_args: Vec<Str> = if prepend_run {
                    iter::once(Str::from("run")).chain(args.into_iter().map(Str::from)).collect()
                } else {
                    args.into_iter().map(Str::from).collect()
                };

                Ok(ResolvedSubcommand {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from(js_path_str)).chain(vitest_args).collect(),
                    cache_config: UserCacheConfig::with_config(EnabledCacheConfig {
                        env: None,
                        untracked_env: None,
                        input: Some(vec![
                            UserInputEntry::Auto(AutoTracking { auto: true }),
                            exclude_glob(
                                "!node_modules/.vite/vitest/**/results.json",
                                InputBase::Package,
                            ),
                        ]),
                        output: None,
                    }),
                    envs: merge_resolved_envs_with_version(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Pack { args } => {
                let cli_options = self.cli_options()?;
                let resolved = (cli_options.pack)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("pack JS path is not valid UTF-8"))?;

                Ok(ResolvedSubcommand {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from(js_path_str))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    cache_config: UserCacheConfig::with_config(EnabledCacheConfig {
                        env: None,
                        untracked_env: None,
                        input: Some(build_pack_cache_inputs()),
                        output: None,
                    }),
                    envs: merge_resolved_envs(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Dev { args } => {
                let cli_options = self.cli_options()?;
                let resolved = (cli_options.vite)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("vite JS path is not valid UTF-8"))?;

                Ok(ResolvedSubcommand {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from(js_path_str))
                        .chain(iter::once(Str::from("dev")))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    cache_config: UserCacheConfig::disabled(),
                    envs: merge_resolved_envs_with_version(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Preview { args } => {
                let cli_options = self.cli_options()?;
                let resolved = (cli_options.vite)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("vite JS path is not valid UTF-8"))?;

                Ok(ResolvedSubcommand {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from(js_path_str))
                        .chain(iter::once(Str::from("preview")))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    cache_config: UserCacheConfig::disabled(),
                    envs: merge_resolved_envs_with_version(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Doc { args } => {
                let cli_options = self.cli_options()?;
                let resolved = (cli_options.doc)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("doc JS path is not valid UTF-8"))?;

                Ok(ResolvedSubcommand {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from(js_path_str))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    cache_config: UserCacheConfig::with_config(EnabledCacheConfig {
                        env: None,
                        untracked_env: None,
                        input: None,
                        output: None,
                    }),
                    envs: merge_resolved_envs(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Check { .. } => {
                anyhow::bail!(
                    "Check is a composite command and cannot be resolved to a single subcommand"
                );
            }
        }
    }
}

/// Create a negative glob entry to exclude a pattern from cache fingerprinting.
fn exclude_glob(pattern: &str, base: InputBase) -> UserInputEntry {
    UserInputEntry::GlobWithBase(GlobWithBase { pattern: Str::from(pattern), base })
}

/// Common cache input entries for the pack command.
/// Excludes dist output files that are both read and written.
/// TODO: The hardcoded `!dist/**` exclusion is a temporary workaround. It will be replaced
/// by a runner-aware approach that automatically excludes task output directories.
fn build_pack_cache_inputs() -> Vec<UserInputEntry> {
    vec![
        UserInputEntry::Auto(AutoTracking { auto: true }),
        exclude_glob("!dist/**", InputBase::Package),
    ]
}

/// Cache input entries for the check command.
/// The vp check subprocess is a full vp CLI process (not resolved to a binary like
/// build/lint/fmt), so it accesses additional directories that must be excluded:
/// - `.vite/task-cache`: task runner state files that change after each run
pub(super) fn check_cache_inputs() -> Vec<UserInputEntry> {
    vec![
        UserInputEntry::Auto(AutoTracking { auto: true }),
        exclude_glob("!node_modules/.vite/task-cache/**", InputBase::Workspace),
        exclude_glob("!node_modules/.vite/task-cache/**", InputBase::Package),
    ]
}

fn merge_resolved_envs(
    envs: &Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>,
    resolved_envs: Vec<(String, String)>,
) -> Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>> {
    let mut envs = FxHashMap::clone(envs);
    for (k, v) in resolved_envs {
        envs.entry(Arc::from(OsStr::new(&k))).or_insert_with(|| Arc::from(OsStr::new(&v)));
    }
    Arc::new(envs)
}

/// Merge resolved envs and inject VP_VERSION for rolldown-vite branding.
fn merge_resolved_envs_with_version(
    envs: &Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>,
    resolved_envs: Vec<(String, String)>,
) -> Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>> {
    let mut merged = merge_resolved_envs(envs, resolved_envs);
    let map = Arc::make_mut(&mut merged);
    map.entry(Arc::from(OsStr::new("VP_VERSION")))
        .or_insert_with(|| Arc::from(OsStr::new(env!("CARGO_PKG_VERSION"))));
    merged
}

/// True when the user already chose an oxlint output mode we must not override.
///
/// `--quiet` is intentionally *not* an override: it only suppresses warning
/// diagnostics and still benefits from the default human reporter summary.
fn lint_has_output_mode_override(args: &[String]) -> bool {
    let mut expect_format_value = false;
    for arg in args {
        if expect_format_value {
            return true;
        }
        if arg == "--" {
            break;
        }
        if arg == "--silent" {
            return true;
        }
        if arg == "--format" || arg == "-f" {
            expect_format_value = true;
            continue;
        }
        if arg.starts_with("--format=") || arg.starts_with("-f=") {
            return true;
        }
    }
    // Trailing bare `--format` without a value still counts as an explicit override
    // so we do not inject a competing `--format=default`.
    expect_format_value
}

/// Inject `--format=default` unless the user already set a format/silent mode.
///
/// Placement: after an auto/user `-c <path>` pair when present and complete,
/// otherwise at the front so incomplete `vp lint -c` never panics on insert.
fn inject_lint_default_format(args: &mut Vec<String>) {
    if lint_has_output_mode_override(args) {
        return;
    }
    let insert_at = if args.len() >= 2 && args.first().is_some_and(|a| a == "-c") { 2 } else { 0 };
    args.insert(insert_at, "--format=default".to_string());
}

#[cfg(test)]
mod tests {
    use super::{inject_lint_default_format, lint_has_output_mode_override};

    #[test]
    fn lint_output_override_detects_format_and_silent_flags() {
        assert!(!lint_has_output_mode_override(&[]));
        assert!(!lint_has_output_mode_override(&["src".into(), ".".into()]));
        assert!(lint_has_output_mode_override(&["--format=json".into()]));
        assert!(lint_has_output_mode_override(&["--format".into(), "unix".into()]));
        assert!(lint_has_output_mode_override(&["-f".into(), "json".into()]));
        assert!(lint_has_output_mode_override(&["--silent".into()]));
        // `--quiet` only suppresses warnings; still inject default format.
        assert!(!lint_has_output_mode_override(&["--quiet".into()]));
        // Value after `--` is a path, not a format override.
        assert!(!lint_has_output_mode_override(&["--".into(), "--format=json".into()]));
    }

    #[test]
    fn inject_default_format_after_complete_config_pair() {
        let mut args = vec!["-c".into(), "vite.config.ts".into(), "src".into()];
        inject_lint_default_format(&mut args);
        assert_eq!(args, vec!["-c", "vite.config.ts", "--format=default", "src"]);
    }

    #[test]
    fn inject_default_format_at_front_when_no_config() {
        let mut args = vec!["src".into()];
        inject_lint_default_format(&mut args);
        assert_eq!(args, vec!["--format=default", "src"]);
    }

    #[test]
    fn inject_default_format_does_not_panic_on_incomplete_c_flag() {
        let mut args = vec!["-c".into()];
        inject_lint_default_format(&mut args);
        assert_eq!(args, vec!["--format=default", "-c"]);
    }

    #[test]
    fn inject_default_format_skips_when_format_already_set() {
        let mut args = vec!["--format=json".into(), "src".into()];
        inject_lint_default_format(&mut args);
        assert_eq!(args, vec!["--format=json", "src"]);
    }

    #[test]
    fn inject_default_format_alongside_quiet() {
        let mut args = vec!["--quiet".into()];
        inject_lint_default_format(&mut args);
        assert_eq!(args, vec!["--format=default", "--quiet"]);
    }
}
