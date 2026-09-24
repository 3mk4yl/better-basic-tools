use anyhow::Result;
use bbt_cli::{
    Cli, Command, DiskCommand, FsCommand, HostCommand, NetCommand, ProcCommand, ServiceCommand,
};
use bbt_core::{CommandContext, OutputFormat, SchemaEnvelope};
use bbt_disk::{DiskUsage, FilesystemReport, PressureInfo};
use bbt_fs::{FsInspection, FsListing, FsTree, TreeNode};
use bbt_host::HostSnapshot;
use bbt_net::ListenerReport;
use bbt_proc::{ProcInspection, ProcList};
use bbt_service::ServiceInspection;
use clap::Parser;
use std::os::unix::ffi::OsStrExt;

const FS_INSPECT_SCHEMA_V1: &str = "bbt.fs.inspect.v1";
const FS_LIST_SCHEMA_V1: &str = "bbt.fs.list.v1";
const FS_TREE_SCHEMA_V1: &str = "bbt.fs.tree.v1";
const DISK_USAGE_SCHEMA_V1: &str = "bbt.disk.usage.v1";
const DISK_FILESYSTEMS_SCHEMA_V1: &str = "bbt.disk.filesystems.v1";
const DISK_PRESSURE_SCHEMA_V1: &str = "bbt.disk.pressure.v1";
const HOST_SNAPSHOT_SCHEMA_V1: &str = "bbt.host.snapshot.v1";
const PROC_INSPECT_SCHEMA_V1: &str = "bbt.proc.inspect.v1";
const PROC_LIST_SCHEMA_V1: &str = "bbt.proc.list.v1";
const NET_LISTENERS_SCHEMA_V1: &str = "bbt.net.listeners.v1";
const SERVICE_INSPECT_SCHEMA_V1: &str = "bbt.service.inspect.v1";

fn main() -> Result<()> {
    let cli = Cli::parse();
    let output_format = cli.output_format();
    let command_context = command_context();

    match cli.command {
        Command::Version => {
            println!("bbt {}", env!("CARGO_PKG_VERSION"));
        }
        Command::Fs { command } => match command {
            FsCommand::Inspect { path } => {
                let inspection = bbt_fs::inspect_path(&path)?;
                render_fs_inspect(&inspection, output_format, command_context)?;
            }
            FsCommand::List { path } => {
                let listing = bbt_fs::list_directory(&path);
                render_fs_list(&listing, output_format, command_context)?;
            }
            FsCommand::Tree {
                path,
                depth,
                one_filesystem,
            } => {
                let tree = bbt_fs::tree_with_options(
                    &path,
                    bbt_fs::TreeOptions::new(depth, one_filesystem),
                );
                render_fs_tree(&tree, output_format, command_context)?;
            }
        },
        Command::Disk { command } => match command {
            DiskCommand::Usage {
                path,
                depth,
                one_filesystem,
                top,
            } => {
                let usage = bbt_disk::usage_with_options(
                    &path,
                    bbt_disk::UsageOptions::new(depth, one_filesystem, top)
                        .expect("the CLI validates disk usage depth"),
                );
                render_disk_usage(&usage, output_format, command_context)?;
            }
            DiskCommand::Filesystems => {
                let report = bbt_disk::filesystems();
                render_disk_filesystems(&report, output_format, command_context)?;
            }
            DiskCommand::Pressure => {
                let info = bbt_disk::pressure();
                render_disk_pressure(&info, output_format, command_context)?;
            }
        },
        Command::Host {
            command: HostCommand::Snapshot,
        } => {
            let snapshot = bbt_host::snapshot();
            render_host_snapshot(&snapshot, output_format, command_context)?;
        }
        Command::Proc { command } => match command {
            ProcCommand::Inspect { pid } => {
                let inspection = bbt_proc::inspect(pid);
                render_proc_inspect(&inspection, output_format, command_context)?;
            }
            ProcCommand::List => {
                let list = bbt_proc::list();
                render_proc_list(&list, output_format, command_context)?;
            }
        },
        Command::Net {
            command: NetCommand::Listeners,
        } => {
            let listeners = bbt_net::listeners();
            render_net_listeners(&listeners, output_format, command_context)?;
        }
        Command::Service {
            command: ServiceCommand::Inspect { name },
        } => {
            let inspection = bbt_service::inspect(&name);
            render_service_inspect(&inspection, output_format, command_context)?;
        }
    }

    Ok(())
}

fn render_fs_inspect(
    inspection: &FsInspection,
    format: OutputFormat,
    command_context: CommandContext,
) -> Result<()> {
    match format {
        OutputFormat::Human => print_human_fs_inspect(inspection),
        OutputFormat::Json => {
            let envelope =
                SchemaEnvelope::new(FS_INSPECT_SCHEMA_V1, command_context.clone(), inspection);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        }
        OutputFormat::Yaml => {
            let envelope =
                SchemaEnvelope::new(FS_INSPECT_SCHEMA_V1, command_context.clone(), inspection);
            println!("{}", serde_norway::to_string(&envelope)?);
        }
        OutputFormat::Agent => {
            let report = bbt_fs::agent_report(inspection, bbt_core::current_timestamp());
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }

    Ok(())
}

fn render_fs_list(
    listing: &FsListing,
    format: OutputFormat,
    command_context: CommandContext,
) -> Result<()> {
    match format {
        OutputFormat::Human => print_human_fs_list(listing),
        OutputFormat::Json => {
            let envelope = SchemaEnvelope::new(FS_LIST_SCHEMA_V1, command_context.clone(), listing);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        }
        OutputFormat::Yaml => {
            let envelope = SchemaEnvelope::new(FS_LIST_SCHEMA_V1, command_context.clone(), listing);
            println!("{}", serde_norway::to_string(&envelope)?);
        }
        OutputFormat::Agent => {
            let report = bbt_fs::list_agent_report(listing, bbt_core::current_timestamp());
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }

    Ok(())
}

fn render_fs_tree(
    tree: &FsTree,
    format: OutputFormat,
    command_context: CommandContext,
) -> Result<()> {
    match format {
        OutputFormat::Human => print_human_fs_tree(tree),
        OutputFormat::Json => {
            let envelope = SchemaEnvelope::new(FS_TREE_SCHEMA_V1, command_context.clone(), tree);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        }
        OutputFormat::Yaml => {
            let envelope = SchemaEnvelope::new(FS_TREE_SCHEMA_V1, command_context.clone(), tree);
            println!("{}", serde_norway::to_string(&envelope)?);
        }
        OutputFormat::Agent => {
            let report = bbt_fs::tree_agent_report(tree, bbt_core::current_timestamp());
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }

    Ok(())
}

fn render_disk_usage(
    usage: &DiskUsage,
    format: OutputFormat,
    command_context: CommandContext,
) -> Result<()> {
    match format {
        OutputFormat::Human => print_human_disk_usage(usage),
        OutputFormat::Json => {
            let envelope =
                SchemaEnvelope::new(DISK_USAGE_SCHEMA_V1, command_context.clone(), usage);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        }
        OutputFormat::Yaml => {
            let envelope =
                SchemaEnvelope::new(DISK_USAGE_SCHEMA_V1, command_context.clone(), usage);
            println!("{}", serde_norway::to_string(&envelope)?);
        }
        OutputFormat::Agent => {
            let report = bbt_disk::agent_report(usage, bbt_core::current_timestamp());
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }

    Ok(())
}

fn render_disk_filesystems(
    report: &FilesystemReport,
    format: OutputFormat,
    command_context: CommandContext,
) -> Result<()> {
    match format {
        OutputFormat::Human => print_human_disk_filesystems(report),
        OutputFormat::Json => {
            let envelope =
                SchemaEnvelope::new(DISK_FILESYSTEMS_SCHEMA_V1, command_context.clone(), report);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        }
        OutputFormat::Yaml => {
            let envelope =
                SchemaEnvelope::new(DISK_FILESYSTEMS_SCHEMA_V1, command_context.clone(), report);
            println!("{}", serde_norway::to_string(&envelope)?);
        }
        OutputFormat::Agent => {
            let report = bbt_disk::filesystems_agent_report(report, bbt_core::current_timestamp());
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }

    Ok(())
}

fn render_disk_pressure(
    info: &PressureInfo,
    format: OutputFormat,
    command_context: CommandContext,
) -> Result<()> {
    match format {
        OutputFormat::Human => print_human_disk_pressure(info),
        OutputFormat::Json => {
            let envelope =
                SchemaEnvelope::new(DISK_PRESSURE_SCHEMA_V1, command_context.clone(), info);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        }
        OutputFormat::Yaml => {
            let envelope =
                SchemaEnvelope::new(DISK_PRESSURE_SCHEMA_V1, command_context.clone(), info);
            println!("{}", serde_norway::to_string(&envelope)?);
        }
        OutputFormat::Agent => {
            let report = bbt_disk::pressure_agent_report(info, bbt_core::current_timestamp());
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }

    Ok(())
}

fn render_host_snapshot(
    snapshot: &HostSnapshot,
    format: OutputFormat,
    command_context: CommandContext,
) -> Result<()> {
    match format {
        OutputFormat::Human => print_human_host_snapshot(snapshot),
        OutputFormat::Json => {
            let envelope =
                SchemaEnvelope::new(HOST_SNAPSHOT_SCHEMA_V1, command_context.clone(), snapshot);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        }
        OutputFormat::Yaml => {
            let envelope =
                SchemaEnvelope::new(HOST_SNAPSHOT_SCHEMA_V1, command_context.clone(), snapshot);
            println!("{}", serde_norway::to_string(&envelope)?);
        }
        OutputFormat::Agent => {
            let report = bbt_host::agent_report(snapshot, bbt_core::current_timestamp());
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }

    Ok(())
}

fn render_proc_inspect(
    inspection: &ProcInspection,
    format: OutputFormat,
    command_context: CommandContext,
) -> Result<()> {
    match format {
        OutputFormat::Human => print_human_proc_inspect(inspection),
        OutputFormat::Json => {
            let envelope =
                SchemaEnvelope::new(PROC_INSPECT_SCHEMA_V1, command_context.clone(), inspection);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        }
        OutputFormat::Yaml => {
            let envelope =
                SchemaEnvelope::new(PROC_INSPECT_SCHEMA_V1, command_context.clone(), inspection);
            println!("{}", serde_norway::to_string(&envelope)?);
        }
        OutputFormat::Agent => {
            let report = bbt_proc::agent_report(inspection, bbt_core::current_timestamp());
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }

    Ok(())
}

fn render_proc_list(
    list: &ProcList,
    format: OutputFormat,
    command_context: CommandContext,
) -> Result<()> {
    match format {
        OutputFormat::Human => print_human_proc_list(list),
        OutputFormat::Json => {
            let envelope = SchemaEnvelope::new(PROC_LIST_SCHEMA_V1, command_context.clone(), list);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        }
        OutputFormat::Yaml => {
            let envelope = SchemaEnvelope::new(PROC_LIST_SCHEMA_V1, command_context.clone(), list);
            println!("{}", serde_norway::to_string(&envelope)?);
        }
        OutputFormat::Agent => {
            let report = bbt_proc::list_agent_report(list, bbt_core::current_timestamp());
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }
    Ok(())
}

fn render_net_listeners(
    listeners: &ListenerReport,
    format: OutputFormat,
    command_context: CommandContext,
) -> Result<()> {
    match format {
        OutputFormat::Human => print_human_net_listeners(listeners),
        OutputFormat::Json => {
            let envelope =
                SchemaEnvelope::new(NET_LISTENERS_SCHEMA_V1, command_context.clone(), listeners);
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        }
        OutputFormat::Yaml => {
            let envelope =
                SchemaEnvelope::new(NET_LISTENERS_SCHEMA_V1, command_context.clone(), listeners);
            println!("{}", serde_norway::to_string(&envelope)?);
        }
        OutputFormat::Agent => {
            let report = bbt_net::agent_report(listeners, bbt_core::current_timestamp());
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }
    Ok(())
}

fn render_service_inspect(
    inspection: &ServiceInspection,
    format: OutputFormat,
    command_context: CommandContext,
) -> Result<()> {
    match format {
        OutputFormat::Human => print_human_service_inspect(inspection),
        OutputFormat::Json => {
            let envelope = SchemaEnvelope::new(
                SERVICE_INSPECT_SCHEMA_V1,
                command_context.clone(),
                inspection,
            );
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        }
        OutputFormat::Yaml => {
            let envelope = SchemaEnvelope::new(
                SERVICE_INSPECT_SCHEMA_V1,
                command_context.clone(),
                inspection,
            );
            println!("{}", serde_norway::to_string(&envelope)?);
        }
        OutputFormat::Agent => {
            let report = bbt_service::agent_report(inspection, bbt_core::current_timestamp());
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }
    Ok(())
}

fn print_human_fs_inspect(inspection: &FsInspection) {
    println!("{}", inspection.path);
    println!();
    println!("Type:        {}", inspection.kind.label());
    println!(
        "Exists:      {}",
        if inspection.exists { "yes" } else { "no" }
    );

    if let Some(size) = inspection.size_bytes {
        println!("Size:        {} bytes", size);
    }

    if let Some(owner) = &inspection.owner {
        println!("Owner:       {}", owner.uid);
    }

    if let Some(group) = &inspection.group {
        println!("Group:       {}", group.gid);
    }

    if let Some(mode) = &inspection.mode {
        println!("Mode:        {}  {}", mode.octal, mode.symbolic);
    }

    if let Some(target) = &inspection.symlink_target {
        println!("Symlink:     -> {target}");
    }

    if let Some(modified) = &inspection.timestamps.modified {
        println!("Modified:    {modified}");
    }

    println!();
    println!("Access for current user:");
    println!(
        "  Read:      {}",
        yes_no_unknown(inspection.access.readable)
    );
    println!(
        "  Write:     {}",
        yes_no_unknown(inspection.access.writable)
    );
    println!(
        "  Execute:   {}",
        yes_no_unknown(inspection.access.executable)
    );

    println!();
    println!("Safe next observations:");
    let quoted_path = bbt_fs::shell_quote(&inspection.path);
    println!("  bbt --json fs inspect -- {quoted_path}");
    println!("  bbt --agent fs inspect -- {quoted_path}");
}

fn print_human_fs_list(listing: &FsListing) {
    println!("Directory listing: {}", listing.path);
    println!();
    if !listing.exists {
        println!("{} does not exist.", listing.path);
    } else {
        if !listing.is_directory {
            println!(
                "{} is not a directory; showing the path itself.",
                listing.path
            );
            println!();
        }
        println!("Entries: {}", listing.entry_count);
        println!();
        println!(
            "{:<10} {:>7} {:>7} {:>12}  {:<30}  NAME",
            "MODE", "OWNER", "GROUP", "SIZE", "MODIFIED"
        );
        for entry in &listing.entries {
            let name = match &entry.symlink_target {
                Some(target) => format!("{} -> {target}", entry.name),
                None => entry.name.clone(),
            };
            println!(
                "{}{:<9} {:>7} {:>7} {:>12}  {:<30}  {}",
                entry.kind.indicator(),
                entry.mode.symbolic,
                entry.owner.uid,
                entry.group.gid,
                entry.size_bytes,
                entry.modified.as_deref().unwrap_or("unknown"),
                name
            );
        }
    }

    if !listing.warnings.is_empty() {
        println!();
        println!("Warnings:");
        for warning in &listing.warnings {
            println!("  {warning}");
        }
    }

    println!();
    println!("Safe next observations:");
    let quoted_path = bbt_fs::shell_quote(&listing.path);
    println!("  bbt --json fs list -- {quoted_path}");
    println!("  bbt --agent fs list -- {quoted_path}");
}

fn print_human_fs_tree(tree: &FsTree) {
    println!("Directory tree: {}", tree.path);
    println!();
    if !tree.exists {
        println!("{} does not exist.", tree.path);
    } else if let Some(root) = &tree.root {
        println!(
            "Max depth:           {}",
            tree.max_depth
                .map(|depth| depth.to_string())
                .unwrap_or_else(|| "unlimited".to_owned())
        );
        println!(
            "One filesystem:      {}",
            if tree.one_filesystem { "yes" } else { "no" }
        );
        println!(
            "Total:               {} ({})",
            bbt_fs::human_bytes(root.total_bytes),
            root.total_bytes
        );
        println!(
            "Apparent size:       {} ({})",
            bbt_fs::human_bytes(root.apparent_bytes),
            root.apparent_bytes
        );
        println!("Entries seen:        {}", tree.entries_seen);
        println!("Skipped other fs:    {}", tree.skipped_different_filesystem);
        println!();
        println!("{}", human_tree_label(root));
        print_human_tree_children(root, "");
    }

    if !tree.warnings.is_empty() {
        println!();
        println!("Warnings:");
        for warning in &tree.warnings {
            println!("  {warning}");
        }
    }

    println!();
    println!("Safe next observations:");
    let quoted_path = bbt_fs::shell_quote(&tree.path);
    println!("  bbt --json fs tree -- {quoted_path}");
    println!("  bbt --agent fs tree -- {quoted_path}");
}

fn print_human_tree_children(node: &TreeNode, prefix: &str) {
    let Some(children) = &node.children else {
        return;
    };
    for (index, child) in children.iter().enumerate() {
        let last = index + 1 == children.len();
        println!(
            "{prefix}{}{}",
            if last { "└── " } else { "├── " },
            human_tree_label(child)
        );
        let child_prefix = format!("{prefix}{}", if last { "    " } else { "│   " });
        print_human_tree_children(child, &child_prefix);
    }
}

fn human_tree_label(node: &TreeNode) -> String {
    format!("{}  [{}]", node.name, bbt_fs::human_bytes(node.total_bytes))
}

fn print_human_disk_usage(usage: &DiskUsage) {
    println!("Disk usage: {}", usage.path);
    println!();
    println!(
        "Exists:              {}",
        if usage.exists { "yes" } else { "no" }
    );
    println!(
        "Max depth:           {}",
        usage
            .max_depth
            .map(|depth| depth.to_string())
            .unwrap_or_else(|| "unlimited".to_owned())
    );
    println!(
        "One filesystem:      {}",
        if usage.one_filesystem { "yes" } else { "no" }
    );
    println!("Top limit:           {}", usage.top_limit);
    println!(
        "Total:              {} ({})",
        bbt_disk::human_bytes(usage.total_bytes),
        usage.total_bytes
    );
    println!(
        "Apparent size:      {} ({})",
        bbt_disk::human_bytes(usage.apparent_bytes),
        usage.apparent_bytes
    );
    println!("Entries seen:        {}", usage.entries_seen);
    println!("Directories seen:    {}", usage.directories_seen);
    println!("Files seen:          {}", usage.files_seen);
    println!("Symlinks seen:       {}", usage.symlinks_seen);
    println!("Unreadable entries:  {}", usage.unreadable_entries);
    println!(
        "Skipped other fs:    {}",
        usage.skipped_different_filesystem
    );

    println!();
    println!("Largest children:");
    if usage.largest_children.is_empty() {
        println!("  none");
    } else {
        for child in &usage.largest_children {
            println!(
                "  {:>12}  {:<12} {}",
                bbt_disk::human_bytes(child.total_bytes),
                child.kind.label(),
                child.path
            );
        }
    }

    if !usage.errors.is_empty() {
        println!();
        println!("Errors:");
        for error in &usage.errors {
            println!("  {:?}: {} ({})", error.kind, error.path, error.message);
        }
    }

    println!();
    println!("Safe next observations:");
    let quoted_path = bbt_disk::shell_quote(&usage.path);
    println!("  bbt --json disk usage -- {quoted_path}");
    println!("  bbt --agent disk usage -- {quoted_path}");
}

fn print_human_disk_filesystems(report: &FilesystemReport) {
    println!("Mounted filesystems: {}", report.filesystems.len());
    println!();
    println!(
        "{:<28} {:<10} {:>10} {:>10} {:>7}  SOURCE",
        "TARGET", "FSTYPE", "SIZE", "AVAIL", "USED"
    );
    for fs in &report.filesystems {
        println!(
            "{:<28} {:<10} {:>10} {:>10} {:>7}  {}",
            fs.target,
            fs.fstype,
            fs.total_bytes
                .map(bbt_disk::human_bytes)
                .unwrap_or_else(|| "unknown".to_owned()),
            fs.available_bytes
                .map(bbt_disk::human_bytes)
                .unwrap_or_else(|| "unknown".to_owned()),
            fs.used_percent
                .map(|value| format!("{value:.1}%"))
                .unwrap_or_else(|| "unknown".to_owned()),
            fs.source
        );
    }
    if !report.warnings.is_empty() {
        println!();
        println!("Warnings:");
        for warning in &report.warnings {
            println!("  {warning}");
        }
    }
    println!();
    println!("Safe next observations:");
    println!("  bbt --json disk filesystems");
    println!("  bbt --agent disk filesystems");
}

fn print_human_disk_pressure(info: &PressureInfo) {
    println!("Disk I/O pressure (PSI)");
    println!();
    if !info.available {
        println!(
            "PSI is unavailable on this system (kernel older than 4.20 or CONFIG_PSI disabled)."
        );
    } else {
        println!(
            "{:<6} {:>8} {:>8} {:>8} {:>16}",
            "", "AVG10", "AVG60", "AVG300", "TOTAL STALL"
        );
        for (label, line) in [("some", &info.some), ("full", &info.full)] {
            match line {
                Some(line) => println!(
                    "{:<6} {:>7.2}% {:>7.2}% {:>7.2}% {:>14.1} s",
                    label,
                    line.avg10,
                    line.avg60,
                    line.avg300,
                    line.total_stalled_usec as f64 / 1_000_000.0
                ),
                None => println!("{:<6} unavailable", label),
            }
        }
        println!();
        println!("some = at least one task stalled on I/O; full = all non-idle tasks stalled.");
    }
    if !info.warnings.is_empty() {
        println!();
        println!("Warnings:");
        for warning in &info.warnings {
            println!("  {warning}");
        }
    }
    println!();
    println!("Safe next observations:");
    println!("  bbt --json disk pressure");
    println!("  bbt --agent disk pressure");
}

fn print_human_host_snapshot(snapshot: &HostSnapshot) {
    println!("Host snapshot: {}", snapshot.hostname);
    println!();
    println!(
        "Kernel:      {} {} ({})",
        snapshot.kernel.sysname, snapshot.kernel.release, snapshot.kernel.machine
    );
    if let Some(pretty_name) = &snapshot.os.pretty_name {
        println!("OS:          {pretty_name}");
    }
    println!(
        "User:        {} uid={} euid={} gid={} egid={}",
        snapshot
            .current_user
            .username
            .as_deref()
            .unwrap_or("unknown"),
        snapshot.current_user.uid,
        snapshot.current_user.effective_uid,
        snapshot.current_user.gid,
        snapshot.current_user.effective_gid
    );
    println!(
        "CPU:         {} logical{}{}",
        snapshot.cpu.logical_cpus,
        snapshot
            .cpu
            .model_name
            .as_ref()
            .map(|model| format!(", {model}"))
            .unwrap_or_default(),
        snapshot
            .cpu
            .cpu_mhz
            .map(|mhz| format!(", {mhz:.0} MHz"))
            .unwrap_or_default()
    );
    if let Some(uptime) = &snapshot.uptime {
        println!("Uptime:      {:.0} seconds", uptime.seconds);
    }
    if let Some(load) = &snapshot.load_average {
        println!(
            "Load avg:    {:.2} {:.2} {:.2}",
            load.one_minute, load.five_minutes, load.fifteen_minutes
        );
    }
    println!();
    println!(
        "Memory:      {} total, {} available, {} used",
        bbt_host::human_bytes(snapshot.memory.total_bytes),
        snapshot
            .memory
            .available_bytes
            .map(bbt_host::human_bytes)
            .unwrap_or_else(|| "unknown".to_owned()),
        snapshot
            .memory
            .used_percent
            .map(|value| format!("{value:.1}%"))
            .unwrap_or_else(|| "unknown".to_owned())
    );
    println!();
    println!("Filesystems:");
    for fs in snapshot.filesystems.iter().take(8) {
        println!(
            "  {:<18} {:>8} used  {}",
            fs.target,
            fs.used_percent
                .map(|value| format!("{value:.1}%"))
                .unwrap_or_else(|| "unknown".to_owned()),
            fs.fstype
        );
    }
    if snapshot.filesystems.len() > 8 {
        println!("  ... {} more", snapshot.filesystems.len() - 8);
    }
    if !snapshot.warnings.is_empty() {
        println!();
        println!("Warnings:");
        for warning in &snapshot.warnings {
            println!("  {warning}");
        }
    }
    println!();
    println!("Safe next observations:");
    println!("  bbt --json host snapshot");
    println!("  bbt --agent disk usage --depth 1 -- /");
}

fn print_human_proc_inspect(inspection: &ProcInspection) {
    println!("Process inspect: {}", inspection.pid);
    println!();
    println!(
        "Exists:      {}",
        if inspection.exists { "yes" } else { "no" }
    );
    println!(
        "Name:        {}",
        bbt_proc::escape_human(inspection.name.as_deref().unwrap_or("unknown"))
    );
    println!(
        "State:       {}",
        bbt_proc::escape_human(inspection.state.as_deref().unwrap_or("unknown"))
    );
    println!(
        "Parent PID:  {}",
        inspection
            .parent_pid
            .map(|pid| pid.to_string())
            .unwrap_or_else(|| "unknown".to_owned())
    );
    println!("Children:    {}", inspection.child_pids.len());
    println!(
        "User:        {} uid={} gid={}",
        bbt_proc::escape_human(inspection.user.as_deref().unwrap_or("unknown")),
        inspection
            .uid
            .map(|uid| uid.to_string())
            .unwrap_or_else(|| "unknown".to_owned()),
        inspection
            .gid
            .map(|gid| gid.to_string())
            .unwrap_or_else(|| "unknown".to_owned())
    );
    println!(
        "Threads:     {}",
        inspection
            .threads
            .map(|threads| threads.to_string())
            .unwrap_or_else(|| "unknown".to_owned())
    );
    println!(
        "RSS:         {}",
        inspection
            .memory
            .vm_rss_bytes
            .map(bbt_proc::human_bytes)
            .unwrap_or_else(|| "unknown".to_owned())
    );
    if let Some(exe) = &inspection.executable {
        println!("Executable:  {}", bbt_proc::escape_human(exe));
    }
    if let Some(cwd) = &inspection.cwd {
        println!("CWD:         {}", bbt_proc::escape_human(cwd));
    }
    if !inspection.command_line.is_empty() {
        let command = inspection
            .command_line
            .iter()
            .map(|argument| bbt_proc::escape_human_argument(argument))
            .collect::<Vec<_>>()
            .join(" ");
        println!("Command:     {command}");
    }
    if !inspection.warnings.is_empty() {
        println!();
        println!("Warnings:");
        for warning in &inspection.warnings {
            println!("  {}", bbt_proc::escape_human(warning));
        }
    }

    println!();
    println!("Safe next observations:");
    println!("  bbt --json proc inspect {}", inspection.pid);
    println!("  bbt --agent host snapshot");
}

fn print_human_proc_list(list: &ProcList) {
    println!("Process list");
    println!();
    println!("Processes: {}", list.process_count);
    println!();
    println!(
        "{:>7} {:>7} {:<10} {:>10} COMMAND",
        "PID", "PPID", "USER", "RSS"
    );
    for process in list.processes.iter().take(30) {
        let command = if process.command_line.is_empty() {
            process.name.as_deref().unwrap_or("unknown").to_owned()
        } else {
            process.command_line.join(" ")
        };
        println!(
            "{:>7} {:>7} {:<10} {:>10} {}",
            process.pid,
            process
                .parent_pid
                .map(|pid| pid.to_string())
                .unwrap_or_else(|| "-".to_owned()),
            bbt_proc::escape_human(process.user.as_deref().unwrap_or("unknown")),
            process
                .vm_rss_bytes
                .map(bbt_proc::human_bytes)
                .unwrap_or_else(|| "unknown".to_owned()),
            bbt_proc::escape_human(&command)
        );
    }
    if list.processes.len() > 30 {
        println!("... {} more", list.processes.len() - 30);
    }
    if !list.warnings.is_empty() {
        println!();
        println!("Warnings:");
        for warning in &list.warnings {
            println!("  {}", bbt_proc::escape_human(warning));
        }
    }
    println!();
    println!("Safe next observations:");
    println!("  bbt --json proc list");
    println!("  bbt --agent net listeners");
}

fn print_human_net_listeners(report: &ListenerReport) {
    println!("Network listeners");
    println!();
    println!("Listeners: {}", report.listener_count);
    println!();
    println!(
        "{:<6} {:<39} {:>5} {:<12} PROCESS",
        "PROTO", "ADDRESS", "PORT", "PIDS"
    );
    for listener in &report.listeners {
        let pids = if listener.pids.is_empty() {
            "-".to_owned()
        } else {
            listener
                .pids
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(",")
        };
        let processes = if listener.processes.is_empty() {
            "unknown".to_owned()
        } else {
            listener.processes.join(",")
        };
        println!(
            "{:<6} {:<39} {:>5} {:<12} {}",
            listener.protocol,
            bbt_net::escape_human(&listener.address),
            listener.port,
            pids,
            bbt_net::escape_human(&processes)
        );
    }
    if !report.warnings.is_empty() {
        println!();
        println!("Warnings:");
        for warning in &report.warnings {
            println!("  {}", bbt_net::escape_human(warning));
        }
    }
    println!();
    println!("Safe next observations:");
    println!("  bbt --json net listeners");
    println!("  bbt --agent proc list");
}

fn print_human_service_inspect(inspection: &ServiceInspection) {
    println!(
        "Service inspect: {}",
        bbt_service::escape_human(&inspection.name)
    );
    println!();
    println!(
        "Exists:       {}",
        if inspection.exists { "yes" } else { "no" }
    );
    println!(
        "Load state:   {}",
        inspection.load_state.as_deref().unwrap_or("unknown")
    );
    println!(
        "Active state: {}",
        inspection.active_state.as_deref().unwrap_or("unknown")
    );
    println!(
        "Sub state:    {}",
        inspection.sub_state.as_deref().unwrap_or("unknown")
    );
    println!(
        "Unit file:    {}",
        inspection.unit_file_state.as_deref().unwrap_or("unknown")
    );
    if let Some(description) = &inspection.description {
        println!("Description:  {}", bbt_service::escape_human(description));
    }
    println!(
        "Main PID:     {}",
        inspection
            .main_pid
            .map(|pid| pid.to_string())
            .unwrap_or_else(|| "none".to_owned())
    );
    if let Some(path) = &inspection.fragment_path {
        println!("Unit path:    {}", bbt_service::escape_human(path));
    }
    if !inspection.warnings.is_empty() {
        println!();
        println!("Warnings:");
        for warning in &inspection.warnings {
            println!("  {}", bbt_service::escape_human(warning));
        }
    }
    println!();
    println!("Safe next observations:");
    println!(
        "  bbt --json service inspect {}",
        bbt_service::shell_quote(&inspection.name)
    );
    println!("  bbt --agent net listeners");
}

fn command_context() -> CommandContext {
    let args: Vec<_> = std::env::args_os().collect();
    let argv = args
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let argv_bytes_hex = args.iter().map(|arg| bytes_hex(arg.as_bytes())).collect();

    CommandContext::new(
        argv,
        argv_bytes_hex,
        std::env::current_dir()
            .ok()
            .map(|path| path.display().to_string()),
        Some(unsafe { libc::geteuid() }),
    )
}

fn bytes_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn yes_no_unknown(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "yes",
        Some(false) => "no",
        None => "unknown",
    }
}
