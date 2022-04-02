use crate::archive::UESerializable;
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