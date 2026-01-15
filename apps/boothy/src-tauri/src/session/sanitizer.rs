use regex::Regex;

/// Sanitizes session name into a valid folder name
///
/// Strategy:
/// 1. Trim whitespace from start/end
/// 2. Reject if empty after trim
/// 3. Replace internal whitespace with '-'
/// 4. Keep only [A-Za-z0-9_-], replace others with '-'
/// 5. Collapse repeated '-' into single '-'
/// 6. Trim '-' from ends
pub fn sanitize_session_name(session_name: &str) -> Result<String, String> {
    // Step 1: Trim
    let trimmed = session_name.trim();

    // Step 2: Reject if empty
    if trimmed.is_empty() {
        return Err("Session name cannot be empty".to_string());
    }

    // Step 3 & 4: Replace whitespace and invalid chars
    let mut sanitized = String::new();
    for ch in trimmed.chars() {
        if ch.is_whitespace() {
            sanitized.push('-');
        } else if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            sanitized.push(ch);
        } else {
            sanitized.push('-');
        }
    }

    // Step 5: Collapse repeated '-'
    let collapse_regex = Regex::new(r"-+").unwrap();
    let collapsed = collapse_regex.replace_all(&sanitized, "-");

    // Step 6: Trim '-' from ends
    let final_name = collapsed.trim_matches('-').to_string();

    // Final check: not empty
    if final_name.is_empty() {
        return Err("Session name resulted in empty folder name after sanitization".to_string());
    }

    Ok(final_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_normal() {
        assert_eq!(
            sanitize_session_name("Test Session").unwrap(),
            "Test-Session"
        );
    }

    #[test]
    fn test_sanitize_special_chars() {
        assert_eq!(
            sanitize_session_name("Test@Session#123").unwrap(),
            "Test-Session-123"
        );
    }

    #[test]
    fn test_sanitize_repeated_dashes() {
        assert_eq!(
            sanitize_session_name("Test   Session").unwrap(),
            "Test-Session"
        );
    }

    #[test]
    fn test_sanitize_trim_dashes() {
        assert_eq!(sanitize_session_name("---Test---").unwrap(), "Test");
    }

    #[test]
    fn test_sanitize_empty_after_trim() {
        assert!(sanitize_session_name("   ").is_err());
    }

    #[test]
    fn test_sanitize_only_special_chars() {
        assert!(sanitize_session_name("@#$%").is_err());
    }
}
