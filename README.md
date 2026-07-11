# File Index

A small, naive file search index written in Rust.

This project recursively scans the current working directory, stores each file as a document, tokenizes its full path, and builds an inverted index that maps tokens to matching files.

The goal of this project is to experiment with search indexing concepts while learning Rust.

## Installation

Clone the repository and build the project with Cargo:

```bash
git clone <repository-url>
cd file-index
cargo build
```

## Usage

Search for a token by passing it as a command-line argument:

```bash
cargo run -- <token>
```

Example:

```bash
cargo run -- rs
```

Example output:

```text
Document[228]: main.rs | src/main.rs
```

The project currently indexes the current working directory recursively.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
