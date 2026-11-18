use std::{
    collections::HashSet,
    fs::File,
    io::{self, Read, Seek, SeekFrom},
};

use crate::bootloader::{BootSector, invalid_data};
use crate::fat::{FIRST_END_OF_CHAIN_MARKER, read_fat_entry};

pub(super) struct DirectoryEntry {
    pub(super) name: String,
    pub(super) is_directory: bool,
    pub(super) first_cluster: u32,
    pub(super) file_size: u32,
}

pub(super) fn run(boot: &BootSector, image: &mut File, path: &str) -> io::Result<()> {
    for entry in open_directory(boot, image, path)? {
        if matches!(entry.name.as_str(), "." | "..") {
            continue;
        }
        let suffix = if entry.is_directory { "/" } else { "" };
        println!("{}{suffix}", entry.name);
    }
    Ok(())
}

pub(super) fn open_directory(
    boot: &BootSector,
    image: &mut File,
    path: &str,
) -> io::Result<Vec<DirectoryEntry>> {
    let mut cluster = boot.root_cluster;
    let mut parents = Vec::new();
    let mut entries = read_directory(boot, image, cluster)?;

    for name in path.split('/').filter(|name| !matches!(*name, "" | ".")) {
        if name == ".." {
            cluster = parents.pop().unwrap_or(boot.root_cluster);
        } else {
            let entry = entries
                .iter()
                .find(|entry| entry.name.eq_ignore_ascii_case(name))
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("directory {name:?} not found"),
                    )
                })?;
            if !entry.is_directory {
                return Err(io::Error::new(
                    io::ErrorKind::NotADirectory,
                    format!("{name:?} is not a directory"),
                ));
            }
            parents.push(cluster);
            cluster = entry.first_cluster;
        }
        entries = read_directory(boot, image, cluster)?;
    }

    Ok(entries)
}

fn read_directory(
    boot: &BootSector,
    image: &mut File,
    mut cluster: u32,
) -> io::Result<Vec<DirectoryEntry>> {
    const ENTRY_SIZE: usize = 32;
    const ATTR_VOLUME_ID: u8 = 0x08;
    const MAX_DIRECTORY_SIZE: usize = 65_536 * ENTRY_SIZE;

    let fat_offset = boot.active_fat_offset()?;
    let cluster_size = usize::from(boot.sectors_per_cluster) * usize::from(boot.bytes_per_sector);
    let data_clusters = 2..=boot.cluster_count()? + 1;
    if !data_clusters.contains(&u64::from(cluster)) {
        return Err(invalid_data(
            "directory start cluster is outside the data region",
        ));
    }
    let directory = if cluster == boot.root_cluster {
        "root directory"
    } else {
        "directory"
    };
    let mut visited = HashSet::new();
    let mut bytes = vec![0; cluster_size];
    let mut entries = Vec::new();

    loop {
        if !visited.insert(cluster) {
            return Err(invalid_data(&format!(
                "cycle in the {directory} cluster chain"
            )));
        }
        if visited.len() * cluster_size > MAX_DIRECTORY_SIZE {
            return Err(invalid_data(&format!(
                "{directory} exceeds the FAT size limit"
            )));
        }

        image.seek(SeekFrom::Start(boot.cluster_offset(cluster)))?;
        image.read_exact(&mut bytes)?;
        for entry in bytes.chunks_exact(ENTRY_SIZE) {
            if entry[0] == 0x00 {
                return Ok(entries);
            }
            if entry[0] == 0xE5 || entry[11] & ATTR_VOLUME_ID != 0 {
                continue;
            }
            entries.push(DirectoryEntry::from_short_entry(entry));
        }

        let next = read_fat_entry(image, fat_offset, cluster)?;
        if next >= FIRST_END_OF_CHAIN_MARKER {
            return Ok(entries);
        }
        if !data_clusters.contains(&u64::from(next)) {
            return Err(invalid_data(&format!(
                "invalid link in the {directory} cluster chain"
            )));
        }
        cluster = next;
    }
}

impl DirectoryEntry {
    fn from_short_entry(entry: &[u8]) -> Self {
        let mut stem = [0; 8];
        stem.copy_from_slice(&entry[..8]);
        if stem[0] == 0x05 {
            stem[0] = 0xE5;
        }

        let case_flags = entry[12];
        let mut name = Self::display_short_name_part(&stem, case_flags & 0x08 != 0);
        let extension = Self::display_short_name_part(&entry[8..11], case_flags & 0x10 != 0);
        if !extension.is_empty() {
            name.push('.');
            name.push_str(&extension);
        }

        let cluster_high = u32::from(u16::from_le_bytes([entry[20], entry[21]]));
        let cluster_low = u32::from(u16::from_le_bytes([entry[26], entry[27]]));
        let file_size = u32::from_le_bytes([entry[28], entry[29], entry[30], entry[31]]);
        Self {
            name,
            is_directory: entry[11] & 0x10 != 0,
            first_cluster: (cluster_high << 16) | cluster_low,
            file_size,
        }
    }

    fn display_short_name_part(bytes: &[u8], lowercase: bool) -> String {
        let mut name = String::new();
        for &byte in bytes.trim_ascii_end() {
            if byte.is_ascii_graphic() || byte == b' ' {
                name.push(char::from(byte));
            } else {
                name.extend(std::ascii::escape_default(byte).map(char::from));
            }
        }
        if lowercase {
            name.make_ascii_lowercase();
        }
        name
    }
}
