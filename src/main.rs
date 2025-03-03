use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use log::{LevelFilter, debug};
use serde_json::json;
use simple_logger::SimpleLogger;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, BufRead};
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::as_24_bit_terminal_escaped;
use thiserror::Error;

/// Recursive rcat
#[derive(Parser)]
#[command(version)]
pub struct Args {
    /// directory name
    #[arg(value_name = "PATH", default_value = ".")]
    path: String,

    /// Disable syntax highlight
    #[arg(long)]
    no_color: bool,

    /// Filter by file extension
    #[arg(long)]
    ext: Option<String>,

    // Maximum recursion depth
    #[arg(long, short)]
    depth: Option<usize>,

    /// List files without parsing contents
    #[arg(long)]
    list: bool,

    /// log level
    #[arg(long,short,action=clap::ArgAction::Count)]
    verbose: u8,

    /// output directory tree in json format
    #[arg(long, short)]
    json: bool,
}

#[derive(Debug, Error)]
enum FileProcessorError {
    #[error("Failed to read directory: {0}")]
    DirectoryRead(#[from] io::Error),

    #[error("Syntax highlighting failed for {0}: {1}")]
    SyntaxHighlighting(String, syntect::Error),

    #[error("Path not found : {0}")]
    PathNotFound(String),
}

struct FileProcessor {
    no_color: bool,
    depth: Option<usize>,
    file_ext: Option<String>,
    list: bool,
    json: bool,
    excluded_files: HashSet<String>,
}

fn get_to_exclude() -> HashSet<String> {
    HashSet::from([
        "target".to_string(),
        ".idea".to_string(),
        ".vscode".to_string(),
        ".git".to_string(),
        "Cargo.lock".to_string(),
        ".gitignore".to_string(),
        ".github".to_string(),
    ])
}

type JsonMap = HashMap<String, serde_json::Value>;

impl FileProcessor {
    fn new(args: Args) -> Self {
        FileProcessor {
            no_color: args.no_color,
            depth: args.depth,
            file_ext: args.ext,
            list: args.list,
            json: args.json,
            excluded_files: get_to_exclude(),
        }
    }
    fn should_skip(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|f| f.to_str())
            .map(|name| self.excluded_files.contains(name))
            .unwrap_or(false)
    }
    fn print_separator(&self) {
        println!("\n{}\n", "━".repeat(50).cyan())
    }
    fn print_file_info(&self, path: &Path) {
        self.print_separator();
        println!(
            "{}  {}\n",
            "▶ OPENING FILE:".bold().yellow(),
            path.display().to_string().bold().green(),
        );

        self.print_separator()
    }
    fn print_file_contents(&self, path: &Path, no_color: bool) -> Result<()> {
        self.print_file_info(path);
        let file =
            fs::File::open(path).context(format!("Could not open file: {}", path.display()))?;
        let content = io::BufReader::new(file);
        if no_color {
            for line in content.lines() {
                let line = line?;
                println!("{:}", line);
            }
        } else {
            // Load syntaxes and themes
            let ps = SyntaxSet::load_defaults_nonewlines();
            let ts = ThemeSet::load_defaults();
            let theme = &ts.themes["base16-ocean.dark"];

            // detect the syntax
            let syntax = ps
                .find_syntax_for_file(path)?
                .unwrap_or(ps.find_syntax_plain_text());

            // highlight
            let mut h = HighlightLines::new(syntax, theme);
            for line in content.lines() {
                let line = line?;
                let highlighted = h.highlight_line(&line, &ps).map_err(|e| {
                    FileProcessorError::SyntaxHighlighting(path.display().to_string(), e)
                })?;
                let escaped = as_24_bit_terminal_escaped(&highlighted[..], false);
                println!("{}", escaped.trim_end());
            }
        }
        println!("\n{}\n", "[ END OF FILE ]".bold().red());
        Ok(())
    }
    fn generate_json(&self, path: &Path) -> serde_json::Value {
        let mut structure: JsonMap = HashMap::new();
        let mut files = vec![];
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                let name = entry.file_name().into_string().unwrap_or_default();

                if self.should_skip(path) {
                    debug!("skipping : {}", name);
                    continue;
                }

                if entry_path.is_dir() {
                    structure.insert(name, self.generate_json(&entry_path));
                } else {
                    files.push(name);
                }
            }
        }
        let mut result = json!({"files": files});
        for (key, value) in structure {
            result[key] = value;
        }
        serde_json::Value::Object(result.as_object().unwrap().clone())
    }
    fn proces_file(&self, path: &Path) -> Result<()> {
        if self.list {
            println!(
                "\n{} {}\n",
                "📄 File:".bold().blue(),
                path.display().to_string().bold().green()
            );
        } else {
            self.print_file_contents(path, self.no_color)?;
        }
        Ok(())
    }

    fn process_directory(&self, dir: &Path, depth: Option<usize>) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }
        for entry in fs::read_dir(dir).context("failed to read directory")? {
            let entry = entry?;
            let path = entry.path();
            // Extract just the last directory name
            if self.should_skip(&path) {
                continue;
            }
            if path.is_file() {
                debug!("file found {}", path.display());
                let file_extension = &path.extension().and_then(|s| s.to_str()).unwrap_or("");
                debug!("extracted file extension: {}", file_extension);
                if self
                    .file_ext
                    .as_ref()
                    .is_none_or(|ext| file_extension == ext)
                {
                    self.proces_file(&path)?;
                }
            }

            if path.is_dir() {
                debug!("directory found {}", path.display());
                if let Some(d) = depth {
                    if d > 0 {
                        self.process_directory(&path, Some(d - 1))?;
                    }
                } else {
                    // if depth is None, continue recursion
                    self.process_directory(&path, depth)?;
                }
            }
        }
        Ok(())
    }

    fn run(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            return Err(
                FileProcessorError::PathNotFound(path.to_str().unwrap().to_string()).into(),
            );
        }
        if self.json {
            let json_structure = self.generate_json(path);
            println!("{}", serde_json::to_string_pretty(&json_structure)?);
            Ok(())
        } else if path.is_dir() {
            self.process_directory(path, self.depth)
        } else {
            self.proces_file(path)
        }
    }
}
fn main() -> Result<()> {
    let args = Args::parse();
    let log_level = match args.verbose {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };
    SimpleLogger::new()
        .with_level(log_level)
        .with_colors(true)
        .init()?;
    let path = args.path.clone();
    let path = Path::new(&path);

    let processor = FileProcessor::new(args);
    processor.run(path)?;
    Ok(())
}
