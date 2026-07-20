use std::io::{self, Read, Seek, SeekFrom};

const MAGIC: &[u8; 4] = b"FIF\0";
const SUPPORTED_VERSION: u32 = 1;
const HEADER_SIZE: u64 = 32;

struct Header {
    version: u32,
    document_count: u32,
    document_table_offset: u32,
    term_count: u32,
    term_table_offset: u32,
    posting_count: u32,
    posting_table_offset: u32,
}

/// A random-access reader for the format produced by `fif::encode`.
///
/// Terms and postings have the same index: posting entry `n` belongs to term
/// entry `n`. The encoder writes terms in sorted order, so term lookup can use
/// binary search without first loading the entire index.
pub struct FifReader<R: Read + Seek> {
    source: R,
    header: Header,
    source_len: u64,
}

impl<R: Read + Seek> FifReader<R> {
    pub fn new(mut source: R) -> io::Result<Self> {
        source.seek(SeekFrom::Start(0))?;

        let mut magic = [0; 4];
        source.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(invalid_data("not a FIF file"));
        }

        let header = Header {
            version: read_u32(&mut source)?,
            document_count: read_u32(&mut source)?,
            document_table_offset: read_u32(&mut source)?,
            term_count: read_u32(&mut source)?,
            term_table_offset: read_u32(&mut source)?,
            posting_count: read_u32(&mut source)?,
            posting_table_offset: read_u32(&mut source)?,
        };

        if header.version != SUPPORTED_VERSION {
            return Err(invalid_data(format!(
                "unsupported FIF version {}",
                header.version
            )));
        }
        if header.term_count != header.posting_count {
            return Err(invalid_data("term and posting counts differ"));
        }

        let source_len = source.seek(SeekFrom::End(0))?;
        if source_len < HEADER_SIZE {
            return Err(invalid_data("truncated FIF header"));
        }

        validate_table(
            source_len,
            header.document_table_offset,
            header.document_count,
            4,
            "document",
        )?;
        validate_table(
            source_len,
            header.term_table_offset,
            header.term_count,
            4,
            "term",
        )?;
        validate_table(
            source_len,
            header.posting_table_offset,
            header.posting_count,
            8,
            "posting",
        )?;

        Ok(Self {
            source,
            header,
            source_len,
        })
    }

    pub fn document(&mut self, index: u32) -> io::Result<Option<String>> {
        if index >= self.header.document_count {
            return Ok(None);
        }

        let entry = table_entry(self.header.document_table_offset, index, 4)?;
        let string_offset = self.read_u32_at(entry)?;
        self.read_string_at(string_offset).map(Some)
    }

    pub fn term(&mut self, index: u32) -> io::Result<Option<String>> {
        if index >= self.header.term_count {
            return Ok(None);
        }

        let entry = table_entry(self.header.term_table_offset, index, 4)?;
        let string_offset = self.read_u32_at(entry)?;
        self.read_string_at(string_offset).map(Some)
    }

    pub fn postings(&mut self, term_index: u32) -> io::Result<Option<Vec<u32>>> {
        if term_index >= self.header.posting_count {
            return Ok(None);
        }

        let entry = table_entry(self.header.posting_table_offset, term_index, 8)?;
        let postings_offset = self.read_u32_at(entry)?;
        let postings_count = self.read_u32_at(entry + 4)?;
        let byte_len = u64::from(postings_count)
            .checked_mul(4)
            .ok_or_else(|| invalid_data("posting list is too large"))?;
        ensure_range(
            self.source_len,
            u64::from(postings_offset),
            byte_len,
            "posting list",
        )?;

        self.source
            .seek(SeekFrom::Start(u64::from(postings_offset)))?;
        let capacity = usize::try_from(postings_count)
            .map_err(|_| invalid_data("posting count does not fit in memory"))?;
        let mut postings = Vec::with_capacity(capacity);
        for _ in 0..postings_count {
            postings.push(read_u32(&mut self.source)?);
        }
        Ok(Some(postings))
    }

    /// Finds a term and returns its index in the term/posting tables.
    pub fn find_term(&mut self, needle: &str) -> io::Result<Option<u32>> {
        let mut low = 0;
        let mut high = self.header.term_count;

        while low < high {
            let middle = low + (high - low) / 2;
            let term = self
                .term(middle)?
                .ok_or_else(|| invalid_data("term table changed while reading"))?;
            match term.as_str().cmp(needle) {
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
                std::cmp::Ordering::Equal => return Ok(Some(middle)),
            }
        }

        Ok(None)
    }

    pub fn postings_for_term(&mut self, term: &str) -> io::Result<Option<Vec<u32>>> {
        let Some(term_index) = self.find_term(term)? else {
            return Ok(None);
        };
        self.postings(term_index)
    }

    pub fn and_search(&mut self, query_terms: &[String]) -> io::Result<Vec<u32>> {
        let Some((first, rest)) = query_terms.split_first() else {
            return Ok(Vec::new());
        };
        let Some(mut matches) = self.postings_for_term(first)? else {
            return Ok(Vec::new());
        };

        for term in rest {
            let Some(postings) = self.postings_for_term(term)? else {
                return Ok(Vec::new());
            };
            matches.retain(|document_id| postings.binary_search(document_id).is_ok());
        }

        Ok(matches)
    }

    fn read_u32_at(&mut self, offset: u64) -> io::Result<u32> {
        ensure_range(self.source_len, offset, 4, "u32 field")?;
        self.source.seek(SeekFrom::Start(offset))?;
        read_u32(&mut self.source)
    }

    fn read_string_at(&mut self, offset: u32) -> io::Result<String> {
        let offset = u64::from(offset);
        ensure_range(self.source_len, offset, 1, "string")?;
        self.source.seek(SeekFrom::Start(offset))?;

        let mut bytes = Vec::new();
        for _ in offset..self.source_len {
            let mut byte = [0];
            self.source.read_exact(&mut byte)?;
            if byte[0] == 0 {
                return String::from_utf8(bytes)
                    .map_err(|_| invalid_data("FIF string is not valid UTF-8"));
            }
            bytes.push(byte[0]);
        }

        Err(invalid_data("unterminated FIF string"))
    }
}

fn read_u32<R: Read>(reader: &mut R) -> io::Result<u32> {
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn table_entry(table_offset: u32, index: u32, entry_size: u64) -> io::Result<u64> {
    u64::from(index)
        .checked_mul(entry_size)
        .and_then(|relative| u64::from(table_offset).checked_add(relative))
        .ok_or_else(|| invalid_data("table entry offset overflow"))
}

fn validate_table(
    source_len: u64,
    offset: u32,
    count: u32,
    entry_size: u64,
    name: &str,
) -> io::Result<()> {
    if count == 0 {
        return Ok(());
    }
    if u64::from(offset) < HEADER_SIZE {
        return Err(invalid_data(format!("invalid {name} table offset")));
    }
    let byte_len = u64::from(count)
        .checked_mul(entry_size)
        .ok_or_else(|| invalid_data(format!("{name} table is too large")))?;
    ensure_range(
        source_len,
        u64::from(offset),
        byte_len,
        &format!("{name} table"),
    )
}

fn ensure_range(source_len: u64, offset: u64, len: u64, name: &str) -> io::Result<()> {
    let end = offset
        .checked_add(len)
        .ok_or_else(|| invalid_data(format!("{name} offset overflow")))?;
    if end > source_len {
        return Err(invalid_data(format!("{name} lies outside the FIF file")));
    }
    Ok(())
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{fif::encode, index::InvertedIndex};
    use std::{fs, io::Cursor, time::SystemTime};

    #[test]
    fn reads_empty_encoded_index() {
        let bytes = encode::try_from_inverted_index(&InvertedIndex::default()).unwrap();
        let mut reader = FifReader::new(Cursor::new(bytes)).unwrap();

        assert_eq!(reader.document(0).unwrap(), None);
        assert_eq!(reader.term(0).unwrap(), None);
    }

    #[test]
    fn reads_documents_terms_and_postings_from_encoder() {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("file-index-reader-{unique}"));
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("alpha.txt"), []).unwrap();
        fs::write(directory.join("beta.txt"), []).unwrap();

        let index = InvertedIndex::from_path(&directory);
        let bytes = encode::try_from_inverted_index(&index).unwrap();
        let mut reader = FifReader::new(Cursor::new(bytes)).unwrap();

        let txt_index = reader.find_term("txt").unwrap().unwrap();
        assert_eq!(reader.term(txt_index).unwrap().as_deref(), Some("txt"));
        let postings = reader.postings_for_term("txt").unwrap().unwrap();
        assert_eq!(postings.len(), 2);
        for posting in postings {
            assert!(reader.document(posting).unwrap().unwrap().ends_with(".txt"));
        }
        assert_eq!(reader.and_search(&["txt".into()]).unwrap().len(), 2);
        assert!(
            reader
                .and_search(&["txt".into(), "missing".into()])
                .unwrap()
                .is_empty()
        );
        assert_eq!(reader.postings_for_term("missing").unwrap(), None);
        assert_eq!(reader.document(2).unwrap(), None);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn rejects_invalid_magic() {
        let mut bytes = vec![0; HEADER_SIZE as usize];
        bytes[..4].copy_from_slice(b"NOPE");

        let error = FifReader::new(Cursor::new(bytes)).err().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}
