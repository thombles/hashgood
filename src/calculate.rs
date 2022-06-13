use super::Algorithm;
use crossbeam_channel::bounded;
use crossbeam_channel::Receiver;
use crypto::digest::Digest;
use crypto::md5::Md5;
use crypto::sha1::Sha1;
use crypto::sha2::Sha256;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

pub type CalculateResult = Result<Vec<(Algorithm, Vec<u8>)>, Box<dyn Error>>;

/// For a given path to the input (may be "-" for STDIN), try to obtain a reader for the data within it.
pub fn get_input_reader(input: &Path) -> Result<Box<dyn Read>, String> {
    if input.to_str() == Some("-") {
        // Special case: standard input
        return Ok(Box::new(std::io::stdin()));
    }
    if !input.exists() {
        return Err(format!(
            "The path '{}' does not exist.",
            input.to_string_lossy()
        ));
    }
    if !input.is_file() {
        return Err(format!(
            "The path '{}' is not a regular file.",
            input.to_string_lossy()
        ));
    }
    match File::open(input) {
        Ok(f) => Ok(Box::new(f)),
        Err(e) => Err(format!("File open: {}", e)),
    }
}

/// For the given input stream, calculate all requested digest types
pub fn create_digests(algorithms: &[Algorithm], mut input: Box<dyn Read>) -> CalculateResult {
    let mut senders = vec![];
    let mut handles = vec![];

    if algorithms.contains(&Algorithm::Md5) {
        let (s, r) = bounded::<Arc<Vec<u8>>>(1);
        senders.push(s);
        handles.push(md5_digest(r));
    }
    if algorithms.contains(&Algorithm::Sha1) {
        let (s, r) = bounded::<Arc<Vec<u8>>>(1);
        senders.push(s);
        handles.push(sha1_digest(r));
    }
    if algorithms.contains(&Algorithm::Sha256) {
        let (s, r) = bounded::<Arc<Vec<u8>>>(1);
        senders.push(s);
        handles.push(sha256_digest(r));
    }

    // 64 KB chunks will be read from the input at 64 KB and supplied to all hashing threads at once
    // Right now that could be up to three threads. If CPU-bound, the other threads will mostly block while the slowest one finishes
    const BUF_SIZE: usize = 1024 * 64;
    let mut buf = [0; BUF_SIZE];
    while let Ok(size) = input.read(&mut buf) {
        if size == 0 {
            break;
        } else {
            // Create a shared read-only copy for the hashers to take as input
            // buf is freed up for more reading
            let chunk = Arc::new(buf[0..size].to_vec());
            for s in &senders {
                s.send(chunk.clone())?;
            }
        }
    }
    drop(senders);
    // Once all data has been sent we just have to wait for the digests to fall out
    Ok(handles.into_iter().map(|h| h.join().unwrap()).collect())
}

/// Calculate the md5 digest of some data on the given channel
fn md5_digest(rx: Receiver<Arc<Vec<u8>>>) -> JoinHandle<(Algorithm, Vec<u8>)> {
    thread::spawn(move || {
        let mut md5 = Md5::new();
        while let Ok(chunk) = rx.recv() {
            md5.input(&chunk);
        }
        let mut result = [0; 16];
        md5.result(&mut result);
        (Algorithm::Md5, result.to_vec())
    })
}

/// Calculate the sha1 digest of some data on the given channel
fn sha1_digest(rx: Receiver<Arc<Vec<u8>>>) -> JoinHandle<(Algorithm, Vec<u8>)> {
    thread::spawn(move || {
        let mut sha1 = Sha1::new();
        while let Ok(chunk) = rx.recv() {
            sha1.input(&chunk);
        }
        let mut result = [0; 20];
        sha1.result(&mut result);
        (Algorithm::Sha1, result.to_vec())
    })
}

/// Calculate the sha256 digest of some data on the given channel
fn sha256_digest(rx: Receiver<Arc<Vec<u8>>>) -> JoinHandle<(Algorithm, Vec<u8>)> {
    thread::spawn(move || {
        let mut sha256 = Sha256::new();
        while let Ok(chunk) = rx.recv() {
            sha256.input(&chunk);
        }
        let mut result = [0; 32];
        sha256.result(&mut result);
        (Algorithm::Sha256, result.to_vec())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const SMALL_DATA: [u8; 10] = ['A' as u8; 10];
    // python3 -c 'print ("A"*10, end="", flush=True)' | md5sum
    const SMALL_DATA_MD5: &'static str = "16c52c6e8326c071da771e66dc6e9e57";
    // python3 -c 'print ("A"*10, end="", flush=True)' | sha1sum
    const SMALL_DATA_SHA1: &'static str = "c71613a7386fd67995708464bf0223c0d78225c4";
    // python3 -c 'print ("A"*10, end="", flush=True)' | sha256sum
    const SMALL_DATA_SHA256: &'static str =
        "1d65bf29403e4fb1767522a107c827b8884d16640cf0e3b18c4c1dd107e0d49d";

    const LARGE_DATA: [u8; 1_000_000] = ['B' as u8; 1_000_000];
    // python3 -c 'print ("B"*1000000, end="", flush=True)' | md5sum
    const LARGE_DATA_MD5: &'static str = "9171f6d67a87ca649a702434a03458a1";
    // python3 -c 'print ("B"*1000000, end="", flush=True)' | sha1sum
    const LARGE_DATA_SHA1: &'static str = "cfae4cebfd01884111bdede7cf983626bb249c94";
    // python3 -c 'print ("B"*1000000, end="", flush=True)' | sha256sum
    const LARGE_DATA_SHA256: &'static str =
        "b9193853f7798e92e2f6b82eda336fa7d6fc0fa90fdefe665f372b0bad8cdf8c";

    fn verify_digest(alg: Algorithm, data: &'static [u8], hash: &str) {
        let reader = Cursor::new(&*data);
        let digests = create_digests(&[alg], Box::new(reader)).unwrap();
        assert_eq!(digests.len(), 1);
        assert_eq!(digests[0], (alg, hex::decode(hash).unwrap()));
    }

    /// Assert that digests for all algorithms are calculated correctly for a small piece
    /// of test data (single block).
    #[test]
    fn small_digests() {
        verify_digest(Algorithm::Md5, &SMALL_DATA, &SMALL_DATA_MD5);
        verify_digest(Algorithm::Sha1, &SMALL_DATA, &SMALL_DATA_SHA1);
        verify_digest(Algorithm::Sha256, &SMALL_DATA, &SMALL_DATA_SHA256);
    }

    /// Assert that digests for all algorithms are calculated correctly for a large piece
    /// of test data. For our purposes, "large" means that it spans several of the 64 KB
    /// blocks used to break up the input processing. Using one million bytes instead of
    /// 1 MiB means that the final block will be slightly smaller than the others.
    #[test]
    fn large_digests() {
        verify_digest(Algorithm::Md5, &LARGE_DATA, &LARGE_DATA_MD5);
        verify_digest(Algorithm::Sha1, &LARGE_DATA, &LARGE_DATA_SHA1);
        verify_digest(Algorithm::Sha256, &LARGE_DATA, &LARGE_DATA_SHA256);
    }
}
