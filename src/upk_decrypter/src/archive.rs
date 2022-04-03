use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use crate::package::FGuid;
use crate::ParserError;
use crate::Result;

pub trait UESerializable {
    type Item;

    fn serialize<Ar>(item: &mut Self::Item, archive: &mut Ar) -> Result<()>
    where Ar: FArchive;
}

pub trait FArchive {

    fn read_bytes(&mut self, buffer: &mut [u8]) -> Result<()>;
    fn read_bytes_vec(&mut self, buffer: &mut Vec<u8>) -> Result<()>;

    fn write_all(&mut self, buf: &[u8]) -> Result<()>;

    fn get_mut(&mut self) -> &mut Vec<u8>; // FIX

    fn seek(&mut self, from: SeekFrom) -> Result<u64>;

    fn len(&mut self) -> usize;

    fn read<Type, const SIZE: usize>(&mut self) -> Result<Type> {
        unsafe {
            let size = std::mem::size_of::<Type>();
            assert!(SIZE == size, "invalid size. SIZE: {} std::mem::size_of: {}", SIZE, size);

            let mut buffer = [0u8; SIZE];
            self.read_bytes(&mut buffer)?;

            Ok(std::mem::transmute_copy::<[u8; SIZE], Type>(&buffer))
        }
    }

    #[inline(always)]
    fn read_i64(&mut self) -> Result<i64> {
        self.read::<i64, 8>()
    }

    #[inline(always)]
    fn read_i32(&mut self) -> Result<i32> {
        self.read::<i32, 4>()
    }

    #[inline(always)]
    fn read_16(&mut self) -> Result<i16> {
        self.read::<i16, 2>()
    }

    #[inline(always)]
    fn read_u32(&mut self) -> Result<u32> {
        self.read::<u32, 4>()
    }

    #[inline(always)]
    fn read_u16(&mut self) -> Result<u16> {
        self.read::<u16, 2>()
    }

    #[inline(always)]
    fn read_u8(&mut self) -> Result<u8> {
        self.read::<u8, 1>()
    }

    fn read_fstring(&mut self) -> Result<String> {
        let length = self.read_i32()?;
        if length == 0 {
            return Ok(String::from(""));
        }

        if length < 0  {
            if length == i32::MIN {
                panic!("Archive is corrupted.")
            }

            let len = -length * 2;
            let mut buffer: Vec<u8> = vec![0; usize::try_from(len)?];
            self.read_bytes_vec(&mut buffer)?;

            //return Ok(String::from_utf8(buffer)?);
            panic!("Unicode FString's are not supported yet.");
        }

        let mut buffer = vec![0u8; usize::try_from(length)?];
        self.read_bytes_vec(&mut buffer)?;

        Ok(String::from_utf8(buffer)?)
    }

    fn read_guid(&mut self) -> Result<FGuid> {
        let mut guid = FGuid { a: 0,  b: 0, c: 0, d: 0 };
        self.read_existing_guid(&mut guid)?;
        
        Ok(guid)
    }

    fn read_existing_guid(&mut self, guid: &mut FGuid) -> Result<()> {
        guid.a = self.read_u32()?;
        guid.b = self.read_u32()?;
        guid.c = self.read_u32()?;
        guid.d = self.read_u32()?;

        Ok(())
    }

}

#[allow(dead_code)]
pub fn read_array<T, Ar, F>(archive: &mut Ar, serialize: F) -> Result<Vec<T>>
where F: Fn(&mut Ar) -> T, Ar: FArchive {
    let length = archive.read_i32()?;
    let mut result: Vec<T> = Vec::with_capacity(usize::try_from(length)?);
    for _ in 0..length {
        let val = serialize(archive);
        result.push(val);
    }

    Ok(result)
}

pub fn read_serializable_array<T, Ar>(archive: &mut Ar) -> Result<Vec<T>>
where Ar: FArchive, T: UESerializable<Item = T> + Default {
    let length = archive.read_i32()?;

    read_sized_serializable_array(archive, length)
}

pub fn read_sized_serializable_array<T, Ar>(archive: &mut Ar, length: i32) -> Result<Vec<T>>
where Ar: FArchive, T: UESerializable<Item = T> + Default {
    if length < 0 {
        return Err(Box::new(ParserError::new("Invalid TArray size")));
    }

    let mut result: Vec<T> = Vec::with_capacity(usize::try_from(length)?);
    for _ in 0..length {
        let mut item = T::default();
        T::serialize(&mut item, archive)?;

        result.push(item);
    }

    Ok(result)
}

pub fn read_serializable<T, Ar>(archive: &mut Ar) -> Result<T> 
where Ar: FArchive, T: UESerializable<Item = T> + Default {
    let mut result = T::default();
    T::serialize(&mut result, archive)?;

    Ok(result)
}

pub struct FByteArchive {
    pub cursor: Cursor<Vec<u8>>,
    pub size: usize
}

impl FByteArchive {

    pub fn new(data: Vec<u8>) -> Self {
        let size = data.len();
        Self {
            cursor: Cursor::new(data),
            size
        }
    }

    pub(crate) fn replace_cursor(&mut self, cursor: Cursor<Vec<u8>>) {
        self.cursor = cursor;
    }

}

impl FArchive for FByteArchive {

    fn read_bytes(&mut self, buffer: &mut [u8]) -> Result<()> {
        self.cursor.read_exact(buffer)?;
        Ok(())
    }

    fn read_bytes_vec(&mut self, buffer: &mut Vec<u8>) -> Result<()> {
        self.cursor.read_exact(buffer)?;
        Ok(())
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.cursor.write_all(buf)?;
        Ok(())
    }

    fn seek(&mut self, from: SeekFrom) -> Result<u64> {
        let result = self.cursor.seek(from)?;
        Ok(result)
    }

    fn len(&mut self) -> usize {
        self.size
    }

    fn get_mut(&mut self) -> &mut Vec<u8> {
        self.cursor.get_mut()
    }
 
}