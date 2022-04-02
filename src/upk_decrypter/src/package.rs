#![allow(non_upper_case_globals)]

use miniz_oxide::inflate::decompress_to_vec_zlib;

use std::cell::RefCell;
use std::io::{SeekFrom, Seek, Cursor, Write};
use std::ops::Deref;
use std::rc::Rc;

use crate::archive::{FArchive, FByteArchive, UESerializable, read_array, read_serializable_array, read_serializable};
use crate::compression::{FCompressedChunk, FCompressedChunkHeader, FCompressedChunkBlock};
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

        let encrypted_size = (self.summary.header_size - self.summary.garbage_size - self.summary.name_offset + 15) & !15;
        self.decrypt(&mut archive, encrypted_size as usize)?;
        self.decompress(&mut archive, encrypted_size as usize)?;

        Ok(())
    }

    pub fn save(&mut self) -> Result<()> {
        

        Ok(())
    }
    
    fn decrypt(&mut self, archive: &mut FByteArchive, encrypted_size: usize) -> Result<()> {
        let summary = &self.summary;
        archive.seek(SeekFrom::Start(self.summary.name_offset as u64))?;

        let keys = self.keys.deref().borrow();
        let main_key = keys.first().unwrap();

        log::info!("decrypting package with key: {}", main_key.to_hex());
        main_key.decrypt(archive, summary.name_offset as u64, encrypted_size as usize)?;

        Ok(())
    }

    fn decompress(&mut self, archive: &mut FByteArchive, encrypted_size: usize) -> Result<()> {
        let header_end = self.summary.name_offset as usize + self.summary.compression_chunkinfo_offset as usize;
        archive.seek(SeekFrom::Start(header_end as u64))?;
        let compressed_chunks: Vec<FCompressedChunk> = read_serializable_array(archive)?;

        let result: Vec<u8> = vec![0u8; self.summary.name_offset as usize + encrypted_size]; // lol make this better
        let mut result_cursor = Cursor::new(result);

        let header = &archive.get_mut()[0..header_end];
        result_cursor.get_mut()[0..header_end].copy_from_slice(header);

        for chunk in compressed_chunks {
            archive.seek(SeekFrom::Start(chunk.compressed_offset as u64))?;

            let header: FCompressedChunkHeader = read_serializable(archive)?;
            let mut blocks: Vec<FCompressedChunkBlock> = vec![];
            let mut total_block_size = 0;

            while total_block_size < header.summary.uncompressed_size {
                let block: FCompressedChunkBlock = read_serializable(archive)?;
                total_block_size += block.uncompressed_size;
                blocks.push(block);
            }

            result_cursor.seek(SeekFrom::Start(chunk.uncompressed_offset as u64))?;
            for block in blocks {
                let mut compressed_data = vec![0u8; block.compressed_size as usize];
                archive.read_bytes(&mut compressed_data)?; // todo: optimize

                let decompressed = decompress_to_vec_zlib(compressed_data.as_slice()).unwrap();
                log::info!("decompressed block of {} bytes", decompressed.len());

                result_cursor.write_all(decompressed.as_slice())?;
            }
        }

        archive.replace_cursor(result_cursor);
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