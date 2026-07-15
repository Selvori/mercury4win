// mercury4win-linux/src-tauri/agent/prompt_templates.rs
// YAML prompt template loading with conditional section rendering
// Mirrors Mercury's prompt governance: template owns final message text,
// executors only choose parameter values.

use std::collections::HashMap;
use std::path::PathBuf;
use serde::Deserialize;

/// A parsed YAML prompt template.
#[derive(Debug, Clone, Deserialize)]
pub struct PromptTemplate {
    pub version: u32,
    #[serde(default)]
    pub system: Option<String>,
    pub user: String,
}

/// Resolved template with rendered messages ready for LLM request.
#[derive(Debug, Clone)]
pub struct RenderedPrompt {
    pub system_message: Option<String>,
    pub user_message: String,
}

impl PromptTemplate {
    /// Render the template with provided parameters. Supports:
    /// - `{{key}}` simple substitution
    /// - `{{#key}}...{{/key}}` conditional sections
    pub fn render(&self, params: &HashMap<String, String>) -> RenderedPrompt {
        let system_message = self
            .system
            .as_ref()
            .map(|tmpl| render_template(tmpl, params));

        let user_message = render_template(&self.user, params);

        let system_message = system_message.filter(|s| !s.is_empty());
        RenderedPrompt {
            system_message,
            user_message,
        }
    }
}

/// Render a template string with {{key}} substitution and {{#key}}...{{/key}} conditionals.
fn render_template(template: &str, params: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    let mut pending = String::new();

    while let Some(ch) = chars.next() {
        if ch == '{' && chars.peek() == Some(&'{') {
            chars.next(); // consume second '{'
            // Collect the key
            let mut key = String::new();
            loop {
                match chars.next() {
                    Some('}') if chars.peek() == Some(&'}') => {
                        chars.next(); // consume second '}'
                        break;
                    }
                    Some(c) => key.push(c),
                    None => break,
                }
            }

            // Check if this is a conditional section opener {{#key}}
            if let Some(cond_key) = key.strip_prefix('#') {
                // Find the body and closing tag
                let closing_tag = format!("{{{{/{}}}}}", cond_key);
                let mut body = String::new();
                let mut depth = 1u32;
                while depth > 0 {
                    match chars.next() {
                        Some(c) => {
                            body.push(c);
                            // Simple check for closing tag inside body
                            if body.ends_with(&closing_tag) {
                                depth -= 1;
                                if depth == 0 {
                                    // Remove the closing tag from body
                                    body.truncate(body.len() - closing_tag.len());
                                    break;
                                }
                            }
                            // Check for nested section
                            if body.ends_with(&format!("{{{{#{}}}}}", cond_key)) {
                                // Not supporting nested same-key
                            }
                        }
                        None => break,
                    }
                }

                // If the key has a non-empty value, render the body
                if let Some(value) = params.get(cond_key) {
                    if !value.is_empty() {
                        result.push_str(&render_template(&body, params));
                    }
                }
            } else {
                // Simple {{key}} substitution
                let value = params.get(&key).map(|s| s.as_str()).unwrap_or("");
                result.push_str(value);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Load built-in prompt template from resources.
pub fn load_builtin_template(
    agent_type: &str,
    strategy: Option<&str>,
) -> Result<PromptTemplate, String> {
    let template_yaml = match (agent_type, strategy) {
        ("translation", Some("hy-mt-optimized")) => {
            include_str!("../../resources/prompts/translation.hy-mt.yaml")
        }
        ("translation", _) => {
            include_str!("../../resources/prompts/translation.default.yaml")
        }
        ("summary", _) => {
            include_str!("../../resources/prompts/summary.default.yaml")
        }
        ("tagging", _) => {
            include_str!("../../resources/prompts/tagging.default.yaml")
        }
        _ => return Err(format!("Unknown agent type: {}", agent_type)),
    };

    serde_yaml::from_str(template_yaml).map_err(|e| format!("Template parse error: {}", e))
}

/// Load a custom prompt template from disk.
pub fn load_custom_template(
    prompts_dir: &std::path::PathBuf,
    agent_type: &str,
) -> Result<Option<PromptTemplate>, String> {
    let path = prompts_dir.join(format!("{}.custom.yaml", agent_type));
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Read error: {}", e))?;
    let template = serde_yaml::from_str(&content)
        .map_err(|e| format!("Parse error: {}", e))?;
    Ok(Some(template))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_substitution() {
        let mut params = HashMap::new();
        params.insert("name".into(), "Alice".into());
        let result = render_template("Hello {{name}}!", &params);
        assert_eq!(result, "Hello Alice!");
    }

    #[test]
    fn test_conditional_present() {
        let mut params = HashMap::new();
        params.insert("byline".into(), "John Doe".into());
        let template = "Title\n{{#byline}}By: {{byline}}{{/byline}}";
        let result = render_template(template, &params);
        assert_eq!(result, "Title\nBy: John Doe");
    }

    #[test]
    fn test_conditional_absent() {
        let params = HashMap::new();
        let template = "Title\n{{#byline}}By: {{byline}}{{/byline}}";
        let result = render_template(template, &params);
        assert_eq!(result.trim(), "Title");
    }

    #[test]
    fn test_template_render() {
        let template = PromptTemplate {
            version: 1,
            system: Some("You are a translator. Target: {{target_language}}".into()),
            user: "Translate:\n{{source_text}}\n{{#previous}}Context: {{previous}}{{/previous}}".to_string(),
        };

        let mut params = HashMap::new();
        params.insert("target_language".into(), "Chinese".into());
        params.insert("source_text".into(), "Hello world".into());
        params.insert("previous".into(), "Bonjour".into());

        let rendered = template.render(&params);
        assert!(rendered.system_message.unwrap().contains("Chinese"));
        assert!(rendered.user_message.contains("Hello world"));
        assert!(rendered.user_message.contains("Context: Bonjour"));
    }

    #[test]
    fn test_conditional_empty_value() {
        let mut params = HashMap::new();
        params.insert("byline".into(), "".into());
        let template = "{{#byline}}By: {{byline}}{{/byline}}";
        let result = render_template(template, &params);
        assert!(result.is_empty() || result == "");
    }

    #[test]
    fn test_load_builtin_summary() {
        let template = load_builtin_template("summary", None).unwrap();
        assert_eq!(template.version, 1);
        assert!(template.system.is_some());
        assert!(!template.user.is_empty());
    }

    #[test]
    fn test_load_builtin_translation() {
        let template = load_builtin_template("translation", None).unwrap();
        assert_eq!(template.version, 4);
        assert!(!template.user.is_empty());
    }

    #[test]
    fn test_load_builtin_tagging() {
        let template = load_builtin_template("tagging", None).unwrap();
        assert_eq!(template.version, 1);
        assert!(!template.user.is_empty());
    }

    #[test]
    fn test_load_hy_mt() {
        let template = load_builtin_template("translation", Some("hy-mt-optimized")).unwrap();
        assert_eq!(template.version, 1);
        assert!(!template.user.is_empty());
    }
}
