use hspfmt::{detect_encoding, format, lex, Kind, Options};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files);
        } else if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "as" || ext == "hsp" || ext == "chsp" {
                    files.push(path);
                }
            }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: hspfmt_corpus directory [directory ...]");
        process::exit(2);
    }

    let mut target_files = Vec::new();
    for arg in &args[1..] {
        let p = Path::new(arg);
        if p.is_dir() {
            collect_files(p, &mut target_files);
        } else if p.is_file() {
            if let Some(ext) = p.extension() {
                if ext == "as" || ext == "hsp" || ext == "chsp" {
                    target_files.push(p.to_path_buf());
                }
            }
        }
    }
    // Sort files for deterministic order
    target_files.sort();

    let mut files = 0usize;
    let mut rejected = 0usize;
    let mut failed = 0usize;

    for path in target_files {
        let Ok(input) = fs::read(&path) else {
            continue;
        };

        let mut options = Options::default();
        let original = match detect_encoding(&input).and_then(|enc| {
            options.encoding = enc;
            lex(&input, enc)
        }) {
            Ok(tokens) => tokens,
            Err(e) => {
                rejected += 1;
                eprintln!("{}: {}", path.display(), e);
                continue;
            }
        };

        files += 1;

        let mut restored = Vec::with_capacity(input.len());
        for token in &original {
            restored.extend_from_slice(&input[token.begin..token.end]);
        }
        if restored != input {
            failed += 1;
            eprintln!("{}: roundtrip failed", path.display());
        }

        let formatted = match format(&input, &options, None) {
            Ok(f) => f,
            Err(e) => {
                rejected += 1;
                eprintln!("{}: {}", path.display(), e);
                continue;
            }
        };

        let check_result: Result<(), String> = (|| {
            let second_format = format(&formatted, &options, None)
                .map_err(|e| format!("second format failed: {}", e))?;
            if second_format != formatted {
                return Err("not idempotent".to_string());
            }

            let after = lex(&formatted, options.encoding)
                .map_err(|e| format!("lex after format failed: {}", e))?;

            let before_tokens: Vec<(Kind, &[u8])> = original
                .iter()
                .filter(|t| t.kind != Kind::Space)
                .map(|t| (t.kind, &input[t.begin..t.end]))
                .collect();

            let after_tokens: Vec<(Kind, &[u8])> = after
                .iter()
                .filter(|t| t.kind != Kind::Space)
                .map(|t| (t.kind, &formatted[t.begin..t.end]))
                .collect();

            if before_tokens != after_tokens {
                return Err("non-whitespace tokens changed".to_string());
            }
            Ok(())
        })();

        if let Err(e) = check_result {
            failed += 1;
            eprintln!("{}: {}", path.display(), e);
        }
    }

    println!(
        "{} tokenized, {} rejected, {} invariant failures",
        files, rejected, failed
    );
    if failed > 0 {
        process::exit(1);
    }
}
