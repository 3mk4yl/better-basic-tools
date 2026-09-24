//! Shared mounted-filesystem enumeration from `/proc/mounts` and `statvfs`.
//!
//! Used by both `bbt host snapshot` and `bbt disk filesystems`.

use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::fs;
use std::io;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemInfo {
    pub source: String,
    pub target: String,
    pub fstype: String,
    pub total_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
    pub used_percent: Option<f64>,
}

pub fn read_filesystems(warnings: &mut Vec<String>) -> Vec<FilesystemInfo> {
    let mounts = match fs::read_to_string("/proc/mounts") {
        Ok(mounts) => mounts,
        Err(error) => {
            warnings.push(format!("/proc/mounts unavailable: {error}"));
            return Vec::new();
        }
    };

    let mut filesystems = Vec::new();
    for line in mounts.lines() {
        let mut parts = line.split_whitespace();
        let Some(source) = parts.next() else { continue };
        let Some(target) = parts.next() else { continue };
        let Some(fstype) = parts.next() else { continue };
        if is_noise_filesystem(fstype) {
            continue;
        }

        let source = unescape_mount_field(source);
        let target = unescape_mount_field(target);
        let (total_bytes, available_bytes, used_percent) = match statvfs(&target) {
            Ok(values) => values,
            Err(error) => {
                warnings.push(format!("statvfs failed for {target}: {error}"));
                (None, None, None)
            }
        };
        filesystems.push(FilesystemInfo {
            source,
            target,
            fstype: fstype.to_owned(),
            total_bytes,
            available_bytes,
            used_percent,
        });
    }

    filesystems.sort_by(|left, right| left.target.cmp(&right.target));
    filesystems
}

fn statvfs(path: &str) -> io::Result<(Option<u64>, Option<u64>, Option<f64>)> {
    let c_path =
        CString::new(path).map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    if unsafe { libc::statvfs(c_path.as_ptr(), stats.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let stats = unsafe { stats.assume_init() };
    let fragment_size = stats.f_frsize;
    let total_bytes = stats.f_blocks.saturating_mul(fragment_size);
    let available_bytes = stats.f_bavail.saturating_mul(fragment_size);
    let used_bytes = total_bytes.saturating_sub(available_bytes);
    Ok((
        Some(total_bytes),
        Some(available_bytes),
        percent(used_bytes, total_bytes),
    ))
}

fn percent(used: u64, total: u64) -> Option<f64> {
    if total == 0 {
        None
    } else {
        Some((used as f64 / total as f64) * 100.0)
    }
}

fn is_noise_filesystem(fstype: &str) -> bool {
    matches!(
        fstype,
        "autofs"
            | "binfmt_misc"
            | "cgroup"
            | "cgroup2"
            | "configfs"
            | "debugfs"
            | "devpts"
            | "devtmpfs"
            | "fusectl"
            | "hugetlbfs"
            | "mqueue"
            | "proc"
            | "pstore"
            | "securityfs"
            | "sysfs"
            | "tracefs"
    )
}

fn unescape_mount_field(value: &str) -> String {
    value
        .replace("\\040", " ")
        .replace("\\011", "\t")
        .replace("\\012", "\n")
        .replace("\\134", "\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unescapes_mount_fields_before_statvfs_paths_need_them() {
        assert_eq!(
            unescape_mount_field("/mnt/with\\040space"),
            "/mnt/with space"
        );
        assert_eq!(
            unescape_mount_field("server:/with\\134slash"),
            "server:/with\\slash"
        );
    }

    #[test]
    fn filters_pseudo_filesystem_noise() {
        assert!(is_noise_filesystem("proc"));
        assert!(is_noise_filesystem("sysfs"));
        assert!(is_noise_filesystem("cgroup2"));
        assert!(!is_noise_filesystem("ext4"));
        assert!(!is_noise_filesystem("tmpfs"));
        assert!(!is_noise_filesystem("btrfs"));
    }

    #[test]
    fn read_filesystems_returns_sorted_non_noise_mounts() {
        let mut warnings = Vec::new();

        let filesystems = read_filesystems(&mut warnings);

        assert!(!filesystems.is_empty());
        assert!(filesystems
            .iter()
            .all(|fs| !is_noise_filesystem(&fs.fstype)));
        assert!(filesystems
            .windows(2)
            .all(|pair| pair[0].target <= pair[1].target));
    }
}
