use dioxus_cli_config::CrateConfig;

use crate::{
    cfg::{ConfigOptsBuild, ConfigOptsServe},
    Result,
};

use super::{desktop, Platform};

pub async fn startup(config: CrateConfig, serve: &ConfigOptsServe) -> Result<()> {
    desktop::startup_with_platform::<FullstackPlatform>(config, serve).await
}

fn start_web_build_thread(
    config: &CrateConfig,
    serve: &ConfigOptsServe,
) -> std::thread::JoinHandle<Result<()>> {
    let serve = serve.clone();
    let target_directory = config.client_target_dir();
    std::fs::create_dir_all(&target_directory).unwrap();
    std::thread::spawn(move || build_web(serve, &target_directory))
}

fn make_desktop_config(config: &CrateConfig, serve: &ConfigOptsServe) -> CrateConfig {
    let mut desktop_config = config.clone();
    desktop_config.target_dir = config.server_target_dir();
    let desktop_feature = serve.server_feature.clone();
    let features = &mut desktop_config.features;
    match features {
        Some(features) => {
            features.push(desktop_feature);
        }
        None => desktop_config.features = Some(vec![desktop_feature]),
    };
    desktop_config
}

struct FullstackPlatform {
    serve: ConfigOptsServe,
    desktop: desktop::DesktopPlatform,
}

impl Platform for FullstackPlatform {
    fn start(config: &CrateConfig, serve: &ConfigOptsServe) -> Result<Self>
    where
        Self: Sized,
    {
        dbg!("impl Platform for FullstackPlatform -> start()");
        let thread_handle = start_web_build_thread(config, serve);

        let desktop_config = make_desktop_config(config, serve);
        let desktop = {
            // let _guard = FullstackServerEnvGuard::new(&serve.clone().into());
            desktop::DesktopPlatform::start(&desktop_config, serve)?
        };
        thread_handle
            .join()
            .map_err(|_| anyhow::anyhow!("Failed to join thread"))??;

        Ok(Self {
            desktop,
            serve: serve.clone(),
        })
    }

    fn rebuild(&mut self, crate_config: &CrateConfig) -> Result<crate::BuildResult> {
        dbg!("impl Platform for FullstackPlatform -> rebuild()");
        let thread_handle = start_web_build_thread(crate_config, &self.serve);
        let result = {
            let desktop_config = make_desktop_config(crate_config, &self.serve);
            // let _guard = FullstackServerEnvGuard::new(&self.serve.clone().into());
            self.desktop.rebuild(&desktop_config)
        };
        thread_handle
            .join()
            .map_err(|_| anyhow::anyhow!("Failed to join thread"))??;
        result
    }
}

fn build_web(serve: ConfigOptsServe, target_directory: &std::path::Path) -> Result<()> {
    let mut web_config: ConfigOptsBuild = serve.into();
    let web_feature = web_config.client_feature.clone();
    let features = &mut web_config.features;
    match features {
        Some(features) => {
            features.push(web_feature);
        }
        None => web_config.features = Some(vec![web_feature]),
    };
    web_config.platform = Some(dioxus_cli_config::Platform::Web);

    // let _guard = FullstackWebEnvGuard::new(&web_config);
    crate::cli::build::Build { build: web_config }.build(None, Some(target_directory))
}

// Debug mode web builds have a very large size by default. If debug mode is not enabled, we strip some of the debug info by default
// This reduces a hello world from ~40MB to ~2MB
pub(crate) struct FullstackWebEnvGuard {
    old_rustflags: Option<String>,
}

impl FullstackWebEnvGuard {
    // serve should be for ConfigOptsServe
    pub fn new(build: &ConfigOptsBuild) -> Self {
        Self {
            old_rustflags: (!build.force_debug).then(|| {
                let old_rustflags = std::env::var("RUSTFLAGS").unwrap_or_default();
                let debug_assertions = if build.release {
                    ""
                } else {
                    " -C debug-assertions"
                };

                std::env::set_var(
                    "RUSTFLAGS",
                    format!(
                        "{old_rustflags} -C debuginfo=none -C strip=debuginfo{debug_assertions}"
                    ),
                );
                old_rustflags
            }),
        }
    }
}

impl Drop for FullstackWebEnvGuard {
    fn drop(&mut self) {
        if let Some(old_rustflags) = self.old_rustflags.take() {
            std::env::set_var("RUSTFLAGS", old_rustflags);
        }
    }
}

// Debug mode web builds have a very large size by default. If debug mode is not enabled, we strip some of the debug info by default
// This reduces a hello world from ~40MB to ~2MB
pub(crate) struct FullstackServerEnvGuard {
    old_rustflags: Option<String>,
}

impl FullstackServerEnvGuard {
    // serve should be for ConfigOptsServe
    pub fn new(build: &ConfigOptsBuild) -> Self {
        Self {
            old_rustflags: (!build.force_debug).then(|| {
                let old_rustflags = std::env::var("RUSTFLAGS").unwrap_or_default();
                let debug_assertions = if build.release {
                    ""
                } else {
                    " -C debug-assertions"
                };

                std::env::set_var(
                    "RUSTFLAGS",
                    format!("{old_rustflags} -C opt-level=2 {debug_assertions}"),
                );
                old_rustflags
            }),
        }
    }
}

impl Drop for FullstackServerEnvGuard {
    fn drop(&mut self) {
        if let Some(old_rustflags) = self.old_rustflags.take() {
            std::env::set_var("RUSTFLAGS", old_rustflags);
        }
    }
}
