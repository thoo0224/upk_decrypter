use clap::{Parser, ArgEnum};
use path_absolutize::*;
use simple_logger::SimpleLogger;

use std::io::{BufReader, BufRead};
use std::path::Path;
use std::fs::File;

use upk_decrypter::{DefaultFileProvider, FileProvider};
use upk_decrypter::encryption::FAesKey;

type Result<Type> = std::result::Result<Type, Box<dyn std::error::Error>>;

#[derive(Debug, Copy, Clone, ArgEnum)]
enum FileProviderType {
    Files,
    Streamed
}

#[derive(Parser, Debug)]
#[clap()]
struct Args {
    #[clap(short, long, default_value = "E:\\Games\\rocketleague\\TAGame\\CookedPCConsole")]
    input: String,

    #[clap(short, long, default_value = "./out")]
    output: String,

    #[clap(short, long, arg_enum, default_value = "files")]
    provider: FileProviderType
}

fn main() -> Result<()> {
    SimpleLogger::new().init()?;

    let args = Args::parse();
    validate_input(&args)?;

    let mut file_provider = DefaultFileProvider::new(&args.output, &args.input);
    file_provider.scan_files()?;

    let keys = load_aes_keys()?;
    for key in keys {
        //log::info!("loaded AES: {}", key.to_hex());
        file_provider.add_faes_key(key);
    }

    let package = file_provider.save_package("Body_Octane_T_SF.upk")?;
    log::info!("{:?}", package);

    Ok(())
}

fn validate_input(args: &Args) -> Result<()> {
    let input_path = Path::new(&args.input);
    let output_path = Path::new(&args.output);
    
    create_if_not_exists(&input_path)?; // check the provider, no need to create the directory if it's streamed
    create_if_not_exists(&output_path)?;

    let provider_name = match args.provider {
        FileProviderType::Files => "DefaultFileProvider",
        FileProviderType::Streamed => "StreamedFileProvider",
    };

    log::info!("using provider: {}", provider_name);
    log::info!("using input path: {:?}", input_path.absolutize().unwrap());
    log::info!("using output path: {:?}", output_path.absolutize().unwrap());

    Ok(())
}

fn create_if_not_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir(path)?;
    }

    Ok(())
}

fn load_aes_keys() -> Result<Vec<FAesKey>> {
    let file = File::open("C:\\Users\\Thoma\\Downloads\\keys.txt")?;
    let reader = BufReader::new(file);
    let keys = reader.lines()
        .into_iter()
        .map(|x| x.unwrap())
        .map(|line| FAesKey::from_base64(&line).unwrap())
        .collect();

    Ok(keys)
}