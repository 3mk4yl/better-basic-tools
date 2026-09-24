use bbt_core::OutputFormat;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "bbt",
    about = "Better Basic Tools: structured Linux facts for humans and agents"
)]
pub struct Cli {
    #[arg(long, global = true, conflicts_with_all = ["json", "yaml", "agent"])]
    pub format: Option<Format>,

    #[arg(long, global = true, conflicts_with_all = ["yaml", "agent", "format"])]
    pub json: bool,

    #[arg(long, global = true, conflicts_with_all = ["json", "agent", "format"])]
    pub yaml: bool,

    #[arg(long, global = true, conflicts_with_all = ["json", "yaml", "format"])]
    pub agent: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Format {
    Human,
    Json,
    Yaml,
    Agent,
}

impl From<Format> for OutputFormat {
    fn from(format: Format) -> Self {
        match format {
            Format::Human => OutputFormat::Human,
            Format::Json => OutputFormat::Json,
            Format::Yaml => OutputFormat::Yaml,
            Format::Agent => OutputFormat::Agent,
        }
    }
}

impl Cli {
    pub fn output_format(&self) -> OutputFormat {
        if self.json {
            OutputFormat::Json
        } else if self.yaml {
            OutputFormat::Yaml
        } else if self.agent {
            OutputFormat::Agent
        } else {
            self.format.map(Into::into).unwrap_or(OutputFormat::Human)
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print bbt version information.
    Version,
    /// Host summary, health, and snapshots.
    Host {
        #[command(subcommand)]
        command: HostCommand,
    },
    /// Filesystem paths, metadata, permissions, and traversal.
    Fs {
        #[command(subcommand)]
        command: FsCommand,
    },
    /// Filesystems, mounts, usage, and disk pressure.
    Disk {
        #[command(subcommand)]
        command: DiskCommand,
    },
    /// Process inspection, trees, and pressure.
    Proc {
        #[command(subcommand)]
        command: ProcCommand,
    },
    /// Network sockets and listener inventory.
    Net {
        #[command(subcommand)]
        command: NetCommand,
    },
    /// systemd service inspection.
    Service {
        #[command(subcommand)]
        command: ServiceCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum HostCommand {
    /// Capture a one-shot host situation report.
    Snapshot,
}

#[derive(Debug, Subcommand)]
pub enum FsCommand {
    /// Inspect one filesystem path and explain its metadata/access.
    Inspect { path: std::path::PathBuf },
    /// List the immediate entries of one directory (non-recursive).
    List {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
    },
    /// Show a directory tree with recursive per-subtree size totals.
    Tree {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
        /// Maximum child reporting depth. Totals are still calculated recursively; 0 reports only the root.
        #[arg(long)]
        depth: Option<u64>,
        /// Stay on the same filesystem device as the root path.
        #[arg(long)]
        one_filesystem: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum DiskCommand {
    /// Calculate disk usage for one path.
    Usage {
        path: std::path::PathBuf,
        /// Child reporting depth: 0 suppresses child rows, 1 reports immediate children. Totals are always full recursive sums.
        #[arg(long, value_parser = parse_usage_depth)]
        depth: Option<u64>,
        /// Stay on the same filesystem device as the root path.
        #[arg(long)]
        one_filesystem: bool,
        /// Number of largest immediate children to include.
        #[arg(long, default_value_t = 10)]
        top: usize,
    },
    /// List all mounted filesystems with capacity and usage.
    Filesystems,
    /// Report kernel I/O pressure stall information (PSI).
    Pressure,
}

fn parse_usage_depth(value: &str) -> Result<u64, String> {
    let depth: u64 = value
        .parse()
        .map_err(|_| format!("`{value}` is not a non-negative integer"))?;
    if depth > 1 {
        return Err(format!(
            "disk usage reports at most one child level: use --depth 0 for totals only or \
             --depth 1 for immediate children, or `bbt fs tree PATH --depth {depth}` for a \
             deeper per-subtree report"
        ));
    }
    Ok(depth)
}

#[derive(Debug, Subcommand)]
pub enum ProcCommand {
    /// Inspect one process by PID.
    Inspect { pid: u32 },
    /// List visible local processes.
    List,
}

#[derive(Debug, Subcommand)]
pub enum NetCommand {
    /// List local listening sockets.
    Listeners,
}

#[derive(Debug, Subcommand)]
pub enum ServiceCommand {
    /// Inspect one systemd service/unit.
    Inspect { name: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbt_core::OutputFormat;
    use clap::Parser;

    fn parse(args: &[&str]) -> Cli {
        Cli::parse_from(args)
    }

    #[test]
    fn defaults_to_human_output() {
        let cli = parse(&["bbt", "disk", "usage", "Cargo.toml"]);

        assert_eq!(cli.output_format(), OutputFormat::Human);
    }

    #[test]
    fn resolves_json_shortcut() {
        let cli = parse(&["bbt", "--json", "fs", "inspect", "Cargo.toml"]);

        assert_eq!(cli.output_format(), OutputFormat::Json);
    }

    #[test]
    fn resolves_yaml_shortcut() {
        let cli = parse(&["bbt", "--yaml", "fs", "inspect", "Cargo.toml"]);

        assert_eq!(cli.output_format(), OutputFormat::Yaml);
    }

    #[test]
    fn resolves_agent_shortcut() {
        let cli = parse(&["bbt", "--agent", "fs", "inspect", "Cargo.toml"]);

        assert_eq!(cli.output_format(), OutputFormat::Agent);
    }

    #[test]
    fn resolves_explicit_format() {
        let cli = parse(&["bbt", "--format", "agent", "fs", "inspect", "Cargo.toml"]);

        assert_eq!(cli.output_format(), OutputFormat::Agent);
    }

    #[test]
    fn rejects_conflicting_output_flags() {
        let result =
            Cli::try_parse_from(["bbt", "--json", "--yaml", "fs", "inspect", "Cargo.toml"]);

        assert!(result.is_err());
    }

    #[test]
    fn parses_disk_filesystems_subcommand() {
        let cli = parse(&["bbt", "disk", "filesystems"]);

        assert!(matches!(
            cli.command,
            Command::Disk {
                command: DiskCommand::Filesystems
            }
        ));
    }

    #[test]
    fn disk_filesystems_takes_no_path_argument() {
        let result = Cli::try_parse_from(["bbt", "disk", "filesystems", "/"]);

        assert!(result.is_err());
    }

    #[test]
    fn parses_disk_pressure_subcommand() {
        let cli = parse(&["bbt", "disk", "pressure"]);

        assert!(matches!(
            cli.command,
            Command::Disk {
                command: DiskCommand::Pressure
            }
        ));
    }

    #[test]
    fn disk_pressure_takes_no_path_argument() {
        let result = Cli::try_parse_from(["bbt", "disk", "pressure", "/"]);

        assert!(result.is_err());
    }

    #[test]
    fn parses_fs_list_subcommand_with_explicit_path() {
        let cli = parse(&["bbt", "fs", "list", "/tmp"]);

        assert!(matches!(
            cli.command,
            Command::Fs {
                command: FsCommand::List { path }
            } if path == std::path::Path::new("/tmp")
        ));
    }

    #[test]
    fn fs_list_path_defaults_to_current_directory() {
        let cli = parse(&["bbt", "fs", "list"]);

        assert!(matches!(
            cli.command,
            Command::Fs {
                command: FsCommand::List { path }
            } if path == std::path::Path::new(".")
        ));
    }

    #[test]
    fn parses_fs_tree_subcommand_with_flags() {
        let cli = parse(&[
            "bbt",
            "fs",
            "tree",
            "/tmp",
            "--depth",
            "2",
            "--one-filesystem",
        ]);

        assert!(matches!(
            cli.command,
            Command::Fs {
                command: FsCommand::Tree {
                    path,
                    depth: Some(2),
                    one_filesystem: true,
                }
            } if path == std::path::Path::new("/tmp")
        ));
    }

    #[test]
    fn fs_tree_path_defaults_to_current_directory() {
        let cli = parse(&["bbt", "fs", "tree"]);

        assert!(matches!(
            cli.command,
            Command::Fs {
                command: FsCommand::Tree {
                    path,
                    depth: None,
                    one_filesystem: false,
                }
            } if path == std::path::Path::new(".")
        ));
    }

    #[test]
    fn disk_usage_accepts_depth_zero_and_one() {
        for depth in ["0", "1"] {
            let result = Cli::try_parse_from(["bbt", "disk", "usage", ".", "--depth", depth]);

            assert!(result.is_ok(), "--depth {depth} must be accepted");
        }
    }

    #[test]
    fn disk_usage_rejects_depth_greater_than_one_with_actionable_message() {
        let error = Cli::try_parse_from(["bbt", "disk", "usage", ".", "--depth", "2"])
            .expect_err("--depth 2 promises reporting the command does not implement");

        let rendered = error.to_string();
        assert!(rendered.contains("--depth 0"), "message: {rendered}");
        assert!(rendered.contains("--depth 1"), "message: {rendered}");
        assert!(rendered.contains("fs tree"), "message: {rendered}");
    }

    #[test]
    fn parses_version_subcommand() {
        let cli = parse(&["bbt", "version"]);

        assert!(matches!(cli.command, Command::Version));
    }
}
