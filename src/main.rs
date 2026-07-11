use std::collections::HashSet;
use std::fs;
use std::path::Path;

struct Document {
    id: usize,
    file_name: String,
    file_path: String,
}

type Posting = usize;

type InvertedIndex = std::collections::HashMap<String, Vec<Posting>>;

fn tokenize(s: &str) -> Vec<String> {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect();
    cleaned.split_whitespace().map(String::from).collect()
}

fn document_path(id: usize, path: &Path) -> Document {
    Document {
        id,
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

    let Some(find_token) = tokenize(&args[1]).into_iter().next() else {
        eprintln!("Search query contains no searchable token");
        return;
    };

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
        let tokens: HashSet<String> = tokenize(&doc.file_path).into_iter().collect();
        for token in tokens {
            index.entry(token).or_default().push(doc.id)
        }
    }

    if let Some(postings) = index.get(&find_token) {
        for &posting in postings {
            println!(
                "Document[{}]: {} | {}",
                posting, files[posting].file_name, files[posting].file_path,
            );
        }
    }
}
