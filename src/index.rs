use crate::document::Document;
use crate::tokenizer;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

type Posting = usize;

#[derive(Serialize, Deserialize)]
pub struct InvertedIndex {
    documents: Vec<Document>,
    postings: HashMap<String, Vec<Posting>>,
}

impl InvertedIndex {
    pub fn from_path(root: &Path) -> Self {
        let mut stack = vec![root.to_path_buf()];
        let mut documents = Vec::new();

        while let Some(path) = stack.pop() {
            if path.is_dir() {
                for entry in fs::read_dir(&path).unwrap() {
                    stack.push(entry.unwrap().path());
                }
            } else if path.is_file() {
                documents.push(Document::from_path(documents.len(), &path));
            }
        }

        let mut postings: HashMap<String, Vec<Posting>> = HashMap::new();

        for document in &documents {
            let tokens: HashSet<String> = tokenizer::tokenize(&document.file_path)
                .into_iter()
                .collect();

            for token in tokens {
                postings.entry(token).or_default().push(document.id);
            }
        }

        Self {
            documents,
            postings,
        }
    }

    pub fn and_search(&self, query_tokens: &[String]) -> Vec<&Document> {
        let Some((first, rest)) = query_tokens.split_first() else {
            return Vec::new();
        };

        let Some(first_postings) = self.postings.get(first) else {
            return Vec::new();
        };

        let mut matches = first_postings.clone();

        for token in rest {
            let Some(postings) = self.postings.get(token) else {
                return Vec::new();
            };
            matches.retain(|posting| postings.contains(posting));
        }

        matches
            .into_iter()
            .filter_map(|posting| self.documents.get(posting))
            .collect()
    }
}
