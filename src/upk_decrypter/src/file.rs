use std::path::{PathBuf, Path};

pub trait GameFile {
    fn read(&self) -> Vec<u8>;
    //fn create_reader(&self) -> FArchive

    fn get_filename(&self) -> &String;
}

#[derive(Debug, Clone)]
pub struct OsGameFile {
    pub file_name: String,
    pub extension: String,
    path: PathBuf,
}

impl GameFile for OsGameFile {

    fn read(&self) -> Vec<u8> {
        std::fs::read(&self.path).unwrap()
    }

    fn get_filename(&self) -> &String {
        &self.file_name
    }

}

impl OsGameFile {
    pub fn new(path_buf: PathBuf) -> Self {
        let path = Path::new(&path_buf);
        let file_name = path.file_name().unwrap().to_os_string().into_string().unwrap();
        let extension = path.extension().unwrap().to_os_string().into_string().unwrap();

        Self {
            file_name,
            extension,
            path: path_buf
        }
    }
}