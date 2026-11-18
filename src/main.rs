mod bootloader;
mod ls;

use std::{env, fs::File, io, process::ExitCode};

use bootloader::BootSector;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("fat32-explorer: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> io::Result<()> {
    let mut args = env::args_os().skip(1);
    let (Some(command), Some(path), None) = (args.next(), args.next(), args.next()) else {
        return Err(usage_error());
    };
    let Some(command @ ("info" | "ls")) = command.to_str() else {
        return Err(usage_error());
    };

    let mut image = File::open(path)?;
    let boot = BootSector::read(&mut image)?;

    if command == "ls" {
        return ls::run(&boot, &mut image);
    }
    print_info(&boot);
    Ok(())
}

fn print_info(boot: &BootSector) {
    println!("Bytes per sector: {}", boot.bytes_per_sector);
    println!("Sectors per cluster: {}", boot.sectors_per_cluster);
    println!("Reserved sectors: {}", boot.reserved_sectors);
    println!("FAT copies: {}", boot.fat_count);
    println!("Sectors per FAT: {}", boot.sectors_per_fat);
    println!("Total sectors: {}", boot.total_sectors);
    println!("First data sector: {}", boot.first_data_sector());
    println!("Root directory cluster: {}", boot.root_cluster);
    println!(
        "Root directory byte offset: {}",
        boot.root_directory_offset()
    );
}

fn usage_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "usage: fat32-explorer <info|ls> <image>",
    )
}
