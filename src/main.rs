mod bootloader;
mod cat;
mod fat;
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
    enum Command<'a> {
        Info,
        List(&'a str),
        Cat(&'a str),
    }

    let mut args = env::args_os().skip(1);
    let (Some(command), Some(image_path)) = (args.next(), args.next()) else {
        return Err(usage_error());
    };
    let target = args.next();
    let command = match (command.to_str(), target.as_deref(), args.next()) {
        (Some("info"), None, None) => Command::Info,
        (Some("ls"), None, None) => Command::List("/"),
        (Some("ls"), Some(directory), None) => {
            Command::List(directory.to_str().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "directory path must be UTF-8")
            })?)
        }
        (Some("cat"), Some(file), None) => Command::Cat(file.to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "file path must be UTF-8")
        })?),
        _ => return Err(usage_error()),
    };

    let mut image = File::open(image_path)?;
    let boot = BootSector::read(&mut image)?;

    match command {
        Command::Info => {
            print_info(&boot);
            Ok(())
        }
        Command::List(path) => ls::run(&boot, &mut image, path),
        Command::Cat(path) => cat::run(&boot, &mut image, path),
    }
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
        "usage: fat32-explorer info <image>\n       fat32-explorer ls <image> [directory]\n       fat32-explorer cat <image> <file>",
    )
}
