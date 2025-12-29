use serde::de::DeserializeOwned;

/// Robustly extracts and parses JSON from an LLM response.
/// Handles: Markdown code blocks, conversational intro/outro, and brace-counting for nested JSON.
pub fn extract_json<T: DeserializeOwned>(response: &str) -> Result<T, String> {
    let trimmed = response.trim();

    // Stage 1: Try direct parse (ideal case - pure JSON response)
    if let Ok(parsed) = serde_json::from_str::<T>(trimmed) {
        return Ok(parsed);
    }

    // Stage 2: Remove markdown code blocks if present
    let cleaned = remove_markdown_blocks(trimmed);
    if let Ok(parsed) = serde_json::from_str::<T>(&cleaned) {
        return Ok(parsed);
    }

    // Stage 3: Use brace-counting to find outermost { } pair
    if let Some(json_str) = find_json_object(&cleaned) {
        if let Ok(parsed) = serde_json::from_str::<T>(json_str) {
            return Ok(parsed);
        }
    }

    // Stage 4: Try finding JSON in the original response (in case markdown removal broke something)
    if let Some(json_str) = find_json_object(trimmed) {
        if let Ok(parsed) = serde_json::from_str::<T>(json_str) {
            return Ok(parsed);
        }
    }

    Err(format!(
        "Failed to extract valid JSON from response. Preview: {}...",
        &trimmed.chars().take(200).collect::<String>()
    ))
}

/// Remove markdown code blocks (```json ... ``` or ``` ... ```)
fn remove_markdown_blocks(text: &str) -> String {
    let mut result = text.to_string();

    // Remove ```json or ``` at the start
    if result.starts_with("```json") {
        result = result.strip_prefix("```json").unwrap_or(&result).to_string();
    } else if result.starts_with("```") {
        result = result.strip_prefix("```").unwrap_or(&result).to_string();
    }

    // Remove ``` at the end
    result = result.trim().to_string();
    if result.ends_with("```") {
        result = result.strip_suffix("```").unwrap_or(&result).to_string();
    }

    result.trim().to_string()
}

/// Find the outermost JSON object using brace counting
fn find_json_object(text: &str) -> Option<&str> {
    let mut brace_count = 0;
    let mut start_idx: Option<usize> = None;

    for (i, ch) in text.char_indices() {
        match ch {
            '{' => {
                if brace_count == 0 {
                    start_idx = Some(i);
                }
                brace_count += 1;
            }
            '}' => {
                brace_count -= 1;
                if brace_count == 0 {
                    if let Some(start) = start_idx {
                        return Some(&text[start..=i]);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestPlan {
        description: String,
        operations: Vec<TestOp>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestOp {
        #[serde(rename = "type")]
        op_type: String,
        path: Option<String>,
    }

    #[test]
    fn test_pure_json() {
        let input = r#"{"description": "test", "operations": []}"#;
        let result: TestPlan = extract_json(input).unwrap();
        assert_eq!(result.description, "test");
    }

    #[test]
    fn test_markdown_code_block() {
        let input = r#"```json
{"description": "test", "operations": []}
```"#;
        let result: TestPlan = extract_json(input).unwrap();
        assert_eq!(result.description, "test");
    }

    #[test]
    fn test_json_with_text_before() {
        let input = r#"Here is the plan:
{"description": "test", "operations": []}"#;
        let result: TestPlan = extract_json(input).unwrap();
        assert_eq!(result.description, "test");
    }

    #[test]
    fn test_nested_json() {
        let input = r#"{"description": "nested", "operations": [{"type": "move", "path": "/test"}]}"#;
        let result: TestPlan = extract_json(input).unwrap();
        assert_eq!(result.operations.len(), 1);
        assert_eq!(result.operations[0].op_type, "move");
    }
}
