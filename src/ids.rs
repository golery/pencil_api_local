/// Deterministic positive 32-bit id. Matches the previous JavaScript FNV-1a
/// implementation, including UTF-16 code units and wrapping 32-bit multiply.
pub fn path_to_id(key: &str) -> u32 {
    let mut hash: u32 = 2_166_136_261;
    for unit in key.encode_utf16() {
        hash ^= u32::from(unit);
        hash = hash.wrapping_mul(16_777_619);
    }
    if hash == 0 {
        1
    } else {
        hash
    }
}

pub fn book_id_for(folder_path: &str) -> u32 {
    path_to_id(&format!("book:{folder_path}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_javascript_vectors() {
        assert_eq!(path_to_id("book:/tmp/Personal"), 1_342_490_085);
        assert_eq!(
            book_id_for("/home/hly/repos/pencil_api_local/data/books/Personal"),
            2_154_611_156
        );
        assert_eq!(path_to_id("/tmp/Personal:."), 2_420_233_300);
        assert_eq!(path_to_id("/tmp/Personal:Welcome.md"), 4_242_283_889);
        assert_eq!(path_to_id("café"), 856_211_068);
        assert_eq!(path_to_id("a😀b"), 2_412_414_209);
    }
}
