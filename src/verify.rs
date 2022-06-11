use super::{
    Algorithm, CandidateHash, CandidateHashes, Hash, MatchLevel, MessageLevel, Opt, Verification,
    VerificationSource,
};
#[cfg(feature = "paste")]
use copypasta::{ClipboardContext, ClipboardProvider};
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
    } else if opt.get_paste() {
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
    #[cfg(feature = "paste")]
    {
        let mut ctx: ClipboardContext = match ClipboardContext::new() {
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
        return Ok(CandidateHashes {
            alg,
            hashes: vec![candidate],
            source: VerificationSource::Clipboard,
        });
    }
    #[cfg(not(feature = "paste"))]
    {
        return Err("Paste not implemented".to_owned());
    }
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

fn try_parse_hash(s: &str) -> Option<(Algorithm, Vec<u8>)> {
    let bytes = match hex::decode(s.trim()) {
        Ok(bytes) => bytes,
        _ => return None,
    };
    let alg = match Algorithm::from_len(bytes.len()) {
        Ok(alg) => alg,
        _ => return None,
    };
    Some((alg, bytes))
}

fn read_raw_candidate_from_file(line: &str, path: &PathBuf) -> Option<CandidateHashes> {
    let (alg, bytes) = try_parse_hash(line)?;
    Some(CandidateHashes {
        alg,
        source: VerificationSource::RawFile(path.clone()),
        hashes: vec![CandidateHash {
            bytes,
            filename: None,
        }],
    })
}

fn read_coreutils_digests_from_file<I, S>(lines: I, path: &PathBuf) -> Option<CandidateHashes>
where
    I: Iterator<Item = io::Result<S>>,
    S: AsRef<str>,
{
    let mut hashes = vec![];
    let mut alg: Option<Algorithm> = None;
    for l in lines {
        if let Ok(l) = l {
            let l = l.as_ref().trim();
            // Allow (ignore) blank lines
            if l.is_empty() {
                continue;
            }
            // Expected format
            // <valid-hash><space><space-or-*><filename>
            let (line_alg, bytes, filename) = match l
                .find(' ')
                .and_then(|space_pos| {
                    // Char before filename should be space for text or * for binary
                    match l.chars().nth(space_pos + 1) {
                        Some(' ') | Some('*') => (l.get(..space_pos)).zip(l.get(space_pos + 2..)),
                        _ => None,
                    }
                })
                .and_then(|(maybe_hash, filename)| {
                    // Filename should be in this position without extra whitespace
                    if filename.trim() == filename {
                        try_parse_hash(maybe_hash).map(|(alg, bytes)| (alg, bytes, filename))
                    } else {
                        None
                    }
                }) {
                Some(t) => t,
                None => {
                    // if we have a line with content we cannot parse, this is an error
                    return None;
                }
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
                bytes,
                filename: Some(filename.to_owned()),
            });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_raw_inputs() {
        let example_path: PathBuf = "some_file".into();
        let valid_md5 = "d229da563da18fe5d58cd95a6467d584";
        let valid_sha1 = "b314c7ebb7d599944981908b7f3ed33a30e78f3a";
        let valid_sha1_2 = valid_sha1.to_uppercase();
        let valid_sha256 = "1eb85fc97224598dad1852b5d6483bbcf0aa8608790dcc657a5a2a761ae9c8c6";

        let invalid1 = "x";
        let invalid2 = "a";
        let invalid3 = "d229da563da18fe5d58cd95a6467d58";
        let invalid4 = "1eb85fc97224598dad1852b5d6483bbcf0aa8608790dcc657a5a2a761ae9c8c67";
        let invalid5 = "1eb85fc97224598dad1852b5d 483bbcf0aa8608790dcc657a5a2a761ae9c8c6";

        assert!(matches!(
            read_raw_candidate_from_file(valid_md5, &example_path),
            Some(CandidateHashes {
                alg: Algorithm::Md5,
                ..
            })
        ));
        assert!(matches!(
            read_raw_candidate_from_file(valid_sha1, &example_path),
            Some(CandidateHashes {
                alg: Algorithm::Sha1,
                ..
            })
        ));
        assert!(matches!(
            read_raw_candidate_from_file(&valid_sha1_2, &example_path),
            Some(CandidateHashes {
                alg: Algorithm::Sha1,
                ..
            })
        ));
        assert!(matches!(
            read_raw_candidate_from_file(valid_sha256, &example_path),
            Some(CandidateHashes {
                alg: Algorithm::Sha256,
                ..
            })
        ));

        for i in &[invalid1, invalid2, invalid3, invalid4, invalid5] {
            assert!(read_raw_candidate_from_file(*i, &example_path).is_none());
        }
    }

    #[test]
    fn test_read_shasums() {
        let shasums = "4b91f7a387a6edd4a7c0afb2897f1ca968c9695b *cp
        75eb7420a9f5a260b04a3e8ad51e50f2838a17fc  lel.txt

        fe6c26d485a3573a1cb0ad0682f5105325a1905f  shasums";
        let lines = shasums.lines().map(|l| std::io::Result::Ok(l));
        let path = PathBuf::from("SHASUMS");
        let candidates = read_coreutils_digests_from_file(lines, &path);

        assert_eq!(
            candidates,
            Some(CandidateHashes {
                alg: Algorithm::Sha1,
                hashes: vec![
                    CandidateHash {
                        bytes: hex::decode("4b91f7a387a6edd4a7c0afb2897f1ca968c9695b").unwrap(),
                        filename: Some("cp".to_owned()),
                    },
                    CandidateHash {
                        bytes: hex::decode("75eb7420a9f5a260b04a3e8ad51e50f2838a17fc").unwrap(),
                        filename: Some("lel.txt".to_owned()),
                    },
                    CandidateHash {
                        bytes: hex::decode("fe6c26d485a3573a1cb0ad0682f5105325a1905f").unwrap(),
                        filename: Some("shasums".to_owned()),
                    }
                ],
                source: VerificationSource::DigestsFile(path),
            })
        );
    }

    #[test]
    fn test_invalid_shasums() {
        let no_format = "4b91f7a387a6edd4a7c0afb2897f1ca968c9695b cp";
        let invalid_format = "4b91f7a387a6edd4a7c0afb2897f1ca968c9695b .cp";
        let extra_space = "4b91f7a387a6edd4a7c0afb2897f1ca968c9695b   cp";

        for digest in [no_format, invalid_format, extra_space] {
            let lines = digest.lines().map(|l| std::io::Result::Ok(l));
            assert!(
                read_coreutils_digests_from_file(lines, &PathBuf::from("SHASUMS")).is_none(),
                "Should be invalid digest: {:?}",
                digest
            );
        }
    }
}
