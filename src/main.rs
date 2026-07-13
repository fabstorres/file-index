mod document;
mod index;
mod tokenizer;

use std::fs;

use index::InvertedIndex;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: cargo run <token>");
        return;
    }

    let query_tokens = tokenizer::tokenize(&args[1]);

    let index = match fs::read_to_string("index.json") {
        Ok(contents) => serde_json::from_str(&contents).unwrap(),
        Err(_err) => {
            let index = InvertedIndex::from_path(&std::env::current_dir().unwrap());
            let contents = serde_json::to_string(&index).unwrap();
            let _ = fs::write("index.json", contents).unwrap();
            index
        }
    };

    for document in index.and_search(&query_tokens) {
        println!(
            "Document[{}] {} | {}",
            document.id, document.file_name, document.file_path
        );
    }
}
