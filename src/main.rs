use std::error::Error;
use std::path::PathBuf;
use std::process;
use structopt::StructOpt;

/// Calculate digests for given input data
mod calculate;

/// Display output nicely in the terminal
mod display;

/// Collect candidate hashes based on options and match them against a calculated hash
mod verify;

#[derive(StructOpt)]
#[structopt(name = "hashgood")]
pub struct Opt {
    /// Read the hash from the clipboard
    #[cfg(feature = "paste")]
    #[structopt(short = "p", long = "paste")]
    paste: bool,

    /// Disable ANSI colours in output
    #[structopt(short = "C", long = "no-colour")]
    no_colour: bool,

    /// A file containing the hash to verify. It can either be a raw hash or a SHASUMS-style listing. Use `-` for standard input.
    #[structopt(short = "c", long = "check", parse(from_os_str))]
    hash_file: Option<PathBuf>,

    /// The file to be verified or `-` for standard input
    #[structopt(name = "input", parse(from_os_str))]
    input: PathBuf,

    /// A hash to verify, supplied directly on the command line
    #[structopt(name = "hash")]
    hash: Option<String>,
}

impl Opt {
    fn get_paste(&self) -> bool {
        #[cfg(feature = "paste")] {
            return self.paste;
        }
        #[cfg(not(feature = "paste"))] {
            return false;
        }
    }
}

/// Types of supported digest algorithm
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Algorithm {
    Md5,
    Sha1,
    Sha256,
}

impl Algorithm {
    /// Assume a hash type from the binary length. Fortunately the typical 3 algorithms we care about are different lengths.
    pub fn from_len(len: usize) -> Result<Algorithm, String> {
        match len {
            16 => Ok(Algorithm::Md5),
            20 => Ok(Algorithm::Sha1),
            32 => Ok(Algorithm::Sha256),
            _ => Err(format!("Unrecognised hash length: {} bytes", len)),
        }
    }
}

/// The method by which one or more hashes were supplied to verify the calculated digest
pub enum VerificationSource {
    CommandArgument,
    Clipboard,
    RawFile(PathBuf),
    DigestsFile(PathBuf),
}

/// A complete standalone hash result
pub struct Hash {
    alg: Algorithm,
    bytes: Vec<u8>,
    filename: String,
}

impl Hash {
    pub fn new(alg: Algorithm, bytes: Vec<u8>, path: &PathBuf) -> Self {
        // Taking the filename component should always work?
        // If not, just fall back to the full path
        let filename = match path.file_name() {
            Some(filename) => filename.to_string_lossy(),
            None => path.to_string_lossy(),
        };
        Self {
            alg,
            bytes,
            filename: filename.to_string(),
        }
    }
}

/// A possible hash to match against. The algorithm is assumed.
pub struct CandidateHash {
    bytes: Vec<u8>,
    filename: Option<String>,
}

/// A list of candidate hashes that our input could potentially match. At this point it is
/// assumed that we will be verifying a digest of a particular, single algorithm.
pub struct CandidateHashes {
    alg: Algorithm,
    hashes: Vec<CandidateHash>,
    source: VerificationSource,
}

/// Summary of an atetmpt to match the calculated digest against candidates
pub enum MatchLevel {
    Ok,
    Maybe,
    Fail,
}

/// The severity of any informational messages to be printed before the final result
pub enum MessageLevel {
    Error,
    Warning,
    Note,
}

/// Overall details of an attempt to match the calculated digest against candidates
pub struct Verification<'a> {
    match_level: MatchLevel,
    comparison_hash: Option<&'a CandidateHash>,
    messages: Vec<(MessageLevel, String)>,
}

/// Entry point - run the program and handle errors ourselves cleanly.
///
/// At the moment there aren't really any errors that can be handled by the application. Therefore
/// stringly-typed errors are used and they are all captured here, where the problem is printed
/// and the application terminates with a non-zero return code.
fn main() {
    hashgood().unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        process::exit(1);
    });
}

/// Main application logic
fn hashgood() -> Result<(), Box<dyn Error>> {
    let opt = get_verified_options()?;
    let candidates = verify::get_candidate_hashes(&opt)?;
    let input = calculate::get_input_reader(&opt.input)?;
    if let Some(c) = candidates {
        // If we have a candidate hash of a particular type, use that specific algorithm
        let hashes = calculate::create_digests(&[c.alg], input)?;
        for (alg, bytes) in hashes {
            // Should always be true
            if c.alg == alg {
                let hash = Hash::new(alg, bytes, &opt.input);
                let verification = verify::verify_hash(&hash, &c);
                display::print_hash(
                    &hash,
                    verification.comparison_hash,
                    Some(&c.source),
                    opt.no_colour,
                )?;
                display::print_messages(verification.messages, opt.no_colour)?;
                display::print_match_level(verification.match_level, opt.no_colour)?;
            }
        }
    } else {
        // If no candidate, calculate all three common digest types for output
        let hashes = calculate::create_digests(
            &[Algorithm::Md5, Algorithm::Sha1, Algorithm::Sha256],
            input,
        )?;
        for (alg, bytes) in hashes {
            let hash = Hash {
                alg,
                bytes,
                filename: opt.input.file_name().unwrap().to_string_lossy().to_string(),
            };
            display::print_hash(&hash, None, None, opt.no_colour)?;
        }
    }
    Ok(())
}

/// Parse the command line options and check for ambiguous or inconsistent settings
fn get_verified_options() -> Result<Opt, String> {
    let opt = Opt::from_args();
    let hash_methods =
        opt.hash.is_some() as i32 + opt.get_paste() as i32 + opt.hash_file.is_some() as i32;
    if hash_methods > 1 {
        if opt.hash.is_some() {
            eprintln!("* specified as command line argument");
        }
        if opt.get_paste() {
            eprintln!("* paste from clipboard (-p)")
        }
        if opt.hash_file.is_some() {
            eprintln!("* check hash from file (-c)")
        }
        return Err("Error: Hashes were provided by multiple methods. Use only one.".to_owned());
    }
    if opt.input.to_str() == Some("-")
        && opt.hash_file.as_ref().and_then(|h| h.to_str()) == Some("-")
    {
        return Err("Error: Cannot use use stdin for both hash file and input data".to_owned());
    }
    Ok(opt)
}
