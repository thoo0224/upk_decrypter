use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::ops::Deref;
use std::path::PathBuf;
use std::rc::Rc;

pub mod package;
pub mod file;
pub mod encryption;
pub mod compression;
mod archive;

use encryption::FAesKey;
use file::OsGameFile;
use package::UnPackage;

pub(crate) type Result<Type> = std::result::Result<Type, Box<dyn std::error::Error>>;

pub trait FileProvider { 
    type GameFileType;

    fn add_faes_key(&mut self, key: FAesKey);
}

pub struct StreamedFileProvider { } // TODO: Low priority

#[allow(dead_code)]
pub struct DefaultFileProvider {
    keys: Rc<RefCell<Vec<FAesKey>>>,
    files: Vec<OsGameFile>,
    output: PathBuf,
    input: PathBuf,
}

impl FileProvider for DefaultFileProvider {
    type GameFileType = OsGameFile;

    fn add_faes_key(&mut self, key: FAesKey) {
        self.keys.deref().borrow_mut().push(key);
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

    pub fn find_game_file(&mut self, name: &str) -> Option<&OsGameFile> {
        self.files.iter().find(|f| f.file_name.to_lowercase() == name.to_lowercase())
    }

    pub fn load_package(&mut self, name: &str) -> Result<UnPackage<OsGameFile>> {
        let file = match self.find_game_file(name) {
            Some(val) => val,
            None => panic!("Package not found.")
        };

        let mut package = UnPackage::<OsGameFile>::new(file.clone(), self.keys.clone());
        package.load()?;

        Ok(package)
    }

}