use clap::{ArgEnum, ArgMatches, Command, command, arg};
use simple_logger::SimpleLogger;
use stopwatch::Stopwatch;
use threadpool::ThreadPool;

use core::panic;
use std::io::{BufReader, BufRead};
use std::path::Path;
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

    pub fn is_physical(self) -> bool {
        matches!(self, FileProviderType::Files)
    }

}

impl std::str::FromStr for FileProviderType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        for variant in Self::value_variants() {
            if variant.to_possible_value().unwrap().matches(s, true) {
                return Ok(*variant);
            }
        }

        Err(format!("Invalid variant: {}", s))
    }
}

fn main() -> Result<()> {
    SimpleLogger::new().init()?;
    let matches = command!()
        .subcommand(get_decrypt_command())
        .get_matches();

    match matches.subcommand() {
        Some(("decrypt", sm)) => decrypt(sm)?,
        _ => todo!(),
    }

    Ok(())
}

fn decrypt(args: &ArgMatches) -> Result<()> {
    let provider_type: FileProviderType = args.value_of_t("provider")?;
    assert!(provider_type.is_physical(), "StreamedFileProvider is currently not supported.");

    let output: String = args.value_of_t("output")?;
    if !Path::new(&output).exists() {
        std::fs::create_dir_all(&output)?;
    }

    let input: String = args.value_of_t("input").unwrap_or(find_rocketleague_dir()?);
    let keys: String = args.value_of_t("keys")?;

    log::info!("using encryption keys file: {}", &keys);
    log::info!("using output directory: {}", &output);
    if provider_type.is_physical() {
        log::info!("using input directory: {}", &input);
    }
    
    let mut file_provider = DefaultFileProvider::new(&output, &input);
    let files_found = file_provider.scan_files_with_pattern("*_T_SF.upk")?;
    log::info!("scanned directory {}, found {} files", &input, files_found);

    let keys = load_aes_keys(&keys)?;
    let num_keys = keys.len();
    for key in keys {
        file_provider.add_faes_key(key);
    }
    log::info!("loaded {} aes keys", num_keys);

    let processors = match args.value_of_t::<usize>("threads") {
        Ok(val) => val,
        Err(_) => num_cpus::get(),
    };

    let thread_pool = ThreadPool::new(processors);
    log::info!("running with {} threads", processors);

    let mut sw = Stopwatch::start_new();
    let files = file_provider.files.clone();
    let arc = Arc::new(file_provider);
    for file in files {
        let provider = arc.clone();
        thread_pool.execute(move || {
            if provider.save_package(file.get_filename().as_str()).is_ok() {
                log::info!("saved package {}", file.file_name);
            }
        });
    }

    thread_pool.join();
    sw.stop();

    log::info!("Finished in {}ms", sw.elapsed().as_millis());
    Ok(())
}

fn get_decrypt_command() -> Command<'static> {
    Command::new("decrypt")
    .about("Decrypts all the upk files in the input directory.")
    .arg(arg!(-i --input <INPUT>).id("input")
        .help("The input directory with all the upk files.")
        .required(false))
    .arg(arg!(-o --output <OUTPUT>).id("output")
        .help("The output directory where all the decrypted files will be written to")
        .default_value("./out")
        .required(false))
    .arg(arg!(-k --keys <KEYS>).id("keys")
        .help("The file with all the encryption keys")
        .required(true)
        .validator(path_exists_validator))
    .arg(arg!(-p --provider <PROVIDER>).id("provider")
        .help("The provider to use for the packages")
        .possible_values(["Files", "Streamed"])
        .default_value("Files")
        .required(false))
    .arg(arg!(-t --threads <THREADS>).id("threads")
        .help("The numbers of threads that will decrypt the packages")
        .required(false))
}

fn load_aes_keys(path: &str) -> Result<Vec<FAesKey>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let keys = reader.lines()
        .into_iter()
        .map(std::result::Result::unwrap)
        .map(|line| FAesKey::from_base64(&line).unwrap())
        .collect();

    Ok(keys)
}

fn path_exists_validator(path: &str) -> std::result::Result<(), String> {
    if !Path::new(path).exists() {
        return Err(String::from("path does not exist!"));
    }

    Ok(())
}