# File Processor

A Rust-based file processor that recursively processes directories, applies syntax highlighting to files, and provides structured error handling using `anyhow` and `thiserror`.

## Features
- Recursively processes directories and files.
- Syntax highlighting using `syntect`.
- Custom error handling with `thiserror`.
- Configurable options like depth, file extensions, and exclusion lists.
- Colored output for better visibility.

## Installation
Ensure you have Rust installed. Then, clone the repository and build the project:

```sh
git clone <repository-url>
cd file-processor
cargo build --release
```

## Usage
Run the program with the desired directory:

```sh
cargo run -- <path> [options]
```

### Options:
- `<path>` (**optional**, default: `.`): The file or directory to process.
- `--depth <n>`: Depth level for recursive search.
- `--ext <extension>`: Filter files by extension.
- `--no-color`: Disable colored output.
- `--list`: List files instead of printing content.
- `--json`: Prints the listings in json format

## Example
```sh
cargo run -- ./src --ext rs --list
```
Lists all `.rs` files in the `src` directory.

## Dependencies
- `anyhow`: Simplified error handling.
- `thiserror`: Custom error types.
- `clap`: Command-line argument parsing.
- `colored`: Colorful terminal output.
- `log` and `simple_logger`: Logging.
- `syntect`: Syntax highlighting.
