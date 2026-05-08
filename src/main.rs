/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0
 */

use anyhow::{anyhow, bail, Context, Result};
use cargo_check_external_types_detail::catalog::ItemCatalog;
use cargo_check_external_types_detail::cargo::CargoRustDocJson;
use cargo_check_external_types_detail::config::Config;
use cargo_check_external_types_detail::error::{ErrorPrinter, ValidationErrors};
use cargo_check_external_types_detail::here;
use cargo_check_external_types_detail::visitor::Visitor;
use cargo_metadata::{CargoOpt, Metadata, Package, TargetKind};
use clap::{Parser, ValueEnum};
use rustdoc_types::Crate;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::process;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum OutputFormat {
    #[default]
    Errors,
    MarkdownTable,
    Json,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Errors => write!(f, "errors"),
            Self::MarkdownTable => write!(f, "markdown-table"),
            Self::Json => write!(f, "json"),
        }
    }
}

/// Shared Cargo / rustdoc options for subcommands that analyze the crate API.
#[derive(clap::Args, Debug, Eq, PartialEq)]
struct CargoRustdocArgs {
    /// Enables all crate features
    #[arg(long, conflicts_with = "no_default_features")]
    all_features: bool,
    /// Disables default features
    #[arg(long, conflicts_with = "all_features")]
    no_default_features: bool,
    /// Comma delimited list of features to enable in the crate
    #[arg(long, value_delimiter = ',')]
    features: Option<Vec<String>>,
    /// Path to the Cargo manifest
    #[arg(long)]
    manifest_path: Option<PathBuf>,
    /// Target triple
    #[arg(long)]
    target: Option<String>,

    /// Path to config toml to read
    #[arg(long)]
    config: Option<PathBuf>,
    /// Enable verbose output for debugging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(clap::Args, Debug, Eq, PartialEq)]
struct CheckExternalTypesDetailArgs {
    #[command(flatten)]
    cargo: CargoRustdocArgs,
    /// Format to output results in
    #[arg(long, default_value_t = OutputFormat::Errors)]
    output_format: OutputFormat,
}

#[derive(clap::Args, Debug, Eq, PartialEq)]
struct ListPublicItemsDetailArgs {
    #[command(flatten)]
    cargo: CargoRustdocArgs,
}

#[derive(Parser, Debug, Eq, PartialEq)]
#[command(author, version, about, bin_name = "cargo")]
enum Args {
    CheckExternalTypesDetail(CheckExternalTypesDetailArgs),
    ListPublicItemsDetail(ListPublicItemsDetailArgs),
}

enum Error {
    ValidationErrors,
    Failure(anyhow::Error),
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Error::Failure(err)
    }
}

fn main() {
    process::exit(match run_main() {
        Ok(_) => 0,
        Err(Error::ValidationErrors) => 1,
        Err(Error::Failure(err)) => {
            println!("{:#}", dbg!(err));
            2
        }
    })
}

fn init_tracing_if_verbose(verbose: bool) {
    if !verbose {
        return;
    }
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("debug"))
        .unwrap();
    let fmt_layer = tracing_subscriber::fmt::layer()
        .without_time()
        .with_ansi(true)
        .with_level(true)
        .with_target(false)
        .pretty();
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();
}

fn run_rustdoc_package(cargo: &CargoRustdocArgs) -> Result<(Metadata, Config, Crate)> {
    init_tracing_if_verbose(cargo.verbose);

    let mut cargo_metadata_cmd = cargo_metadata::MetadataCommand::new();
    if cargo.all_features {
        cargo_metadata_cmd.features(CargoOpt::AllFeatures);
    }
    if cargo.no_default_features {
        cargo_metadata_cmd.features(CargoOpt::NoDefaultFeatures);
    }
    if let Some(features) = &cargo.features {
        cargo_metadata_cmd.features(CargoOpt::SomeFeatures(features.clone()));
    }
    let crate_path = if let Some(manifest_path) = &cargo.manifest_path {
        cargo_metadata_cmd.manifest_path(manifest_path);
        manifest_path
            .canonicalize()
            .context(here!())?
            .parent()
            .expect("parent path")
            .to_path_buf()
    } else {
        std::env::current_dir()
            .context(here!())?
            .canonicalize()
            .context(here!())?
    };
    let cargo_metadata = cargo_metadata_cmd.exec().context(here!())?;

    let config = if let Some(config_path) = &cargo.config {
        let contents = fs::read_to_string(config_path).context("failed to read config file")?;
        toml::from_str(&contents).context("failed to parse config file")?
    } else {
        resolve_config(&cargo_metadata)
            .context("failed to parse config from Cargo.toml metadata")?
    };

    let cargo_features = if let Some(features) = cargo.features.clone() {
        features
    } else {
        resolve_features(&cargo_metadata)?
    };
    let cargo_lib_name = resolve_lib_name(&cargo_metadata)?;

    eprintln!("Running rustdoc to produce json doc output...");
    let package = CargoRustDocJson::new(
        cargo_lib_name,
        crate_path,
        &cargo_metadata.target_directory,
        cargo_features,
        cargo.target.clone(),
    )
    .run()
    .context(here!())?;

    Ok((cargo_metadata, config, package))
}

fn run_check(args: CheckExternalTypesDetailArgs) -> Result<(), Error> {
    let (cargo_metadata, config, package) = run_rustdoc_package(&args.cargo)?;

    eprintln!("Examining all public types...");
    let errors = Visitor::new(config, package)?.visit_all()?;
    match args.output_format {
        OutputFormat::Errors => {
            ErrorPrinter::new(&cargo_metadata.workspace_root).pretty_print_errors(&errors);
            if errors.error_count() > 0 {
                return Err(Error::ValidationErrors);
            }
        }
        OutputFormat::MarkdownTable => print_markdown_table(&errors),
        OutputFormat::Json => {
            let value = errors.to_json_value();
            let rendered = serde_json::to_string_pretty(&value)
                .context("failed to serialize diagnostics as JSON")?;
            println!("{rendered}");
        }
    }

    Ok(())
}

fn run_list_public_items_detail(args: ListPublicItemsDetailArgs) -> Result<(), Error> {
    let (_cargo_metadata, config, package) = run_rustdoc_package(&args.cargo)?;

    eprintln!("Cataloging public items and external usage...");
    let (_errors, catalog) = Visitor::new_with_catalog(config, package)?
        .visit_all_with_catalog()
        .map_err(Error::from)?;
    print_catalog_json(&catalog).map_err(Error::from)?;

    Ok(())
}

fn print_catalog_json(catalog: &ItemCatalog) -> Result<()> {
    let value = catalog.to_json_value();
    let rendered =
        serde_json::to_string_pretty(&value).context("failed to serialize catalog as JSON")?;
    println!("{rendered}");
    Ok(())
}

fn run_main() -> Result<(), Error> {
    match Args::parse() {
        Args::CheckExternalTypesDetail(args) => run_check(args),
        Args::ListPublicItemsDetail(args) => run_list_public_items_detail(args),
    }
}

fn print_markdown_table(errors: &ValidationErrors) {
    println!(
        "| External crate | External type | Exposure kind | API path | Local struct chain | Role | Source |"
    );
    println!("| --- | --- | --- | --- | --- | --- | --- |");
    let mut rows: Vec<String> = errors
        .iter()
        .filter_map(|e| e.markdown_table_row().map(|r| r.format_line()))
        .collect();
    rows.sort();
    for row in rows {
        println!("{row}");
    }
}

fn resolve_config(metadata: &Metadata) -> Result<Config> {
    let crate_metadata = match serde_json::from_value::<HashMap<String, serde_json::Value>>(
        resolve_root_package(metadata)?.metadata.clone(),
    ) {
        Ok(m) => m,
        // We avoid using ? on the serde_json::from_value because when the metadata is not provided
        // this will err trying to unmarshal a null value into a map. In this instance we want to
        // use the default config.
        Err(_) => return Ok(Default::default()),
    };

    Ok(
        if let Some(our_metadata) = crate_metadata.get(env!("CARGO_CRATE_NAME")) {
            // Here we do use ? to propagate the error from the unmarshal - it would indicate
            // the metadata config is present, but invalid.
            serde_json::from_value(our_metadata.clone())?
        } else {
            Default::default()
        },
    )
}

fn resolve_features(metadata: &Metadata) -> Result<Vec<String>> {
    let root_package = resolve_root_package(metadata)?;
    if let Some(resolve) = &metadata.resolve {
        let root_node = resolve
            .nodes
            .iter()
            .find(|&n| n.id == root_package.id)
            .ok_or_else(|| anyhow!("Failed to find node for root package"))?;
        Ok(root_node.features.clone())
    } else {
        bail!("Cargo metadata didn't have resolved nodes");
    }
}

fn resolve_lib_name(metadata: &Metadata) -> Result<String> {
    let lib_targets = resolve_root_package(metadata)?
        .targets
        .iter()
        .filter(|t| t.kind.contains(&TargetKind::Lib))
        .collect::<Vec<_>>();
    if lib_targets.len() != 1 {
        bail!(
            "Expected crate to define 1 lib target, found {}",
            lib_targets.len()
        );
    }
    Ok(lib_targets.first().unwrap().name.clone())
}

fn resolve_root_package(metadata: &Metadata) -> Result<&Package> {
    metadata
        .root_package()
        .ok_or_else(|| {
            let workspace_members = metadata.workspace_members.as_slice().iter().map(|id| id.to_string()).collect::<Vec<_>>().join("\n");
            if !workspace_members.is_empty() {
                anyhow!("it appears you're trying to run `cargo-check-external-types-detail` on a workspace Cargo.toml; Instead, run it on one of the workspace member Cargo.tomls directly:\n{workspace_members}")
            } else {
                anyhow!("No root package found")
            }
        })
}

#[cfg(test)]
mod cli_tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Args::command().debug_assert();
    }
}

#[cfg(test)]
mod arg_parse_tests {
    use super::*;
    use clap::Parser;

    fn default_cargo_args() -> CargoRustdocArgs {
        CargoRustdocArgs {
            all_features: false,
            no_default_features: false,
            features: None,
            manifest_path: None,
            target: None,
            config: None,
            verbose: false,
        }
    }

    #[test]
    fn defaults() {
        assert_eq!(
            Args::CheckExternalTypesDetail(CheckExternalTypesDetailArgs {
                cargo: default_cargo_args(),
                output_format: OutputFormat::Errors,
            }),
            Args::try_parse_from(["cargo", "check-external-types-detail"]).unwrap()
        );
    }

    #[test]
    fn list_public_items_detail_defaults() {
        assert_eq!(
            Args::ListPublicItemsDetail(ListPublicItemsDetailArgs {
                cargo: default_cargo_args(),
            }),
            Args::try_parse_from(["cargo", "list-public-items-detail"]).unwrap()
        );
    }

    #[test]
    fn all_features() {
        assert_eq!(
            Args::CheckExternalTypesDetail(CheckExternalTypesDetailArgs {
                cargo: CargoRustdocArgs {
                    all_features: true,
                    ..default_cargo_args()
                },
                output_format: OutputFormat::Errors,
            }),
            Args::try_parse_from(["cargo", "check-external-types-detail", "--all-features"])
                .unwrap()
        );
    }

    #[test]
    fn no_default_features() {
        assert_eq!(
            Args::CheckExternalTypesDetail(CheckExternalTypesDetailArgs {
                cargo: CargoRustdocArgs {
                    no_default_features: true,
                    ..default_cargo_args()
                },
                output_format: OutputFormat::Errors,
            }),
            Args::try_parse_from([
                "cargo",
                "check-external-types-detail",
                "--no-default-features"
            ])
            .unwrap()
        );
    }

    #[test]
    fn feature_list() {
        assert_eq!(
            Args::CheckExternalTypesDetail(CheckExternalTypesDetailArgs {
                cargo: CargoRustdocArgs {
                    features: Some(vec!["foo".into(), "bar".into()]),
                    ..default_cargo_args()
                },
                output_format: OutputFormat::Errors,
            }),
            Args::try_parse_from([
                "cargo",
                "check-external-types-detail",
                "--features",
                "foo,bar"
            ])
            .unwrap()
        );
    }

    #[test]
    fn manifest_path() {
        assert_eq!(
            Args::CheckExternalTypesDetail(CheckExternalTypesDetailArgs {
                cargo: CargoRustdocArgs {
                    manifest_path: Some("test-path".into()),
                    ..default_cargo_args()
                },
                output_format: OutputFormat::Errors,
            }),
            Args::try_parse_from([
                "cargo",
                "check-external-types-detail",
                "--manifest-path",
                "test-path"
            ])
            .unwrap()
        );
    }

    #[test]
    fn target() {
        assert_eq!(
            Args::CheckExternalTypesDetail(CheckExternalTypesDetailArgs {
                cargo: CargoRustdocArgs {
                    target: Some("x86_64-unknown-linux-gnu".into()),
                    ..default_cargo_args()
                },
                output_format: OutputFormat::Errors,
            }),
            Args::try_parse_from([
                "cargo",
                "check-external-types-detail",
                "--target",
                "x86_64-unknown-linux-gnu"
            ])
            .unwrap()
        );
    }

    #[test]
    fn verbose() {
        assert_eq!(
            Args::CheckExternalTypesDetail(CheckExternalTypesDetailArgs {
                cargo: CargoRustdocArgs {
                    verbose: true,
                    ..default_cargo_args()
                },
                output_format: OutputFormat::Errors,
            }),
            Args::try_parse_from(["cargo", "check-external-types-detail", "--verbose"]).unwrap()
        );
    }

    #[test]
    fn output_format_markdown_table() {
        assert_eq!(
            Args::CheckExternalTypesDetail(CheckExternalTypesDetailArgs {
                cargo: default_cargo_args(),
                output_format: OutputFormat::MarkdownTable,
            }),
            Args::try_parse_from([
                "cargo",
                "check-external-types-detail",
                "--output-format",
                "markdown-table"
            ])
            .unwrap()
        );
    }

    #[test]
    fn output_format_json() {
        assert_eq!(
            Args::CheckExternalTypesDetail(CheckExternalTypesDetailArgs {
                cargo: default_cargo_args(),
                output_format: OutputFormat::Json,
            }),
            Args::try_parse_from([
                "cargo",
                "check-external-types-detail",
                "--output-format",
                "json"
            ])
            .unwrap()
        );
    }

    #[test]
    fn output_format_invalid() {
        assert!(Args::try_parse_from([
            "cargo",
            "check-external-types-detail",
            "--output-format",
            "yaml"
        ])
        .is_err());
    }

    #[test]
    fn conflict_all_features_no_default_features() {
        // Check `--all-features` and `--no-default-features` conflict
        assert!(Args::try_parse_from([
            "cargo",
            "check-external-types-detail",
            "--all-features",
            "--no-default-features"
        ])
        .is_err());
    }
}
