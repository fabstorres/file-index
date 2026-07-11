use std::fs;
use std::path::PathBuf;

struct Document {
    id: usize,
    file_name: String,
    file_path: String,
}

type Posting = usize;

type InvertedIndex = std::collections::HashMap<String, Vec<Posting>>;

fn normalize_path(path: &String) -> String {
    let filtered: String = path
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    filtered.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn document_path(id: usize, path: &PathBuf) -> Document {
    Document {
        id: id,
        file_name: path.file_name().unwrap().to_string_lossy().into_owned(),
        file_path: path.to_string_lossy().into_owned(),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: cargo run <token>");
        return;
    }

    let find_token = &args[1];

    let mut stack = Vec::new();
    let mut files = Vec::new();

    let mut index: InvertedIndex = std::collections::HashMap::new();

    stack.push(std::env::current_dir().unwrap());

    while let Some(path) = stack.pop() {
        if path.is_dir() {
            for entry in fs::read_dir(&path).unwrap() {
                stack.push(entry.unwrap().path())
            }
        } else if path.is_file() {
            files.push(document_path(files.len(), &path));
        }
    }

    for doc in &files {
        let norm = normalize_path(&doc.file_path);
        let tokens = norm.split_whitespace();
        for token in tokens {
            index.entry(token.to_string()).or_default().push(doc.id)
        }
    }

    if let Some(postings) = index.get(find_token) {
        for &posting in postings {
            println!(
                "Document[{}]: {} | {}",
                posting, files[posting].file_name, files[posting].file_path,
            );
        }
    }
}
