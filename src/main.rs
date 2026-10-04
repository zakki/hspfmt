use hspfmt::{
    format, lex, BlockComments, CommentStyle, Encoding, FullWidthSpaces, OperatorStyle, Options,
    Parentheses, Spacing,
};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::time::SystemTime;

#[cfg(windows)]
mod windows_file {
    use std::ffi::c_void;
    use std::fs::File;
    use std::io;
    use std::os::windows::{ffi::OsStrExt, io::AsRawHandle};
    use std::path::Path;

    // BY_HANDLE_FILE_INFORMATION; FILETIME consists of two DWORDs.
    #[repr(C)]
    #[derive(Default)]
    struct FileInformation {
        attributes: u32,
        creation_time: [u32; 2],
        last_access_time: [u32; 2],
        last_write_time: [u32; 2],
        volume_serial_number: u32,
        size_high: u32,
        size_low: u32,
        number_of_links: u32,
        index_high: u32,
        index_low: u32,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetFileInformationByHandle(
            handle: *mut c_void,
            information: *mut FileInformation,
        ) -> i32;
        fn ReplaceFileW(
            replaced: *const u16,
            replacement: *const u16,
            backup: *const u16,
            flags: u32,
            exclude: *mut c_void,
            reserved: *mut c_void,
        ) -> i32;
    }

    pub fn link_count(path: &Path) -> io::Result<u32> {
        let file = File::open(path)?;
        let mut information = FileInformation::default();
        // SAFETY: the file owns a live handle and information has the Win32 layout.
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(information.number_of_links)
    }

    pub fn replace(path: &Path, temporary: &Path) -> io::Result<()> {
        let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let temporary: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: both names are NUL-terminated UTF-16 buffers alive for the call.
        // Flags are zero so ACL/attribute merge failures are reported, not ignored.
        let success = unsafe {
            ReplaceFileW(
                path.as_ptr(),
                temporary.as_ptr(),
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if success == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

fn path_option(arg: &OsStr, prefix: &str) -> Option<PathBuf> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        arg.as_bytes()
            .strip_prefix(prefix.as_bytes())
            .map(|path| PathBuf::from(OsStr::from_bytes(path)))
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::{OsStrExt, OsStringExt};
        let arg: Vec<u16> = arg.encode_wide().collect();
        let prefix: Vec<u16> = prefix.encode_utf16().collect();
        arg.strip_prefix(prefix.as_slice())
            .map(|path| PathBuf::from(OsString::from_wide(path)))
    }
    #[cfg(not(any(unix, windows)))]
    {
        arg.to_str()?.strip_prefix(prefix).map(PathBuf::from)
    }
}

struct Cleanup {
    file: PathBuf,
    directory: PathBuf,
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.file);
        let _ = fs::remove_dir(&self.directory);
    }
}

fn replace_file(path: &Path, output: &[u8]) -> Result<(), String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut directory = PathBuf::new();

    // Use timestamp and process ID to generate pseudorandom names without external crates
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = process::id();

    for attempt in 0..100u64 {
        let rand_val =
            (now.wrapping_mul(6364136223846793005) ^ (pid as u128)).wrapping_add(attempt as u128);
        let candidate = parent.join(format!(".hspfmt-{}", rand_val as u32));
        if fs::create_dir(&candidate).is_ok() {
            directory = candidate;
            break;
        }
    }

    if directory.as_os_str().is_empty() {
        return Err("cannot create temporary directory".to_string());
    }

    let temporary = directory.join("output");
    let _cleanup = Cleanup {
        file: temporary.clone(),
        directory: directory.clone(),
    };

    let mut file =
        File::create(&temporary).map_err(|_| "cannot create temporary output".to_string())?;
    file.write_all(output)
        .map_err(|_| "temporary output write failed".to_string())?;
    file.sync_all()
        .map_err(|_| "temporary output write failed".to_string())?;
    drop(file);

    #[cfg(not(windows))]
    {
        let permissions = fs::metadata(path)
            .map_err(|e| format!("cannot read input permissions: {}", e))?
            .permissions();
        fs::set_permissions(&temporary, permissions)
            .map_err(|e| format!("cannot preserve input permissions: {}", e))?;
        fs::rename(&temporary, path).map_err(|e| format!("cannot replace input file: {}", e))?;
    }
    #[cfg(windows)]
    windows_file::replace(path, &temporary)
        .map_err(|e| format!("cannot replace input file: {}", e))?;

    Ok(())
}

fn number(s: &str) -> Result<usize, String> {
    if s.is_empty() || !s.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("invalid numeric option: {}", s));
    }
    let significant = s.find(|c| c != '0');
    if let Some(pos) = significant {
        if s.len() - pos > 5 {
            return Err(format!("numeric option too large: {}", s));
        }
    }
    match s.parse::<usize>() {
        Ok(v) if v <= 10000 => Ok(v),
        _ => Err(format!("numeric option too large: {}", s)),
    }
}

fn parentheses(s: &str) -> Result<Parentheses, String> {
    match s {
        "preserve" => Ok(Parentheses::Preserve),
        "add" => Ok(Parentheses::Add),
        "remove" => Ok(Parentheses::Remove),
        _ => Err(format!("invalid parentheses mode: {}", s)),
    }
}

fn blank_lines(s: &str) -> Result<i32, String> {
    if s == "preserve" {
        return Ok(-1);
    }
    let value = number(s)?;
    if value > 16 {
        return Err("blank line count must be between 0 and 16".to_string());
    }
    Ok(value as i32)
}

fn operator_style(s: &str) -> Result<OperatorStyle, String> {
    match s {
        "preserve" => Ok(OperatorStyle::Preserve),
        "hsp" => Ok(OperatorStyle::Hsp),
        "c" => Ok(OperatorStyle::C),
        _ => Err(format!("invalid operator style: {}", s)),
    }
}

fn spacing(s: &str) -> Result<Spacing, String> {
    match s {
        "preserve" => Ok(Spacing::Preserve),
        "space" => Ok(Spacing::Space),
        "compact" => Ok(Spacing::Compact),
        _ => Err(format!("invalid spacing mode: {}", s)),
    }
}

fn parse_formatting_option(arg: &str, options: &mut Options) -> Result<bool, String> {
    if arg == "--tabs" {
        options.tabs = true;
    } else if arg == "--compact-operators" {
        options.operator_spacing = Spacing::Compact;
    } else if let Some(v) = arg.strip_prefix("--operator-spacing=") {
        options.operator_spacing = spacing(v)?;
    } else if let Some(v) = arg.strip_prefix("--comma-spacing=") {
        options.comma_spacing = spacing(v)?;
    } else if let Some(v) = arg.strip_prefix("--colon-spacing=") {
        options.colon_spacing = spacing(v)?;
    } else if let Some(v) = arg.strip_prefix("--comment-spacing=") {
        options.comment_spacing = spacing(v)?;
    } else if arg == "--hsp-prefixes" {
        options.hsp_numeric_prefixes = true;
    } else if arg == "--short-if" {
        options.short_if = true;
    } else if let Some(value) = arg.strip_prefix("--full-width-spaces=") {
        match value {
            "preserve" => options.full_width_spaces = FullWidthSpaces::Preserve,
            "normalize" => options.full_width_spaces = FullWidthSpaces::Normalize,
            _ => return Err(format!("invalid full-width space mode: {}", value)),
        }
    } else if let Some(v) = arg.strip_prefix("--operator-style=") {
        options.operator_style = operator_style(v)?;
    } else if let Some(v) = arg.strip_prefix("--increment-style=") {
        options.increment_style = operator_style(v)?;
    } else if arg == "--indent-labels" {
        options.indent_labels = true;
    } else if arg == "--no-indent-labels" {
        options.indent_labels = false;
    } else if let Some(value) = arg.strip_prefix("--comment-style=") {
        match value {
            "preserve" => options.comment_style = CommentStyle::Preserve,
            "semicolon" => options.comment_style = CommentStyle::Semicolon,
            "c" => options.comment_style = CommentStyle::C,
            _ => return Err(format!("invalid comment style: {}", value)),
        }
    } else if let Some(value) = arg.strip_prefix("--block-comments=") {
        match value {
            "preserve" => options.block_comments = BlockComments::Preserve,
            "lines" => options.block_comments = BlockComments::Lines,
            "block" => options.block_comments = BlockComments::Block,
            _ => return Err(format!("invalid block comment mode: {}", value)),
        }
    } else if let Some(v) = arg.strip_prefix("--condition-parens=") {
        options.condition_parens = parentheses(v)?;
    } else if let Some(v) = arg.strip_prefix("--repeat-parens=") {
        options.repeat_parens = parentheses(v)?;
    } else if let Some(v) = arg.strip_prefix("--blank-lines-before-module=") {
        options.blank_lines_before_module = blank_lines(v)?;
    } else if let Some(v) = arg.strip_prefix("--blank-lines-before-deffunc=") {
        options.blank_lines_before_deffunc = blank_lines(v)?;
    } else if let Some(v) = arg.strip_prefix("--blank-lines-before-defcfunc=") {
        options.blank_lines_before_defcfunc = blank_lines(v)?;
    } else if arg == "--encoding=auto" {
        options.encoding = Encoding::Auto;
    } else if arg == "--encoding=cp932" {
        options.encoding = Encoding::Cp932;
    } else if arg == "--encoding=utf8" {
        options.encoding = Encoding::Utf8;
    } else if let Some(v) = arg.strip_prefix("--encoding=") {
        return Err(format!("invalid encoding: {}", v));
    } else if let Some(v) = arg.strip_prefix("--base-indent=") {
        options.base_indent = number(v)?;
    } else if let Some(v) = arg.strip_prefix("--loop-indent=") {
        options.loop_indent = number(v)?;
    } else if let Some(value) = arg.strip_prefix("--indent=") {
        options.preserve_indent = value == "preserve";
        if !options.preserve_indent {
            options.indent_width = number(value)?;
        }
    } else if let Some(v) = arg.strip_prefix("--line-width=") {
        options.line_width = number(v)?;
    } else {
        return Ok(false);
    }
    Ok(true)
}

fn load_config(path: &Path, options: &mut Options) -> Result<(), String> {
    let content =
        fs::read(path).map_err(|_| format!("cannot open config file: {}", path.display()))?;
    for (line_idx, line) in content.split(|&b| b == b'\n').enumerate() {
        // These ASCII separators cannot occur inside a CP932 multibyte character.
        let is_space = |b: &u8| matches!(b, b' ' | b'\t' | b'\r' | b'\n');
        let start = line.iter().position(|b| !is_space(b)).unwrap_or(line.len());
        let end = line
            .iter()
            .rposition(|b| !is_space(b))
            .map_or(start, |i| i + 1);
        let trimmed = &line[start..end];
        if trimmed.is_empty() || trimmed.starts_with(b"#") || trimmed.starts_with(b";") {
            continue;
        }
        let trimmed = std::str::from_utf8(trimmed).map_err(|_| {
            format!(
                "{}:{}: config options must be ASCII",
                path.display(),
                line_idx + 1
            )
        })?;
        let opt = if trimmed.starts_with("--") {
            trimmed.to_string()
        } else {
            format!("--{}", trimmed)
        };
        match parse_formatting_option(&opt, options) {
            Ok(true) => {}
            _ => {
                return Err(format!(
                    "{}:{}: invalid or unsupported config option: {}",
                    path.display(),
                    line_idx + 1,
                    opt
                ));
            }
        }
    }
    Ok(())
}

fn print_help() {
    print!(
        "Usage: hspfmt [options] [file...|-]\n\
         Writes formatted source to stdout unless --write is specified.\n  \
         --write, -w          Replace the input file(s) in place (no stdout)\n  \
         --check              Exit 1 if formatting differs, 0 if unchanged\n  \
         --roundtrip          Reconstruct source from lossless tokens\n  \
         --config=FILE        Load configuration file (default: .hspfmt)\n  \
         --no-config          Disable configuration file loading\n  \
         --stdin-filepath=PATH File path used in diagnostic messages for stdin\n  \
         --indent=N|preserve --tabs (default: 4 spaces)\n  \
         --base-indent=N      Base indentation levels (0..16; default: 0)\n  \
         --loop-indent=N      Indentation levels per loop (0..16; default: 1)\n  \
         --compact-operators Remove optional binary-operator spaces\n  \
         --operator-spacing=preserve|space|compact (default: space)\n  \
         --comma-spacing=preserve|space|compact (default: space after commas)\n  \
         --colon-spacing=preserve|space|compact (default: space)\n  \
         --comment-spacing=preserve|space|compact (before comments; default: space)\n  \
         --full-width-spaces=preserve|normalize (default: preserve)\n  \
         --hsp-prefixes       Convert 0x/0b to $/% (preserve digits)\n  \
         --operator-style=preserve|hsp|c (binary operator spelling)\n  \
         --increment-style=preserve|hsp|c (increment/decrement statements)\n  \
         --short-if           Collapse if blocks containing ordinary statements\n  \
         --line-width=N       Short-if byte width limit (default: 100)\n  \
         --indent-labels / --no-indent-labels (default: no indentation)\n  \
         --comment-style=preserve|semicolon|c (line comment markers)\n  \
         --block-comments=preserve|lines|block (standalone comments)\n  \
         --condition-parens=preserve|add|remove (if/while)\n  \
         --repeat-parens=preserve|add|remove (each repeat argument)\n  \
         --blank-lines-before-module=N|preserve (0..16)\n  \
         --blank-lines-before-deffunc=N|preserve (0..16)\n  \
         --blank-lines-before-defcfunc=N|preserve (0..16)\n  \
         --encoding=auto|utf8|cp932 (default: auto; bytes are preserved)\n\
         Exit 2 indicates an input, syntax, option, or output error.\n"
    );
}

fn run() -> Result<i32, (Option<String>, String, Option<usize>)> {
    let args: Vec<OsString> = env::args_os().collect();
    let mut options = Options::default();
    let mut check = false;
    let mut write = false;
    let mut roundtrip = false;
    let mut positional = false;
    let mut stdin_filepath = PathBuf::new();
    let mut config_path = PathBuf::new();
    let mut no_config = false;
    let mut filenames = Vec::new();

    // First pass: scan for help, no-config, config
    for arg in &args[1..] {
        if arg == "--" {
            break;
        }
        if arg == "--help" {
            print_help();
            return Ok(0);
        } else if arg == "--no-config" {
            no_config = true;
        } else if let Some(path) = path_option(arg, "--config=") {
            config_path = path;
            if config_path.as_os_str().is_empty() {
                return Err((
                    None,
                    "--config requires a non-empty file path".to_string(),
                    None,
                ));
            }
        }
    }

    if !no_config {
        if !config_path.as_os_str().is_empty() {
            load_config(&config_path, &mut options).map_err(|e| (None, e, None))?;
        } else if Path::new(".hspfmt").exists() {
            load_config(Path::new(".hspfmt"), &mut options).map_err(|e| (None, e, None))?;
        }
    }

    // Second pass: options and filenames
    for arg in &args[1..] {
        if positional {
            filenames.push(PathBuf::from(arg));
            continue;
        }
        let text = arg.to_str().unwrap_or("");
        if arg == "--" {
            positional = true;
        } else if arg == "--no-config" || path_option(arg, "--config=").is_some() {
            // Already handled
        } else if arg == "--check" {
            check = true;
        } else if arg == "--write" || arg == "-w" {
            write = true;
        } else if arg == "--roundtrip" {
            roundtrip = true;
        } else if let Some(path) = path_option(arg, "--stdin-filepath=") {
            stdin_filepath = path;
        } else if parse_formatting_option(text, &mut options).map_err(|e| (None, e, None))? {
            // Handled formatting option
        } else if arg != "-" && arg.to_string_lossy().starts_with('-') {
            return Err((
                None,
                format!("unknown option: {}", arg.to_string_lossy()),
                None,
            ));
        } else {
            filenames.push(PathBuf::from(arg));
        }
    }

    if filenames.is_empty() {
        filenames.push(PathBuf::from("-"));
    }
    let is_stdin = filenames.len() == 1 && filenames[0] == Path::new("-");

    if !stdin_filepath.as_os_str().is_empty() && !is_stdin {
        return Err((
            None,
            "--stdin-filepath can only be used with stdin".to_string(),
            None,
        ));
    }

    if filenames.len() > 1 {
        if !write && !check {
            return Err((
                None,
                "multiple files are only supported with --write or --check".to_string(),
                None,
            ));
        }
        if roundtrip {
            return Err((
                None,
                "--roundtrip cannot be combined with multiple files".to_string(),
                None,
            ));
        }
        for f in &filenames {
            if f == Path::new("-") {
                return Err((
                    None,
                    "cannot combine stdin with multiple files".to_string(),
                    None,
                ));
            }
        }
    }

    if write && is_stdin {
        return Err((
            None,
            "--write requires an input file, not stdin".to_string(),
            None,
        ));
    }
    if write && (check || roundtrip) {
        return Err((
            None,
            "--write cannot be combined with --check or --roundtrip".to_string(),
            None,
        ));
    }

    let mut has_diff = false;
    let mut replacements = Vec::new();

    for filename in &filenames {
        let input_name = if filename == Path::new("-") {
            if !stdin_filepath.as_os_str().is_empty() {
                stdin_filepath.display().to_string()
            } else {
                "<stdin>".to_string()
            }
        } else {
            filename.display().to_string()
        };

        if write {
            let p = Path::new(filename);
            let symlink_meta = fs::symlink_metadata(p).map_err(|_| {
                (
                    Some(input_name.clone()),
                    "--write requires a regular file without symbolic or hard links".to_string(),
                    None,
                )
            })?;
            if !symlink_meta.is_file() {
                return Err((
                    Some(input_name),
                    "--write requires a regular file without symbolic or hard links".to_string(),
                    None,
                ));
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if symlink_meta.nlink() != 1 {
                    return Err((
                        Some(input_name),
                        "--write requires a regular file without symbolic or hard links"
                            .to_string(),
                        None,
                    ));
                }
            }
            #[cfg(windows)]
            if windows_file::link_count(p).ok() != Some(1) {
                return Err((
                    Some(input_name),
                    "--write requires a regular file without symbolic or hard links".to_string(),
                    None,
                ));
            }
        }

        let source = if filename == Path::new("-") {
            let mut buf = Vec::new();
            io::stdin().read_to_end(&mut buf).map_err(|_| {
                (
                    Some(input_name.clone()),
                    "input read failed".to_string(),
                    None,
                )
            })?;
            buf
        } else {
            fs::read(filename).map_err(|_| {
                (
                    Some(input_name.clone()),
                    "cannot open input file".to_string(),
                    None,
                )
            })?
        };

        let output = if roundtrip {
            let tokens = lex(&source, options.encoding).map_err(|e| {
                (
                    Some(input_name.clone()),
                    e.message().to_string(),
                    if e.line() > 0 { Some(e.line()) } else { None },
                )
            })?;
            let mut out = Vec::with_capacity(source.len());
            for token in tokens {
                out.extend_from_slice(&source[token.begin..token.end]);
            }
            out
        } else {
            let mut diagnostics = Vec::new();
            let res = format(&source, &options, Some(&mut diagnostics)).map_err(|e| {
                (
                    Some(input_name.clone()),
                    e.message().to_string(),
                    if e.line() > 0 { Some(e.line()) } else { None },
                )
            })?;
            for diag in diagnostics {
                eprintln!(
                    "{}:{}: warning: ambiguous label or multiplication; preserving whitespace",
                    input_name, diag.line
                );
                let _ = io::stderr().write_all(&diag.source);
                eprintln!();
            }
            res
        };

        if check {
            if output != source {
                has_diff = true;
                eprintln!("hspfmt: {}: formatting differs", input_name);
            }
        } else if write {
            if output != source {
                replacements.push((filename.clone(), output));
            }
        } else {
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            handle
                .write_all(&output)
                .and_then(|_| handle.flush())
                .map_err(|_| {
                    (
                        Some(input_name.clone()),
                        "output write failed".to_string(),
                        None,
                    )
                })?;
        }
    }

    for (fname, content) in replacements {
        replace_file(&fname, &content).map_err(|e| (Some(fname.display().to_string()), e, None))?;
    }

    if check {
        return Ok(if has_diff { 1 } else { 0 });
    }
    Ok(0)
}

fn main() {
    match run() {
        Ok(code) => process::exit(code),
        Err((input_name, msg, line)) => {
            eprint!("hspfmt: ");
            if let Some(name) = input_name {
                eprint!("{}", name);
                if let Some(l) = line {
                    eprint!(":{}", l);
                }
                eprintln!(": {}", msg);
            } else {
                eprintln!("{}", msg);
            }
            process::exit(2);
        }
    }
}
