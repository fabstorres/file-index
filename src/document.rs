use std::path::Path;

pub struct Document {
    pub id: usize,
    pub file_name: String,
    pub file_path: String,
}

impl Document {
    pub fn from_path(id: usize, path: &Path) -> Self {
        Self {
            id,
            file_name: path.file_name().unwrap().to_string_lossy().into_owned(),
            file_path: path.to_string_lossy().into_owned(),
        }
    }
}
