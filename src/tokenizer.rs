pub fn tokenize(s: &str) -> Vec<String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_non_ascii_paths() {
        let cases = [
            (
                "/home/用户/文档/报告.txt",
                vec!["home", "用户", "文档", "报告", "txt"],
            ),
            (
                "/home/josé/música/canción.mp3",
                vec!["home", "josé", "música", "canción", "mp3"],
            ),
            ("/tmp/日本語/写真.png", vec!["tmp", "日本語", "写真", "png"]),
            (
                "/пользователь/документы/файл.rs",
                vec!["пользователь", "документы", "файл", "rs"],
            ),
        ];

        for (path, expected) in cases {
            assert_eq!(tokenize(path), expected, "failed for path: {path}");
        }
    }
}
