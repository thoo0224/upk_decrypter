use miniz_oxide::inflate::decompress_to_vec_zlib;

use std::io::{Cursor, SeekFrom, Seek, Write};

use crate::archive::{UESerializable, read_serializable, FArchive};
use crate::Result;

#[derive(Debug, Default)]
pub struct FCompressedChunk {
    pub uncompressed_offset: i32,
    pub uncompressed_size: i32,
    pub compressed_offset: i32,
    pub compressed_size: i32
}

impl UESerializable for FCompressedChunk {
    type Item = FCompressedChunk;

    fn serialize<Ar>(item: &mut Self::Item, archive: &mut Ar) -> Result<()>
    where Ar: crate::archive::FArchive {
        item.uncompressed_offset = i32::try_from(archive.read_i64()?)?;
        item.uncompressed_size = archive.read_i32()?;

        item.compressed_offset = i32::try_from(archive.read_i64()?)?;
        item.compressed_size = archive.read_i32()?;

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct FCompressedChunkBlock {
    pub compressed_size: i32,
    pub uncompressed_size: i32
}

impl UESerializable for FCompressedChunkBlock {
    type Item = FCompressedChunkBlock;

    fn serialize<Ar>(item: &mut Self::Item, archive: &mut Ar) -> Result<()>
    where Ar: crate::archive::FArchive {
        item.compressed_size = archive.read_i32()?;
        item.uncompressed_size = archive.read_i32()?;

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct FCompressedChunkHeader {
    pub tag: i32,
    pub block_size: i32,
    pub summary: FCompressedChunkBlock
}

impl UESerializable for FCompressedChunkHeader {
    type Item = FCompressedChunkHeader;

    fn serialize<Ar>(item: &mut Self::Item, archive: &mut Ar) -> Result<()>
    where Ar: crate::archive::FArchive {
        item.tag = archive.read_i32()?;
        item.block_size = archive.read_i32()?;
        item.summary = read_serializable(archive)?;

        Ok(())
    }
}

pub fn decompress<Ar>(archive: &mut Ar, cursor: &mut Cursor<Vec<u8>>, compressed_chunks: &[FCompressedChunk]) -> Result<()>
where Ar: FArchive {
    for chunk in compressed_chunks {
        archive.seek(SeekFrom::Start(u64::try_from(chunk.compressed_offset)?))?;

        let header: FCompressedChunkHeader = read_serializable(archive)?;
        let mut blocks: Vec<FCompressedChunkBlock> = vec![];
        let mut total_block_size = 0;

        while total_block_size < header.summary.uncompressed_size {
            let block: FCompressedChunkBlock = read_serializable(archive)?;
            total_block_size += block.uncompressed_size;
            blocks.push(block);
        }

        cursor.seek(SeekFrom::Start(usize::try_from(chunk.uncompressed_offset)? as u64))?;
        for block in blocks {
            let mut compressed_data = vec![0u8; usize::try_from(block.compressed_size)?];
            archive.read_bytes(&mut compressed_data)?; // todo: optimize

            let decompressed = decompress_to_vec_zlib(compressed_data.as_slice()).unwrap();
            cursor.write_all(decompressed.as_slice())?;
        }

        //log::info!("decompressed chunk of {} bytes", 0)
    }

    Ok(())
}