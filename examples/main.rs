use std::{
    fs::{self, File},
    io::{Seek, Write},
    iter::Iterator,
    path::{Path, PathBuf},
    process::exit,
    time::Instant,
};

use walkdir::{DirEntry, WalkDir};
use zip::write::FileOptions;

fn main() {
    std::process::exit(real_main());
}

const METHOD_STORED: Option<zip::CompressionMethod> = Some(zip::CompressionMethod::Stored);

#[cfg(any(
    feature = "deflate",
    feature = "deflate-miniz",
    feature = "deflate-zlib"
))]
const METHOD_DEFLATED: Option<zip::CompressionMethod> = Some(zip::CompressionMethod::Deflated);
#[cfg(not(any(
    feature = "deflate",
    feature = "deflate-miniz",
    feature = "deflate-zlib"
)))]
const METHOD_DEFLATED: Option<zip::CompressionMethod> = None;

#[cfg(feature = "bzip2")]
const METHOD_BZIP2: Option<zip::CompressionMethod> = Some(zip::CompressionMethod::Bzip2);
#[cfg(not(feature = "bzip2"))]
const METHOD_BZIP2: Option<zip::CompressionMethod> = None;

#[cfg(feature = "zstd")]
const METHOD_ZSTD: Option<zip::CompressionMethod> = Some(zip::CompressionMethod::Zstd);
#[cfg(not(feature = "zstd"))]
const METHOD_ZSTD: Option<zip::CompressionMethod> = None;

fn real_main() -> i32 {
    let args: Vec<_> = std::env::args().collect();

    if args.len() < 2 {
        println!(
            "Usage: {} <source_directory> <destination_zipfile>",
            args[0]
        );
        return 1;
    }
    let mut src_dir = args[1].to_string();
    if src_dir.ends_with("/") || src_dir.ends_with("\\") {
        src_dir.remove(src_dir.len() - 1);
    }

    let dst_file = if args.len() < 3 {
        format!("{}.zip", src_dir)
    } else {
        args[2].to_string()
    };
    // let src_dir = src_dir.display().to_string();
    for &method in [METHOD_STORED, METHOD_DEFLATED, METHOD_BZIP2, METHOD_ZSTD].iter() {
        if method.is_none() {
            continue;
        }
        match doit(&src_dir, &dst_file, method.unwrap()) {
            Ok(_) => {
                println!("done: {src_dir} written to {dst_file}");
                exit(0)
            }

            Err(e) => {
                println!("Error: {e:?}");
                exit(1)
            }
        }
    }
    0
}

fn zip_dir<T>(
    it: &mut dyn Iterator<Item = DirEntry>,
    prefix: &str,
    writer: T,
    method: zip::CompressionMethod,
) -> zip::result::ZipResult<()>
where
    T: Write + Seek,
{
    let mut zip = zip::ZipWriter::new(writer);
    zip.add_directory(prefix, Default::default())?;

    let options = FileOptions::default()
        .compression_method(method)
        .unix_permissions(0o755);

    let start = Instant::now();
    for entry in it {
        let path = entry.path();
        let name = path;
        // Write file or directory explicitly. Some unzip tools unzip files with directory paths correctly, some do not!
        if path.is_file() {
            // println!("adding file {path:?} as {name:?} ...");
            #[allow(deprecated)]
            // Not deprecated method force the use of string which results into corrupt path if "\" are used instead of "/" (e.g. on Windows)
            zip.start_file_from_path(name, options)?;
            zip.write_all(fs::read(name).unwrap_or_default().as_slice())?;
        } else if !name.as_os_str().is_empty() {
            // println!("adding dir {path:?} as {name:?} ...");
            #[allow(deprecated)]
            zip.add_directory_from_path(name, options)?;
        }
    }
    let duration = start.elapsed();
    eprintln!("Duration: {:#?}", &duration);

    zip.finish()?;
    Result::Ok(())
}

fn zip_file(zip_name: &str, tozip_path: &PathBuf) -> zip::result::ZipResult<()> {
    let path = std::path::Path::new(zip_name);
    let file = std::fs::File::create(path).unwrap();

    let mut zip = zip::ZipWriter::new(file);

    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o755);

    zip.start_file(
        tozip_path.file_name().map(|s| s.to_str().unwrap()).unwrap(),
        options,
    )?;
    zip.write_all(
        &fs::read(tozip_path)
            .expect(&format!("File to zip {:?} should be readable.", tozip_path))
            .as_slice(),
    )?;
    zip.finish()?;
    Ok(())
}


fn doit(
    src_dir: &str,
    dst_file: &str,
    method: zip::CompressionMethod,
) -> zip::result::ZipResult<()> {
    if !Path::new(src_dir).is_dir() {
        return zip_file(dst_file, &PathBuf::from(src_dir));
    }

    let path = Path::new(dst_file);
    let file = File::create(path).unwrap();

    let walkdir = WalkDir::new(src_dir);
    let it = walkdir.into_iter();

    zip_dir(&mut it.filter_map(|e| e.ok()), src_dir, file, method)?;

    Ok(())
}
