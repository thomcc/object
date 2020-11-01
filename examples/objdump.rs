use object::read::archive::ArchiveFile;
use object::read::macho::{FatArch, FatHeader};
use object::{Object, ObjectComdat, ObjectSection, ObjectSymbol};
use std::{env, fs, process};

fn main() {
    let arg_len = env::args().len();
    if arg_len <= 1 {
        eprintln!("Usage: {} <file> ...", env::args().next().unwrap());
        process::exit(1);
    }

    for file_path in env::args().skip(1) {
        if arg_len > 2 {
            println!();
            println!("{}:", file_path);
        }

        let file = match fs::File::open(&file_path) {
            Ok(file) => file,
            Err(err) => {
                println!("Failed to open file '{}': {}", file_path, err,);
                continue;
            }
        };
        let file = match unsafe { memmap::Mmap::map(&file) } {
            Ok(mmap) => mmap,
            Err(err) => {
                println!("Failed to map file '{}': {}", file_path, err,);
                continue;
            }
        };

        if let Ok(archive) = ArchiveFile::parse(&*file) {
            println!("Format: Archive (kind: {:?})", archive.kind());
            for member in archive.members() {
                if let Ok(member) = member {
                    println!();
                    println!("{}:", String::from_utf8_lossy(member.name()));
                    dump_object(member.data());
                }
            }
        } else if let Ok(arches) = FatHeader::parse_arch32(&*file) {
            println!("Format: Mach-O Fat 32");
            for arch in arches {
                println!();
                println!("Fat Arch: {:?}", arch.architecture());
                if let Ok(data) = arch.data(&*file) {
                    dump_object(data);
                }
            }
        } else if let Ok(arches) = FatHeader::parse_arch64(&*file) {
            println!("Format: Mach-O Fat 64");
            for arch in arches {
                println!();
                println!("Fat Arch: {:?}", arch.architecture());
                if let Ok(data) = arch.data(&*file) {
                    dump_object(data);
                }
            }
        } else {
            dump_object(&*file);
        }
    }
}

fn dump_object(data: &[u8]) {
    let file = match object::File::parse(data) {
        Ok(file) => file,
        Err(err) => {
            println!("Failed to parse file: {}", err);
            return;
        }
    };
    println!(
        "Format: {:?} {:?}-endian {}-bit",
        file.format(),
        file.endianness(),
        if file.is_64() { "64" } else { "32" }
    );
    println!("Architecture: {:?}", file.architecture());
    match file.mach_uuid() {
        Ok(Some(uuid)) => println!("Mach UUID: {:x?}", uuid),
        Ok(None) => {}
        Err(e) => println!("Failed to parse Mach UUID: {}", e),
    }
    match file.build_id() {
        Ok(Some(build_id)) => println!("Build ID: {:x?}", build_id),
        Ok(None) => {}
        Err(e) => println!("Failed to parse build ID: {}", e),
    }
    match file.gnu_debuglink() {
        Ok(Some((filename, crc))) => println!(
            "GNU debug link: {} CRC: {:08x}",
            String::from_utf8_lossy(filename),
            crc,
        ),
        Ok(None) => {}
        Err(e) => println!("Failed to parse GNU debug link: {}", e),
    }

    for segment in file.segments() {
        println!("{:?}", segment);
    }

    for section in file.sections() {
        println!("{}: {:?}", section.index().0, section);
    }

    for comdat in file.comdats() {
        print!("{:?} Sections:", comdat);
        for section in comdat.sections() {
            print!(" {}", section.0);
        }
        println!();
    }

    for symbol in file.symbols() {
        println!("{}: {:?}", symbol.index().0, symbol);
    }

    for section in file.sections() {
        if section.relocations().next().is_some() {
            println!(
                "\n{} relocations",
                section.name().unwrap_or("<invalid name>")
            );
            for relocation in section.relocations() {
                println!("{:?}", relocation);
            }
        }
    }
}
