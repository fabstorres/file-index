mod document;
mod fif;
mod index;
mod tokenizer;

use std::{
    fs::{self, File},
    io::{self, BufReader},
    path::Path,
};

use fif::{encode, reader::FifReader};
use index::InvertedIndex;

const INDEX_PATH: &str = "index.fif";

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: cargo run <token>");
        return Ok(());
    }

    let query_tokens = tokenizer::tokenize(&args[1]);

    let mut reader = open_or_create_index(Path::new(INDEX_PATH))?;

    for document_id in reader.and_search(&query_tokens)? {
        if let Some(path) = reader.document(document_id)? {
            println!("{path}");
        }
    }

    Ok(())
}

fn open_or_create_index(path: &Path) -> io::Result<FifReader<BufReader<File>>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let index = InvertedIndex::from_path(&std::env::current_dir().unwrap());
            let bytes = encode::try_from_inverted_index(&index)?;
            fs::write(path, bytes)?;
            File::open(path)?
        }
        Err(error) => return Err(error),
    };

    FifReader::new(BufReader::new(file))
}
