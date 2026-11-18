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
    let (Some(command), Some(path)) = (args.next(), args.next()) else {
        return Err(usage_error());
    };
    let directory = args.next();
    let list_path = match (command.to_str(), directory.as_deref(), args.next()) {
        (Some("info"), None, None) => None,
        (Some("ls"), None, None) => Some("/"),
        (Some("ls"), Some(directory), None) => Some(directory.to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "directory path must be UTF-8")
        })?),
        _ => return Err(usage_error()),
    };

    let mut image = File::open(path)?;
    let boot = BootSector::read(&mut image)?;

    if let Some(path) = list_path {
        return ls::run(&boot, &mut image, path);
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
        "usage: fat32-explorer info <image>\n       fat32-explorer ls <image> [directory]",
    )
}
