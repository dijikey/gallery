use std::{
    ffi::OsString,
    fs::{self, DirEntry, File},
    io::{self, BufReader, BufWriter, Write},
    path::PathBuf,
};

use age::secrecy::SecretString;
use bincode::{Decode, Encode};

#[derive(Debug, Clone, Decode, Encode)]
pub enum ContentType {
    Folder(Vec<Content>),
    File(Vec<u8>),
}

#[derive(Debug, Clone, Decode, Encode)]
pub struct Content {
    inner: ContentType,
    pub title: String,
}

impl Content {
    pub fn decrypt(mut self, key: SecretString) -> Result<Self, Error> {
        let identity = age::scrypt::Identity::new(key);

        match self.inner {
            ContentType::File(items) => {
                let encrypted = age::decrypt(&identity, &items)?;
                self.inner = ContentType::File(encrypted);
                Ok(self)
            }
            _ => Err(Error::NullBytes),
        }
    }

    pub fn encrypt(mut self, key: SecretString) -> Result<Self, Error> {
        let identity = age::scrypt::Recipient::new(key);

        match &self.inner {
            ContentType::File(items) => {
                let encrypted = age::encrypt(&identity, &items)?;
                self.inner = ContentType::File(encrypted);
                Ok(self)
            }
            _ => Err(Error::NullBytes),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Machine {
    key: SecretString,
    pub dir: PathBuf,
    to: String,
}

impl Machine {
    pub fn with_key(key: SecretString, dir: PathBuf) -> Self {
        Self {
            key,
            dir,
            to: uuid::Uuid::new_v4().to_string(),
        }
    }

    pub fn decrypt(&self) -> Result<(), Error> {
        let dir = self.dir.parent().unwrap().join(&self.to);
        self.inner_decrypt(self.walk_dir_decode()?, dir)
    }

    fn inner_decrypt(&self, vec: Vec<Content>, dir: PathBuf) -> Result<(), Error> {
        fs::create_dir(&dir)?;

        for content in vec {
            match content.inner {
                ContentType::Folder(contents) => {
                    let dir = dir.join(&content.title);
                    self.inner_decrypt(contents, dir)?;
                }
                ContentType::File(_) => {
                    println!("File decrypt({:?})", content.title);
                    let encrypted = content.decrypt(self.key.clone())?;

                    let dir = dir.join(&encrypted.title);
                    let mut writer = BufWriter::new(File::create(dir)?);

                    if let ContentType::File(bytes) = encrypted.inner {
                        writer.write_all(&bytes)?
                    }
                }
            }
        }

        Ok(())
    }

    pub fn encrypt(&self) -> Result<(), Error> {
        let dir = self.dir.parent().unwrap().join(&self.to);
        self.inner_encrypt(self.walk_dir_encode()?, dir)
    }

    fn inner_encrypt(&self, vec: Vec<Content>, dir: PathBuf) -> Result<(), Error> {
        fs::create_dir(&dir)?;

        let config = bincode::config::standard();
        for content in vec {
            match content.inner {
                ContentType::Folder(contents) => {
                    let dir = dir.join(&uuid::Uuid::new_v4().to_string());
                    self.inner_encrypt(contents, dir)?;
                }
                ContentType::File(_) => {
                    println!("File encrypt({:?})", content.title);
                    let encrypted = content.encrypt(self.key.clone())?;

                    let dir = dir.join(uuid::Uuid::new_v4().to_string());
                    let mut writer = BufWriter::new(File::create(dir)?);
                    bincode::encode_into_std_write(encrypted, &mut writer, config)?;
                }
            }
        }

        Ok(())
    }

    fn walk_dir_decode(&self) -> Result<Vec<Content>, Error> {
        let result = Ok(fs::read_dir(&self.dir)?
            .into_iter()
            .filter_map(|f| match f {
                Ok(v) => Some(v),
                Err(err) => {
                    dbg!(&err);
                    None
                }
            })
            .filter(|f| f.file_type().is_ok())
            .filter_map(|f| {
                if f.file_type().unwrap().is_dir() {
                    let machine = Machine::with_key(self.key.clone(), f.path());
                    let inner = match machine.walk_dir_decode() {
                        Ok(v) => v,
                        Err(_) => return None,
                    };

                    return Some(Content {
                        inner: ContentType::Folder(inner),
                        title: uuid::Uuid::new_v4().to_string(),
                    });
                }

                let file = match File::open(f.path()) {
                    Ok(f) => f,
                    Err(_) => return None,
                };

                let mut reader = BufReader::new(file);
                let config = bincode::config::standard();

                let decode = bincode::decode_from_std_read(&mut reader, config);
                match decode {
                    Ok(c) => Some(c),
                    Err(_) => None,
                }
            })
            .collect());

        result
    }

    fn walk_dir_encode(&self) -> Result<Vec<Content>, Error> {
        Ok(fs::read_dir(&self.dir)?
            .into_iter()
            .filter_map(|f| match f {
                Ok(v) => Some(v),
                Err(err) => {
                    dbg!(&err);
                    None
                }
            })
            .filter(|f| f.file_type().is_ok())
            .filter_map(|e| match self.transmute(e) {
                Ok(v) => Some(v),
                Err(err) => {
                    dbg!(&err);
                    None
                }
            })
            .collect())
    }

    fn transmute(&self, entry: DirEntry) -> Result<Content, Error> {
        let title = match entry.file_name().into_string() {
            Ok(s) => s,
            Err(str) => return Err(Error::InvalidUnicode(str)),
        };

        let r#type = match &entry.file_type().unwrap().is_dir() {
            true => ContentType::Folder(
                Machine::with_key(self.key.clone(), entry.path()).walk_dir_encode()?,
            ),
            false => ContentType::File(fs::read(entry.path())?),
        };

        Ok(Content {
            inner: r#type,
            title,
        })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum Error {
    IOError(io::Error),
    InvalidUnicode(OsString),
    EncryptError(age::EncryptError),
    EncodeError(bincode::error::EncodeError),
    DecryptError(age::DecryptError),
    NullBytes,
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::IOError(value)
    }
}

impl From<age::EncryptError> for Error {
    fn from(value: age::EncryptError) -> Self {
        Self::EncryptError(value)
    }
}

impl From<age::DecryptError> for Error {
    fn from(value: age::DecryptError) -> Self {
        Self::DecryptError(value)
    }
}

impl From<bincode::error::EncodeError> for Error {
    fn from(value: bincode::error::EncodeError) -> Self {
        Self::EncodeError(value)
    }
}
