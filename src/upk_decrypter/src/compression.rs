use crate::archive::{UESerializable, read_serializable};
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
        item.uncompressed_offset = archive.read_i64()? as i32;
        item.uncompressed_size = archive.read_i32()?;

        item.compressed_offset = archive.read_i64()? as i32;
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