use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
};

pub(super) const FIRST_END_OF_CHAIN_MARKER: u32 = 0x0FFF_FFF8;

pub(super) fn read_fat_entry(image: &mut File, fat_offset: u64, cluster: u32) -> io::Result<u32> {
    let offset = fat_offset + u64::from(cluster) * 4;
    let mut bytes = [0; 4];
    image.seek(SeekFrom::Start(offset))?;
    image.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes) & 0x0FFF_FFFF)
}
