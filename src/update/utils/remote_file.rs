use std::{
    cmp::min,
    io::{Read, Seek},
    time::Duration,
    vec,
};

use chrono::{DateTime, Utc};
use reqwest::header::{ACCEPT_RANGES, LAST_MODIFIED, RANGE};
use tokio::runtime::Handle;
use tracing::debug;

use super::super::error::Error;

const CHUNK_SIZE: usize = 100_000_000;

pub struct RemoteFile {
    pub last_modified: DateTime<Utc>,
    pub content_length: u64,

    client: reqwest::Client,
    url: String,
}

pub struct RemoteFileReader {
    pub last_modified: DateTime<Utc>,
    pub content_length: u64,

    client: reqwest::Client,
    url: String,
    position: u64,
    chunk_offset: u64,
    chunk: Vec<u8>,
}

// Use blocking client

impl RemoteFile {
    pub async fn new(url: String) -> Result<Self, Error> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(3600))
            .build()
            .map_err(|req_error| Error::Download { req_error })?;

        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|req_error| Error::Download { req_error })?;

        let content_length = resp
            .content_length()
            .ok_or(Error::MissingContentLengthHeader)?;

        let accept_ranges_str = resp
            .headers()
            .get(ACCEPT_RANGES)
            .ok_or(Error::MissingAcceptRangesHeader)?
            .to_str()
            .map_err(|head_error| Error::InvalidAcceptRangesHeader { head_error })?;

        if accept_ranges_str != "bytes" {
            return Err(Error::InvalidAcceptRangesValue {
                value: accept_ranges_str.to_string(),
            });
        }

        // Decode Last-Modified header
        let last_modified_str = resp
            .headers()
            .get(LAST_MODIFIED)
            .ok_or(Error::MissingLastModifiedHeader)?
            .to_str()
            .map_err(|head_error| Error::InvalidLastModifiedHeader { head_error })?;

        let last_modified = DateTime::parse_from_rfc2822(last_modified_str)
            .map_err(|date_error| Error::InvalidLastModifiedDate { date_error })?;

        let last_modified = last_modified.with_timezone(&Utc);

        Ok(RemoteFile {
            last_modified,
            content_length,

            client,
            url,
        })
    }

    pub fn to_reader(&self) -> RemoteFileReader {
        RemoteFileReader {
            last_modified: self.last_modified,
            content_length: self.content_length,

            client: self.client.clone(),
            url: self.url.clone(),
            position: 0,
            chunk_offset: 0,
            chunk: vec![],
        }
    }
}

impl Read for RemoteFileReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Fill buf with data from position to position + buf.len()
        debug!(
            "Reading from {} to {}",
            self.position,
            self.position + buf.len() as u64
        );

        if self.chunk_offset <= self.position
            && self.position + buf.len() as u64 <= self.chunk_offset + self.chunk.len() as u64
        {
            let start = (self.position - self.chunk_offset) as usize;
            let end = start + buf.len();

            buf.copy_from_slice(&self.chunk[start..end]);

            self.position += buf.len() as u64;

            return Ok(buf.len());
        }

        Handle::current().block_on(async {
            let resp = self
                .client
                .get(&self.url)
                .header(
                    RANGE,
                    format!(
                        "bytes={}-{}",
                        self.position,
                        self.position + CHUNK_SIZE as u64 - 1
                    ),
                )
                .send()
                .await
                .map_err(|req_error| std::io::Error::new(std::io::ErrorKind::Other, req_error))?;

            let bytes = resp
                .bytes()
                .await
                .map_err(|req_error| std::io::Error::new(std::io::ErrorKind::Other, req_error))?;

            self.chunk = bytes.to_vec();
            self.chunk_offset = self.position;

            let len = min(buf.len(), self.chunk.len());

            buf.copy_from_slice(&self.chunk[0..len]);

            self.position += len as u64;

            Ok(len)
        })
    }
}

impl Seek for RemoteFileReader {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        debug!("Seeking from {} to {:#?}", self.position, pos);

        let new_position = match pos {
            std::io::SeekFrom::Start(position) => position as i64,
            std::io::SeekFrom::End(position) => self.content_length as i64 + position,
            std::io::SeekFrom::Current(position) => self.position as i64 + position,
        };

        if new_position < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid seek position",
            ));
        }

        self.position = new_position as u64;

        Ok(self.position)
    }
}
