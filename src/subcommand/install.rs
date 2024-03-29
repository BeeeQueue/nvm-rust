use std::{path::Path, time::Duration};

use anyhow::{Context, Result};
use clap::Parser;
use node_semver::Range;
use ureq;

use crate::{
    archives, constants, files,
    node_version::{
        filter_version_req, parse_range, InstalledNodeVersion, NodeVersion, OnlineNodeVersion,
    },
    subcommand::{switch::SwitchCommand, Action},
    Config,
};

#[derive(Parser, Clone, Debug)]
#[command(about = "Install a new node version", alias = "i", alias = "add")]
pub struct InstallCommand {
    /// A semver range. The latest version matching this range will be installed
    #[arg(value_parser = parse_range)]
    pub version: Option<Range>,
    /// Switch to the new version after installing it
    #[arg(long, short, default_value("false"))]
    pub switch: bool,
    /// Enable corepack after installing the new version
    #[arg(long, default_value("true"), hide(true), env("NVM_ENABLE_COREPACK"))]
    pub enable_corepack: bool,
}

impl Action<InstallCommand> for InstallCommand {
    fn run(config: &Config, options: &InstallCommand) -> Result<()> {
        let version_filter = options
            .version
            .clone()
            .or_else(|| files::get_version_file().map(|version_file| version_file.range()));

        if version_filter.is_none() {
            anyhow::bail!("You did not pass a version and we did not find any version files (package.json#engines, .nvmrc) in the current directory.");
        }
        let version_filter = version_filter.unwrap();

        let online_versions = OnlineNodeVersion::fetch_all()?;
        let filtered_versions = filter_version_req(online_versions, &version_filter);

        let version_to_install = filtered_versions.first().context(format!(
            "Did not find a version matching `{}`!",
            &version_filter
        ))?;

        if !config.force && InstalledNodeVersion::is_installed(config, version_to_install.version())
        {
            println!(
                "{} is already installed - skipping...",
                version_to_install.version()
            );

            return Ok(());
        }

        let install_path = version_to_install.install_path(config);
        download_and_extract_to(version_to_install, &install_path)?;

        if config.force
            || (options.switch
                && dialoguer::Confirm::new()
                    .with_prompt(format!("Switch to {}?", version_to_install.to_string()))
                    .default(true)
                    .interact()?)
        {
            SwitchCommand::run(
                &config.with_force(),
                &SwitchCommand {
                    version: Some(Range::parse(version_to_install.to_string())?),
                },
            )?;
        }

        if options.enable_corepack {
            if let Err(e) = std::process::Command::new(
                install_path
                    .join("bin")
                    .join(format!("corepack{}", constants::EXEC_EXT)),
            )
            .arg("enable")
            .output()
            {
                println!("⚠️ Failed to automatically enable corepack!\n{e}",)
            }
        }

        Ok(())
    }
}

fn download_and_extract_to(version: &OnlineNodeVersion, path: &Path) -> Result<()> {
    let url = version.download_url();
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(30))
        .timeout_read(Duration::from_secs(120))
        .timeout_write(Duration::from_secs(120))
        .build();

    println!("Downloading from {url}...");
    let response = agent
        .get(&url)
        .call()
        .context(format!("Failed to download version: {}", version.version()))?;

    let length: usize = response.header("Content-Length").unwrap().parse()?;
    let mut bytes: Vec<u8> = Vec::with_capacity(length);
    response.into_reader().read_to_end(&mut bytes)?;

    archives::extract_archive(bytes, path)
}
