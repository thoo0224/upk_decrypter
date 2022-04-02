use aes::Aes256;
use block_modes::{BlockMode, Ecb, block_padding::ZeroPadding};

use std::borrow::Borrow;
use std::io::SeekFrom;
use std::ops::Add;

use crate::archive::FArchive;
use crate::Result;

const KEY_SIZE: usize = 32;

#[allow(dead_code)]
#[derive(Debug)] // display to hex
pub struct FAesKey {
    pub(crate) key: [u8; KEY_SIZE]
}

impl FAesKey {

    pub fn from_base64(base64: &str) -> Result<Self> {
        let decoded = base64::decode(base64)?;
        Ok(Self {
            key: decoded.try_into().unwrap()
        })
    } 

    pub fn to_hex(&self) -> String {
        "0x".to_owned().add(&hex::encode(&self.key))
    }

    pub fn decrypt<Ar>(&self, archive: &mut Ar, offset: u64, len: usize) -> Result<()> 
    where Ar: FArchive {
        let cipher = Ecb::<Aes256, ZeroPadding>::new_from_slices(&self.key, Default::default())?;

        archive.seek(SeekFrom::Start(offset))?;
        let mut encrypted = vec![0; len];
        archive.read_bytes_vec(&mut encrypted)?;

        let decrypted = cipher.decrypt(encrypted.as_mut_slice())?;
        log::info!("Decrypted block of {} bytes", decrypted.len());

        archive.seek(SeekFrom::Start(offset))?;
        archive.write_all( decrypted)?;

        Ok(())
    }

}