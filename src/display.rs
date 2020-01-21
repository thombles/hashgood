use super::{Algorithm, CandidateHash, Hash, MatchLevel, MessageLevel, VerificationSource};
use std::borrow::Borrow;
use std::error::Error;
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub type PrintResult = Result<(), Box<dyn Error>>;

fn filename_display(filename: &str) -> &str {
    if filename == "-" {
        return "standard input";
    }
    filename
}

fn get_stdout(no_colour: bool) -> StandardStream {
    if no_colour {
        StandardStream::stdout(ColorChoice::Never)
    } else {
        StandardStream::stdout(ColorChoice::Always)
    }
}

fn write_filename(mut stdout: &mut StandardStream, filename: &str) -> PrintResult {
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
    write!(&mut stdout, "{}", filename_display(filename))?;
    stdout.reset()?;
    Ok(())
}

fn write_algorithm(mut stdout: &mut StandardStream, alg: Algorithm) -> PrintResult {
    match alg {
        Algorithm::Md5 => {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Magenta)))?;
            write!(&mut stdout, "MD5")?;
        }
        Algorithm::Sha1 => {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
            write!(&mut stdout, "SHA-1")?;
        }
        Algorithm::Sha256 => {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
            write!(&mut stdout, "SHA-256")?;
        }
    }
    stdout.reset()?;
    Ok(())
}

fn print_hex_compare(print: &str, against: &str, mut stdout: &mut StandardStream) -> PrintResult {
    for (p, a) in print.chars().zip(against.chars()) {
        if p == a {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
        } else {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
        }
        write!(&mut stdout, "{}", p)?;
    }
    stdout.reset()?;
    writeln!(&mut stdout)?;
    Ok(())
}

fn write_source(
    mut stdout: &mut StandardStream,
    verify_source: &VerificationSource,
    candidate_filename: &Option<String>,
) -> PrintResult {
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
    match &verify_source {
        VerificationSource::CommandArgument => {
            writeln!(&mut stdout, "command line argument")?;
        }
        VerificationSource::Clipboard => {
            writeln!(&mut stdout, "pasted from clipboard")?;
        }
        VerificationSource::RawFile(raw_path) => match raw_path.to_string_lossy().borrow() {
            "-" => {
                writeln!(&mut stdout, "from standard input")?;
            }
            path => {
                writeln!(&mut stdout, "from file '{}' containing raw hash", path)?;
            }
        },
        VerificationSource::DigestsFile(digest_path) => {
            match digest_path.to_string_lossy().borrow() {
                "-" => {
                    writeln!(
                        &mut stdout,
                        "'{}' from digests on standard input",
                        candidate_filename.as_ref().unwrap()
                    )?;
                }
                path => {
                    writeln!(
                        &mut stdout,
                        "'{}' in digests file '{}'",
                        candidate_filename.as_ref().unwrap(),
                        path
                    )?;
                }
            }
        }
    }
    stdout.reset()?;
    Ok(())
}

pub fn print_hash(
    hash: &Hash,
    verify_hash: Option<&CandidateHash>,
    verify_source: Option<&VerificationSource>,
    no_colour: bool,
) -> PrintResult {
    let mut stdout = get_stdout(no_colour);

    write_filename(&mut stdout, &hash.filename)?;
    write!(&mut stdout, " / ")?;
    write_algorithm(&mut stdout, hash.alg)?;
    writeln!(&mut stdout)?;

    // Handle basic case first - nothing to compare it to
    let hash_hex = hex::encode(&hash.bytes);
    let verify_hash = match verify_hash {
        None => {
            write!(&mut stdout, "{}\n\n", hash_hex)?;
            return Ok(());
        }
        Some(verify_hash) => verify_hash,
    };
    let other_hex = hex::encode(&verify_hash.bytes);

    // Do a top-to-bottom comparison
    print_hex_compare(&hash_hex, &other_hex, &mut stdout)?;
    print_hex_compare(&other_hex, &hash_hex, &mut stdout)?;

    // Show the source of our hash
    if let Some(source) = verify_source {
        write_source(&mut stdout, source, &verify_hash.filename)?;
    }

    writeln!(&mut stdout)?;
    Ok(())
}

pub fn print_messages(messages: Vec<(MessageLevel, String)>, no_colour: bool) -> PrintResult {
    let mut stdout = get_stdout(no_colour);

    for (level, msg) in &messages {
        match level {
            MessageLevel::Error => {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
                write!(&mut stdout, "(error) ")?;
            }
            MessageLevel::Warning => {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
                write!(&mut stdout, "(warning) ")?;
            }
            MessageLevel::Note => {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
                write!(&mut stdout, "(note) ")?;
            }
        }
        stdout.reset()?;
        writeln!(&mut stdout, "{}", msg)?;
    }
    if !messages.is_empty() {
        writeln!(&mut stdout)?
    }

    Ok(())
}

pub fn print_match_level(match_level: MatchLevel, no_colour: bool) -> PrintResult {
    let mut stdout = get_stdout(no_colour);
    write!(&mut stdout, "Result: ")?;
    match match_level {
        MatchLevel::Ok => {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
            writeln!(&mut stdout, "OK")?;
        }
        MatchLevel::Maybe => {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
            writeln!(&mut stdout, "MAYBE")?;
        }
        MatchLevel::Fail => {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
            writeln!(&mut stdout, "FAIL")?;
        }
    }
    stdout.reset()?;
    Ok(())
}
