use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::fmt;

pub mod package;
pub mod file;
pub mod encryption;
pub mod compression;
mod archive;

use file::{OsGameFile, GameFile};
use encryption::FAesKey;
use package::UnPackage;

pub type Result<Type> = std::result::Result<Type, Box<dyn std::error::Error>>;

#[derive(Debug)]
#[allow(dead_code)]
struct ParserError {
    message: String
}

impl ParserError {
    pub fn new(message: &str) -> Self {
        Self{
            message: message.to_owned()
        }
    }
}

impl fmt::Display for ParserError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = self.message.as_str();
        write!(formatter, "{}", message)
    }
}

impl std::error::Error for ParserError {
}

pub trait FileProvider { 
    type GameFileType;

    fn add_faes_key(&mut self, key: FAesKey);
}

pub struct StreamedFileProvider { } // TODO: Low priority

#[allow(dead_code)]
pub struct DefaultFileProvider {
    keys: Arc<Mutex<Vec<FAesKey>>>,
    pub files: Vec<OsGameFile>,
    output: PathBuf,
    input: PathBuf,
}

impl FileProvider for DefaultFileProvider {
    type GameFileType = OsGameFile;

    fn add_faes_key(&mut self, key: FAesKey) {
        //self.keys.clone().borrow_mut().push(key);
        let mut keys = self.keys.lock().unwrap();
        keys.push(key);
    }
}

impl DefaultFileProvider {

    pub fn new(output_dir: &str, input_dir: &str) -> Self {
        Self {
            keys: Arc::new(Mutex::new(Vec::new())),
            files: Vec::new(),
            output: PathBuf::from(output_dir),
            input: PathBuf::from(input_dir)
        }
    }


    pub fn scan_files(&mut self) -> Result<()> {
        self.scan_files_with_pattern("*.upk")
    }

    pub fn scan_files_with_pattern(&mut self, pattern: &str) -> Result<()> {
        let mut path = PathBuf::from(&self.input);
        path.push(pattern);

        for entry in glob::glob(path.as_os_str().to_str().unwrap()).unwrap() {
            if let Ok(path) = entry {
                self.files.push(OsGameFile::new(path));
            }
        }

        Ok(())
    }

    pub fn load_package(&mut self, name: &str) -> Result<UnPackage<OsGameFile>> {
        let mut package = self.get_package(name)?;
        package.load()?;

        Ok(package)
    }

    pub fn save_package(&self, name: &str) -> Result<UnPackage<OsGameFile>> {
        let mut package = self.get_package(name)?;
        let mut path = PathBuf::new();
        path.push(self.output.as_os_str().to_str().unwrap());
        path.push(package.file.get_filename().to_string().as_str());

        package.save(path)?;

        Ok(package)
    }

    fn get_package(&self, name: &str) -> Result<UnPackage<OsGameFile>> {
        let file = match self.find_game_file(name) {
            Some(val) => val,
            None => return Err(Box::new(ParserError::new("Package not found.")))
        };

        let package = UnPackage::<OsGameFile>::new(file.clone(), self.keys.clone());
        Ok(package)
    }

    pub fn find_game_file(&self, name: &str) -> Option<&OsGameFile> {
        self.files.iter().find(|f| f.file_name.to_lowercase() == name.to_lowercase())
    }

    pub fn get_files(&mut self) -> &Vec<OsGameFile> {
        &self.files
    }

}