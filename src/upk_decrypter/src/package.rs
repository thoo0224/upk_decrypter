#![allow(non_upper_case_globals)]

use std::arch;
use std::cell::RefCell;
use std::io::SeekFrom;
use std::ops::Deref;
use std::rc::Rc;

use crate::archive::{FArchive, FByteArchive, UESerializable, read_array, read_serializable_array};
use crate::compression::FCompressedChunk;
use crate::encryption::FAesKey;
use crate::file::GameFile;
use crate::Result;

const PACKAGE_MAGIC: u32 = 0x9E2A83C1;

pub const PKG_Cooked: u32 = 0x00000008;
pub const PKG_StoreCompressed: u32 = 0x02000000;

pub const COMPRESS_None: u32 = 0x00;
pub const COMPRESS_ZLIB: u32 = 0x01;
pub const COMPRESS_GZIP: u32 = 0x02;

#[derive(Debug)]
pub enum ECompressionFlags {
    None,
    Zlib,
    Gzip
}

impl Default for ECompressionFlags {
    fn default() -> Self {
        Self::None
    }
}

impl From<u32> for ECompressionFlags{
    fn from(val: u32) -> Self {
        match val {
            COMPRESS_ZLIB => Self::Zlib,
            COMPRESS_GZIP => Self::Gzip,
            COMPRESS_None | _ => Self::None
        }
    }
}

#[derive(Debug)]
pub struct UnPackage<File: GameFile> {
    pub file: File,
    pub keys: Rc<RefCell<Vec<FAesKey>>>,
    pub summary: FPackageFileSummary
}

impl<File> UnPackage<File>
where File : GameFile {

    pub fn new(file: File, keys: Rc<RefCell<Vec<FAesKey>>>) -> Self {
        Self {
            file: file,
            keys,
            summary: FPackageFileSummary::default()
        }
    }

    pub fn load(&mut self) -> Result<()> {
        log::info!("loading package {}", self.file.get_filename());

        let data = self.file.read();
        let mut archive = FByteArchive::new(data);
        FPackageFileSummary::serialize(&mut self.summary, &mut archive)?;
        let compressed_chunks = self.decrypt(&mut archive)?;
        self.decompress(&mut archive)?;

        Ok(())
    }
    
    pub fn decrypt(&mut self, archive: &mut FByteArchive) -> Result<Vec<FCompressedChunk>> {
        let summary = &self.summary;
        archive.seek(SeekFrom::Start(self.summary.name_offset as u64))?;

        let encrypted_size = (summary.header_size - summary.garbage_size - summary.name_offset + 15) & !15;
        let keys = self.keys.deref().borrow();
        let main_key = keys.first().unwrap();

        log::info!("Decrypting package with: {}", main_key.to_hex());
        let decrypted = main_key.decrypt(archive, summary.name_offset as u64, encrypted_size as usize)?;

        let mut header_archive = FByteArchive::new(decrypted);
        header_archive.seek(SeekFrom::Start(self.summary.compression_chunkinfo_offset as u64))?;

        read_serializable_array(&mut header_archive)
    }

    pub fn decompress(&mut self, archive: &mut FByteArchive) -> Result<()> {
        archive.seek(SeekFrom::Start(self.summary.compression_chunkinfo_offset as u64))?;
        let len = archive.read_i32()?;

        Ok(())
    }

}

#[derive(Debug, Default)]
pub struct FGuid {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
}

#[derive(Debug, Default)]
pub struct FGenerationInfo {
    pub export_count: i32,
    pub name_count: i32,
    pub net_object_count: i32
}

impl UESerializable for FGenerationInfo {
    type Item = FGenerationInfo;

    fn serialize<Ar: FArchive>(item: &mut Self::Item, archive: &mut Ar) -> Result<()> {
        item.export_count = archive.read_i32()?;
        item.name_count = archive.read_i32()?;
        item.net_object_count = archive.read_i32()?;

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct FPackageFileSummary {
    pub magic: u32,
    pub file_version: u16,
    pub licensee_version: u16,
    pub header_size: i32,
    pub package_group: String,
    pub package_flags: u32,
    pub name_count: i32,
    pub name_offset: i32,
    pub export_count: i32,
    pub export_offset: i32,
    pub import_count: i32,
    pub import_offset: i32,
    pub depends_offset: i32,
    pub guid: FGuid,
    pub generations: Vec<FGenerationInfo>,
    pub engine_version: i32,
    pub cooker_version: i32,
    pub compression_flags: ECompressionFlags,
    pub compressed_chunks: Vec<FCompressedChunk>,
    pub additional_packages_to_cook: Vec<String>,
    pub unknown_structs: i32,
    pub garbage_size: i32,
    pub compression_chunkinfo_offset: i32,
    pub last_block_size: i32
}

impl UESerializable for FPackageFileSummary {
    type Item = FPackageFileSummary;

    // todo: minimal serialization for saving only
    fn serialize<Ar: FArchive>(val: &mut Self::Item, archive: &mut Ar) -> Result<()> {
        val.magic = archive.read_u32()?;
        if val.magic != PACKAGE_MAGIC {
            panic!("Invalid file magic. Magic = {} PACKAGE_MAGIC = {}", val.magic, PACKAGE_MAGIC);
        }

        val.file_version = archive.read_u16()?;
        val.licensee_version = archive.read_u16()?;
        val.header_size = archive.read_i32()?;
        val.package_group = archive.read_fstring()?;
        val.package_flags = archive.read_u32()?;
        val.name_count = archive.read_i32()?;
        val.name_offset = archive.read_i32()?;
        val.export_count = archive.read_i32()?;
        val.export_offset = archive.read_i32()?;
        val.import_count = archive.read_i32()?;
        val.import_offset = archive.read_i32()?;
        val.depends_offset = archive.read_i32()?;
        archive.seek(SeekFrom::Current(4 * 4))?;

        archive.read_existing_guid(&mut val.guid)?;
        val.generations = read_serializable_array(archive)?;
        val.engine_version = archive.read_i32()?;
        val.cooker_version = archive.read_i32()?;
        val.compression_flags = ECompressionFlags::from(archive.read_u32()?);
        val.compressed_chunks = read_serializable_array(archive)?;
        archive.seek(SeekFrom::Current(4))?;

        val.additional_packages_to_cook = read_array(archive, |ar| ar.read_fstring().unwrap())?;
        val.unknown_structs = archive.read_i32()?;
        for _ in 0..val.unknown_structs {
            archive.seek(SeekFrom::Current(4 * 5))?;
            read_array(archive, |ar| ar.read_i32())?;
        }

        val.garbage_size = archive.read_i32()?;
        val.compression_chunkinfo_offset = archive.read_i32()?;
        val.last_block_size = archive.read_i32()?;

        Ok(())
    }
}