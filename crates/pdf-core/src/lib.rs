//! PDF logic for JJ PDF Studio.
//!
//! This crate is deliberately free of any Tauri dependency so it can be unit
//! tested anywhere and reused later (for example by a command-line tool).

/// Version of the core library, taken from `Cargo.toml` at compile time.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_not_empty_and_looks_like_semver() {
        let v = version();
        assert!(!v.is_empty());
        assert_eq!(v.split('.').count(), 3, "expected x.y.z, got {v}");
    }
}
