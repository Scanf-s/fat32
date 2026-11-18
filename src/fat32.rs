use std::{
    fs::File,
    io::{self, Read},
};

pub struct BootSector {
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub fat_count: u8,
    pub sectors_per_fat: u32,
    pub total_sectors: u32,
    pub root_cluster: u32,
}

impl BootSector {
    pub fn read(image: &mut File) -> io::Result<Self> {
        let mut bytes = [0; 512];
        image.read_exact(&mut bytes)?;

        let u16_at = |offset: usize| u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        let u32_at = |offset: usize| {
            u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ])
        };

        if bytes[510..512] != [0x55, 0xAA] {
            return Err(invalid_data("missing boot sector signature"));
        }
        // FAT32 uses 32-bit sizes and stores the root directory in data clusters.
        if u16_at(17) != 0 || u16_at(19) != 0 || u16_at(22) != 0 {
            return Err(invalid_data("expected a FAT32 boot sector at byte 0"));
        }
        if u16_at(42) != 0 {
            return Err(invalid_data("unsupported FAT32 version"));
        }

        let boot = Self {
            bytes_per_sector: u16_at(11),
            sectors_per_cluster: bytes[13],
            reserved_sectors: u16_at(14),
            fat_count: bytes[16],
            sectors_per_fat: u32_at(36),
            total_sectors: u32_at(32),
            root_cluster: u32_at(44),
        };
        boot.validate(image.metadata()?.len())?;
        Ok(boot)
    }

    pub fn first_data_sector(&self) -> u64 {
        u64::from(self.reserved_sectors)
            + u64::from(self.fat_count) * u64::from(self.sectors_per_fat)
    }

    pub fn root_directory_offset(&self) -> u64 {
        let sectors_from_data_start =
            u64::from(self.root_cluster - 2) * u64::from(self.sectors_per_cluster);
        (self.first_data_sector() + sectors_from_data_start) * u64::from(self.bytes_per_sector)
    }

    fn validate(&self, image_size: u64) -> io::Result<()> {
        if !matches!(self.bytes_per_sector, 512 | 1024 | 2048 | 4096)
            || !self.sectors_per_cluster.is_power_of_two()
            || self.reserved_sectors == 0
            || self.fat_count == 0
            || self.sectors_per_fat == 0
        {
            return Err(invalid_data("invalid FAT32 volume geometry"));
        }

        let total_sectors = u64::from(self.total_sectors);
        let data_sectors = total_sectors
            .checked_sub(self.first_data_sector())
            .ok_or_else(|| invalid_data("data region starts outside the volume"))?;
        let cluster_count = data_sectors / u64::from(self.sectors_per_cluster);
        let last_cluster = cluster_count + 1;
        if cluster_count < 65_525 || last_cluster >= 0x0FFF_FFF7 {
            return Err(invalid_data("cluster count is outside the FAT32 range"));
        }
        if !(2..=last_cluster).contains(&u64::from(self.root_cluster)) {
            return Err(invalid_data(
                "root directory cluster is outside the data region",
            ));
        }

        let sector_size = u64::from(self.bytes_per_sector);
        let fat_size = u64::from(self.sectors_per_fat) * sector_size;
        if fat_size / 4 <= last_cluster {
            return Err(invalid_data("FAT is too small for the data region"));
        }
        if total_sectors * sector_size > image_size {
            return Err(invalid_data(
                "image is shorter than the declared volume size",
            ));
        }
        Ok(())
    }
}

fn invalid_data(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
