use std::{
    collections::HashSet,
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
};

use crate::bootloader::{BootSector, invalid_data};
use crate::fat::{FIRST_END_OF_CHAIN_MARKER, read_fat_entry};
use crate::ls::{DirectoryEntry, open_directory};

pub(super) fn run(boot: &BootSector, image: &mut File, path: &str) -> io::Result<()> {
    let (parent, name) = path.rsplit_once('/').unwrap_or(("/", path));
    if name.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected a file name",
        ));
    }

    let entry = open_directory(boot, image, parent)?
        .into_iter()
        .find(|entry| entry.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, format!("file {path:?} not found"))
        })?;
    if entry.is_directory {
        return Err(io::Error::new(
            io::ErrorKind::IsADirectory,
            format!("{path:?} is a directory"),
        ));
    }

    write_contents(boot, image, &entry, &mut io::stdout().lock())
}

fn write_contents(
    boot: &BootSector,
    image: &mut File,
    entry: &DirectoryEntry,
    output: &mut impl Write,
) -> io::Result<()> {
    let mut remaining = u64::from(entry.file_size);
    if remaining == 0 {
        return Ok(());
    }

    let fat_offset = boot.active_fat_offset()?;
    let data_clusters = 2..=boot.cluster_count()? + 1;
    let cluster_size = usize::from(boot.sectors_per_cluster) * usize::from(boot.bytes_per_sector);
    let mut bytes = vec![0; cluster_size];
    let mut cluster = entry.first_cluster;
    let mut visited = HashSet::new();

    loop {
        if !data_clusters.contains(&u64::from(cluster)) {
            return Err(invalid_data("file cluster is outside the data region"));
        }
        if !visited.insert(cluster) {
            return Err(invalid_data("cycle in the file cluster chain"));
        }

        let count = remaining.min(cluster_size as u64) as usize;
        image.seek(SeekFrom::Start(boot.cluster_offset(cluster)))?;
        image.read_exact(&mut bytes[..count])?;
        output.write_all(&bytes[..count])?;
        remaining -= count as u64;
        if remaining == 0 {
            return output.flush();
        }

        cluster = read_fat_entry(image, fat_offset, cluster)?;
        if cluster >= FIRST_END_OF_CHAIN_MARKER {
            return Err(invalid_data(
                "file cluster chain ends before the declared size",
            ));
        }
    }
}
