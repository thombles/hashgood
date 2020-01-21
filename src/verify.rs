use super::{
    Algorithm, CandidateHash, CandidateHashes, Hash, MatchLevel, MessageLevel, Opt, Verification,
    VerificationSource,
};
use clipboard::ClipboardContext;
use clipboard::ClipboardProvider;
use regex::Regex;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::PathBuf;

/// Calculate a list of candidate hashes based on the options specified.
/// If no hash options have been specified returns None.
/// It is assumed to be verified previously that at most one mode has been specified.
pub fn get_candidate_hashes(opt: &Opt) -> Result<Option<CandidateHashes>, String> {
    if let Some(hash_string) = &opt.hash {
        return Ok(Some(get_by_parameter(hash_string)?));
    } else if opt.paste {
        return Ok(Some(get_from_clipboard()?));
    } else if let Some(hash_file) = &opt.hash_file {
        return Ok(Some(get_from_file(hash_file)?));
    }
    Ok(None)
}

/// Generate a candidate hash from the provided command line parameter, or throw an error.
fn get_by_parameter(param: &str) -> Result<CandidateHashes, String> {
    let bytes =
        hex::decode(&param).map_err(|_| "Provided hash is invalid or truncated hex".to_owned())?;
    let alg = Algorithm::from_len(bytes.len())?;
    let candidate = CandidateHash {
        filename: None,
        bytes,
    };
    Ok(CandidateHashes {
        alg,
        hashes: vec![candidate],
        source: VerificationSource::CommandArgument,
    })
}

/// Generate a candidate hash from the system clipboard, or throw an error.
fn get_from_clipboard() -> Result<CandidateHashes, String> {
    let mut ctx: ClipboardContext = match ClipboardProvider::new() {
        Ok(ctx) => ctx,
        Err(e) => return Err(format!("Error getting system clipboard: {}", e)),
    };

    let possible_hash = match ctx.get_contents() {
        Ok(value) => value,
        Err(e) => format!("Error reading from clipboard: {}", e),
    };

    let bytes = hex::decode(&possible_hash)
        .map_err(|_| "Clipboard contains invalid or truncated hex".to_owned())?;
    let alg = Algorithm::from_len(bytes.len())?;
    let candidate = CandidateHash {
        filename: None,
        bytes,
    };
    Ok(CandidateHashes {
        alg,
        hashes: vec![candidate],
        source: VerificationSource::Clipboard,
    })
}

/// Generate a candidate hash from the digests file specified (could be "-" for STDIN), or throw an error.
fn get_from_file(path: &PathBuf) -> Result<CandidateHashes, String> {
    // Get a reader for either standard input or the chosen path
    let reader: Box<dyn Read> = if path.to_str() == Some("-") {
        Box::new(std::io::stdin())
    } else {
        Box::new(File::open(path).map_err(|_| {
            format!(
                "Unable to open check file at path '{}'",
                path.to_string_lossy()
            )
        })?)
    };

    // Read the first line, trimmed
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|_| "Error reading from check file".to_owned())?;
    let line = line.trim().to_owned();

    // Does our first line look like a raw hash on its own? If so, use that
    if let Some(candidate) = read_raw_candidate_from_file(&line, &path) {
        return Ok(candidate);
    }

    // Maybe it's a digests file
    // Reconstruct the full iterator by joining our already-read line with the others
    let full_lines = vec![Ok(line)].into_iter().chain(reader.lines());

    // Does the entire file look like a coreutils-style digests file? (SHA1SUMS, etc.)
    if let Some(candidate) = read_coreutils_digests_from_file(full_lines, &path) {
        return Ok(candidate);
    }

    // If neither of these techniques worked this is a fatal error
    // The user requested we use this input but we couldn't
    Err(format!(
        "Provided check file '{}' was neither a hash nor a valid digests file",
        path.to_string_lossy()
    ))
}

fn read_raw_candidate_from_file(line: &str, path: &PathBuf) -> Option<CandidateHashes> {
    // It is a little sad to use a dynamic regex in an otherwise nice Rust program
    // These deserve to be replaced with a good old fashioned static parser
    // But let's be honest: the impact is negligible
    let re = Regex::new(r"^([[:xdigit:]]{32}|[[:xdigit:]]{40}|[[:xdigit:]]{64})$").unwrap();
    if re.is_match(line) {
        // These should both always succeed due to the matching
        let bytes = match hex::decode(line) {
            Ok(bytes) => bytes,
            _ => return None,
        };
        let alg = match Algorithm::from_len(bytes.len()) {
            Ok(alg) => alg,
            _ => return None,
        };
        return Some(CandidateHashes {
            alg,
            source: VerificationSource::RawFile(path.clone()),
            hashes: vec![CandidateHash {
                bytes,
                filename: None,
            }],
        });
    }
    None
}

fn read_coreutils_digests_from_file<I>(lines: I, path: &PathBuf) -> Option<CandidateHashes>
where
    I: Iterator<Item = io::Result<String>>,
{
    let re = Regex::new(
        r"^(?P<hash>([[:xdigit:]]{32}|[[:xdigit:]]{40}|[[:xdigit:]]{64})) .(?P<filename>.+)$",
    )
    .unwrap();

    let mut hashes = vec![];
    let mut alg: Option<Algorithm> = None;
    for l in lines {
        if let Ok(l) = l {
            let l = l.trim();
            // Allow (ignore) blank lines
            if l.is_empty() {
                continue;
            }
            // If we can capture a valid line, use it
            if let Some(captures) = re.captures(&l) {
                let hash = &captures["hash"];
                let filename = &captures["filename"];
                // Decode the hex and algorithm for this line
                let line_bytes = match hex::decode(hash) {
                    Ok(bytes) => bytes,
                    _ => return None,
                };
                let line_alg = match Algorithm::from_len(line_bytes.len()) {
                    Ok(alg) => alg,
                    _ => return None,
                };
                if alg.is_some() && alg != Some(line_alg) {
                    // Different algorithms in the same digest file are not supported
                    return None;
                } else {
                    // If we are the first line, we define the overall algorithm
                    alg = Some(line_alg);
                }
                // So far so good - create an entry for this line
                hashes.push(CandidateHash {
                    bytes: line_bytes,
                    filename: Some(filename.to_owned()),
                });
            } else {
                // But if we have a line with content we cannot parse, this is an error
                return None;
            }
        }
    }

    // It is a failure if we got zero hashes or we somehow don't know the algorithm
    if hashes.is_empty() {
        return None;
    }
    let alg = match alg {
        Some(alg) => alg,
        _ => return None,
    };

    // Otherwise all is well and we can return our results
    Some(CandidateHashes {
        alg,
        source: VerificationSource::DigestsFile(path.clone()),
        hashes,
    })
}

/// Determine if the calculated hash matches any of the candidates.
///
/// Ok result: the hash matches, and if the candidate has a filename, that matches too
/// Maybe result: the hash matches but the filename does not
/// Fail result: neither of the above
pub fn verify_hash<'a>(calculated: &Hash, candidates: &'a CandidateHashes) -> Verification<'a> {
    let mut ok: Option<&CandidateHash> = None;
    let mut maybe: Option<&CandidateHash> = None;
    let mut messages = Vec::new();

    for candidate in &candidates.hashes {
        if candidate.bytes == calculated.bytes {
            match candidate.filename {
                None => ok = Some(candidate),
                Some(ref candidate_filename) if candidate_filename == &calculated.filename => {
                    ok = Some(candidate)
                }
                Some(ref candidate_filename) => {
                    messages.push((
                        MessageLevel::Warning,
                        format!(
                            "The matched hash has filename '{}', which does not match the input.",
                            candidate_filename
                        ),
                    ));
                    maybe = Some(candidate);
                }
            }
        }
    }

    // Warn that a "successful" MD5 result is not necessarily great
    if candidates.alg == Algorithm::Md5 && (ok.is_some() || maybe.is_some()) {
        messages.push((
            MessageLevel::Note,
            "MD5 can easily be forged. Use a stronger algorithm if possible.".to_owned(),
        ))
    }

    // If we got a full match, great
    if ok.is_some() {
        return Verification {
            match_level: MatchLevel::Ok,
            comparison_hash: ok,
            messages,
        };
    }

    // Second priority, a "maybe" result
    if maybe.is_some() {
        return Verification {
            match_level: MatchLevel::Maybe,
            comparison_hash: maybe,
            messages,
        };
    }

    // Otherwise we failed
    // If we only had one candidate hash, include it
    let comparison = match candidates.hashes.len() {
        1 => Some(&candidates.hashes[0]),
        _ => None,
    };
    Verification {
        match_level: MatchLevel::Fail,
        comparison_hash: comparison,
        messages,
    }
}
