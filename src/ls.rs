use std::{
    collections::HashSet,
    fs::File,
    io::{self, Read, Seek, SeekFrom},
};

use crate::bootloader::{BootSector, invalid_data};

struct DirectoryEntry {
    name: String,
    is_directory: bool,
}

pub(super) fn run(boot: &BootSector, image: &mut File) -> io::Result<()> {
    for entry in read_root_directory(boot, image)? {
        let suffix = if entry.is_directory { "/" } else { "" };
        println!("{}{suffix}", entry.name);
    }
    Ok(())
}

fn read_root_directory(boot: &BootSector, image: &mut File) -> io::Result<Vec<DirectoryEntry>> {
    const ENTRY_SIZE: usize = 32;
    const ATTR_VOLUME_ID: u8 = 0x08;
    const MAX_DIRECTORY_SIZE: usize = 65_536 * ENTRY_SIZE;

    let fat_offset = boot.active_fat_offset()?;
    let cluster_size = usize::from(boot.sectors_per_cluster) * usize::from(boot.bytes_per_sector);
    let last_cluster = boot.cluster_count()? + 1;
    let mut cluster = boot.root_cluster;
    let mut visited = HashSet::new();
    let mut bytes = vec![0; cluster_size];
    let mut entries = Vec::new();

    loop {
        if !visited.insert(cluster) {
            return Err(invalid_data("cycle in the root directory cluster chain"));
        }
        if visited.len() * cluster_size > MAX_DIRECTORY_SIZE {
            return Err(invalid_data("root directory exceeds the FAT size limit"));
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
        if next >= 0x0FFF_FFF8 {
            return Ok(entries);
        }
        if !(2..=last_cluster).contains(&u64::from(next)) {
            return Err(invalid_data(
                "invalid link in the root directory cluster chain",
            ));
        }
        cluster = next;
    }
}

fn read_fat_entry(image: &mut File, fat_offset: u64, cluster: u32) -> io::Result<u32> {
    let offset = fat_offset + u64::from(cluster) * 4;
    let mut bytes = [0; 4];
    image.seek(SeekFrom::Start(offset))?;
    image.read_exact(&mut bytes)?;
    // FAT32 reserves the high four bits of each FAT entry.
    Ok(u32::from_le_bytes(bytes) & 0x0FFF_FFFF)
}

impl DirectoryEntry {
    fn from_short_entry(entry: &[u8]) -> Self {
        let mut stem = [0; 8];
        stem.copy_from_slice(&entry[..8]);
        // FAT uses 0x05 for an initial 0xE5 byte to distinguish it from a deleted entry.
        if stem[0] == 0x05 {
            stem[0] = 0xE5;
        }

        let case_flags = entry[12];
        let mut name = display_short_name_part(&stem, case_flags & 0x08 != 0);
        let extension = display_short_name_part(&entry[8..11], case_flags & 0x10 != 0);
        if !extension.is_empty() {
            name.push('.');
            name.push_str(&extension);
        }

        Self {
            name,
            is_directory: entry[11] & 0x10 != 0,
        }
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
