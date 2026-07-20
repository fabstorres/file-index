use std::io::{self, Cursor, Seek, SeekFrom, Write};

use crate::index::InvertedIndex;

fn reserve_header<W: Write>(writer: &mut W) -> io::Result<()> {
    writer.write_all(b"FIF\0")?;
    writer.write_all(&1_u32.to_le_bytes())?;
    writer.write_all(&0_u32.to_le_bytes())?;
    writer.write_all(&32_u32.to_le_bytes())?;
    writer.write_all(&0_u32.to_le_bytes())?;
    writer.write_all(&0_u32.to_le_bytes())?;
    writer.write_all(&0_u32.to_le_bytes())?;
    writer.write_all(&0_u32.to_le_bytes())?;
    Ok(())
}

fn reserve_offsets<W: Write>(
    writer: &mut W,
    entry_count: usize,
    u32s_per_entry: usize,
) -> io::Result<()> {
    for _ in 0..entry_count {
        for _ in 0..u32s_per_entry {
            writer.write_all(&0_u32.to_le_bytes())?;
        }
    }

    Ok(())
}

pub fn try_from_inverted_index(index: &InvertedIndex) -> io::Result<Vec<u8>> {
    let mut cursor = Cursor::new(Vec::new());
    let mut terms: Vec<_> = index.postings().iter().collect();
    terms.sort_unstable_by_key(|(term, _)| *term);

    reserve_header(&mut cursor)?;
    let document_table_offset = cursor.position();
    reserve_offsets(&mut cursor, index.documents().len(), 1)?;
    let terms_table_offset = cursor.position();
    reserve_offsets(&mut cursor, terms.len(), 1)?;
    let posting_table_offset = cursor.position();
    reserve_offsets(&mut cursor, terms.len(), 2)?;

    cursor.seek(SeekFrom::Start(8))?;
    cursor.write_all(&(index.documents().len() as u32).to_le_bytes())?;
    cursor.seek(SeekFrom::Start(16))?;
    cursor.write_all(&(terms.len() as u32).to_le_bytes())?;
    if !terms.is_empty() {
        cursor.write_all(&(terms_table_offset as u32).to_le_bytes())?;
    } else {
        cursor.seek(SeekFrom::Current(4))?;
    }
    cursor.write_all(&(terms.len() as u32).to_le_bytes())?;
    if !terms.is_empty() {
        cursor.write_all(&(posting_table_offset as u32).to_le_bytes())?;
    }

    let mut pc = document_table_offset;
    for document in index.documents() {
        let raw_offset = cursor.seek(SeekFrom::End(0))?;
        cursor.seek(SeekFrom::Start(pc))?;
        cursor.write_all(&(raw_offset as u32).to_le_bytes())?;
        cursor.seek(SeekFrom::End(0))?;
        cursor.write_all(document.file_path.as_bytes())?;
        pc += size_of::<u32>() as u64;
    }

    pc = terms_table_offset;
    for (term, _) in &terms {
        let raw_offset = cursor.seek(SeekFrom::End(0))?;
        cursor.seek(SeekFrom::Start(pc))?;
        cursor.write_all(&(raw_offset as u32).to_le_bytes())?;
        cursor.seek(SeekFrom::End(0))?;
        cursor.write_all(term.as_bytes())?;
        pc += size_of::<u32>() as u64;
    }

    pc = posting_table_offset;
    for (_, postings) in terms {
        let raw_offset = cursor.seek(SeekFrom::End(0))?;
        cursor.seek(SeekFrom::Start(pc))?;
        cursor.write_all(&(raw_offset as u32).to_le_bytes())?;
        cursor.write_all(&(postings.len() as u32).to_le_bytes())?;
        cursor.seek(SeekFrom::End(0))?;
        for &posting in postings {
            cursor.write_all(&(posting as u32).to_le_bytes())?;
        }
        pc += (2 * size_of::<u32>()) as u64;
    }

    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reserves_fif_header() {
        let mut bytes = Vec::new();

        reserve_header(&mut bytes).unwrap();

        assert_eq!(
            bytes,
            [
                b'F', b'I', b'F', 0, // magic
                1, 0, 0, 0, // version
                0, 0, 0, 0, // Document Count
                32, 0, 0, 0, // Document Table Offset
                0, 0, 0, 0, // Term Count
                0, 0, 0, 0, // Term Table Offset
                0, 0, 0, 0, // Postings Count
                0, 0, 0, 0, // Postings Table Offset
            ]
        );
    }
    #[test]
    fn reserves_offsets_section() {
        let mut document_offsets = Vec::new();
        let mut term_offsets = Vec::new();
        let mut posting_offsets = Vec::new();

        reserve_offsets(&mut document_offsets, 3, 1).unwrap();
        reserve_offsets(&mut term_offsets, 5, 1).unwrap();
        reserve_offsets(&mut posting_offsets, 7, 2).unwrap();

        assert_eq!(document_offsets, vec![0; 3 * size_of::<u32>()]);
        assert_eq!(term_offsets, vec![0; 5 * size_of::<u32>()]);
        assert_eq!(posting_offsets, vec![0; 7 * 2 * size_of::<u32>()]);
    }
    #[test]
    fn empty_binary_index_format() {
        let index = InvertedIndex::default();
        let bytes = try_from_inverted_index(&index).unwrap();

        assert_eq!(bytes.len(), 32);
        assert_eq!(&bytes[0..4], b"FIF\0");

        let fields: Vec<u32> = bytes[4..]
            .chunks_exact(size_of::<u32>())
            .map(|field| u32::from_le_bytes(field.try_into().unwrap()))
            .collect();

        assert_eq!(fields, [1, 0, 32, 0, 0, 0, 0]);
    }

    #[test]
    fn inspect_binary_index_from_current_directory() {
        let current_directory = std::env::current_dir().unwrap();
        let index = InvertedIndex::from_path(&current_directory);
        let bytes = try_from_inverted_index(&index).unwrap();

        println!("{bytes:?}");
    }
}
