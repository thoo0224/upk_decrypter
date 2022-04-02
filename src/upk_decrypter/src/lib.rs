use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::fmt;

pub mod package;
pub mod file;
pub mod encryption;
pub mod compression;
mod archive;

use encryption::FAesKey;
use file::{OsGameFile, GameFile};
use package::UnPackage;

pub(crate) type Result<Type> = std::result::Result<Type, Box<dyn std::error::Error>>;

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
    keys: Rc<RefCell<Vec<FAesKey>>>,
    pub files: Vec<OsGameFile>,
    output: PathBuf,
    input: PathBuf,
}

impl FileProvider for DefaultFileProvider {
    type GameFileType = OsGameFile;

    fn add_faes_key(&mut self, key: FAesKey) {
        self.keys.clone().borrow_mut().push(key);
    }
}

impl DefaultFileProvider {

    pub fn new(output_dir: &str, input_dir: &str) -> Self {
        Self {
            keys: Rc::new(RefCell::new(Vec::new())),
            files: Vec::new(),
            output: PathBuf::from(output_dir),
            input: PathBuf::from(input_dir)
        }
    }


    pub fn scan_files(&mut self) -> Result<()> {
        let paths = std::fs::read_dir(&self.input)?;
        self.files = paths.into_iter()
            .map(|entry| {
                let path = entry.unwrap().path();
                return OsGameFile::new(path);
            })
            .filter(|file| file.extension == "upk")
            .collect();
        
        log::info!("scanned input directory, found {} packages", self.files.len());
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
            None => panic!("Package not found.")
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