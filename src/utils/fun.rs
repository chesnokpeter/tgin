pub fn hide_segment(input: &str) -> String {
    if let Some(colon_idx) = input.find(':') {
        if let Some(slash_idx) = input[colon_idx..].find('/') {
            let start = colon_idx;
            let end = colon_idx + slash_idx;
            let before = &input[..start];
            let after = &input[end..];
            let hidden_len = end - start;
            let hidden = "*".repeat(hidden_len);
            format!("{}{}{}", before, hidden, after)
        } else {
            let before = &input[..colon_idx];
            let hidden = "*".repeat(input.len() - colon_idx);
            format!("{}{}", before, hidden)
        }
    } else {
        input.to_string()
    }
}

pub fn mask_secret(input: &str, secret: &str) -> String {
    if secret.is_empty() {
        return input.to_string();
    }
    let mask = "*".repeat(secret.chars().count());
    input.replace(secret, &mask)
}