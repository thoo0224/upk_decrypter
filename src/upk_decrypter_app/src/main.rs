use clap::{Parser, ArgEnum};
use path_absolutize::*;
use simple_logger::SimpleLogger;
use stopwatch::Stopwatch;
use threadpool::ThreadPool;

use core::panic;
use std::io::{BufReader, BufRead};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::fs::File;

use upk_decrypter::{DefaultFileProvider, FileProvider};
use upk_decrypter::encryption::FAesKey;
use upk_decrypter::file::GameFile;
use upk_decrypter::Result;

mod epic;
use epic::find_rocketleague_dir;

#[derive(Debug, Copy, Clone, ArgEnum, PartialEq)]
enum FileProviderType {
    Files,
    Streamed
}

impl FileProviderType {
    pub fn is_physical(&self) -> bool {
        matches!(self, FileProviderType::Files)
    }
}

#[derive(Parser, Debug)]
#[clap()]
struct Args {
    #[clap(short, long)] // todo: detect rocket league directory
    input: Option<String>,

    #[clap(short, long, default_value = "./out")]
    output: String,

    #[clap(short, long, default_value = "C:\\Users\\Thoma\\Downloads\\keys.txt")]
    keys: String,

    #[clap(short, long)]
    threads: Option<usize>,

    #[clap(short, long, arg_enum, default_value = "files")]
    provider: FileProviderType
}

fn main() -> Result<()> {
    SimpleLogger::new().init()?;
    let mut args = Args::parse();
    if !args.provider.is_physical() {
        panic!("StreamedFileProvider is currently not supported.");
    }

    validate_input(&mut args)?;

    let mut file_provider = DefaultFileProvider::new(&args.output, &args.input.unwrap());
    let files_found = file_provider.scan_files_with_pattern("*_T_SF.upk")?;
    log::info!("scanned directory, found {} files", files_found);

    let keys = load_aes_keys(&args.keys)?;
    let num_keys = keys.len();
    for key in keys {
        file_provider.add_faes_key(key);
    }
    log::info!("loaded {} aes keys", num_keys);

    let processors = match args.threads {
        Some(val) => val,
        None => num_cpus::get(),
    };

    let thread_pool = ThreadPool::new(processors);
    log::info!("running with {} threads", processors);

    let mut sw = Stopwatch::start_new();
    let files = file_provider.files.clone();
    let arc = Arc::new(file_provider);
    for file in files {
        let provider = arc.clone();
        thread_pool.execute(move || {
            if let Ok(_) = provider.save_package(file.get_filename().as_str()) {
                log::info!("saved package {}", file.file_name)
            }
        });
    }

    thread_pool.join();
    sw.stop();

    log::info!("Finished in {}ms", sw.elapsed().as_millis());

    Ok(())
}

fn validate_input(args: &mut Args) -> Result<()> {
    args.input = match args.input.clone() {
        Some(val) => Some(val),
        None => {
            let folder = find_rocketleague_dir()?;
            let p: PathBuf = [&folder, "TAGame", "CookedPCConsole"].iter().collect();
            Some(p.to_str().unwrap().to_string())
        },
    };

    let provider_name = match args.provider {
        FileProviderType::Files => "DefaultFileProvider",
        FileProviderType::Streamed => "StreamedFileProvider",
    };
    log::info!("using provider: {}", provider_name);

    if args.provider.is_physical() {
        let temp = args.input.clone().unwrap();
        let input_path = Path::new(&temp);
        create_if_not_exists(&input_path)?;
        log::info!("using input directory: {:?}", input_path.absolutize().unwrap());
    }

    let output_path = Path::new(&args.output);
    create_if_not_exists(&output_path)?;
    log::info!("using output directory: {:?}", output_path.absolutize().unwrap());

    Ok(())
}

fn create_if_not_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir(path)?;
    }

    Ok(())
}

fn load_aes_keys(path: &str) -> Result<Vec<FAesKey>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let keys = reader.lines()
        .into_iter()
        .map(|x| x.unwrap())
        .map(|line| FAesKey::from_base64(&line).unwrap())
        .collect();

    Ok(keys)
}