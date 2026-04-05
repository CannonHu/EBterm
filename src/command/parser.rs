//! Command file parser
//!
//! Parses command files to extract commands with support for comments and metadata.

use std::path::Path;

use super::CommandError;

/// Parsed command entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCommand {
    /// Line number in the source file (1-indexed)
    pub line_number: usize,
    /// The command content (after comment removal)
    pub content: String,
    /// Optional description from inline comment
    pub description: Option<String>,
}

impl ParsedCommand {
    /// Get a preview of the command content (truncated if too long)
    pub fn preview(&self, max_len: usize) -> String {
        if self.content.len() <= max_len {
            self.content.clone()
        } else {
            format!("{}...", &self.content[..max_len.saturating_sub(3)])
        }
    }
}

/// Trait for parsing command files
pub trait CommandParser: Send + Sync {
    /// Parse a command file and return list of commands
    fn parse_file(&self, path: &Path) -> Result<Vec<ParsedCommand>, CommandError>;

    /// Parse command content from a string
    fn parse_string(&self, content: &str) -> Result<Vec<ParsedCommand>, CommandError>;
}

/// Default command file parser implementation
pub struct DefaultCommandParser {
    max_file_size: u64,
    comment_prefixes: Vec<String>,
}

impl Default for DefaultCommandParser {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024, // 10MB
            comment_prefixes: vec!["//".to_string(), "#".to_string()],
        }
    }
}

impl DefaultCommandParser {
    /// Extract comment from a line if present
    fn extract_comment(&self, line: &str) -> (String, Option<String>) {
        for prefix in &self.comment_prefixes {
            if let Some(pos) = line.find(prefix) {
                let content = line[..pos].trim().to_string();
                let comment = line[pos + prefix.len()..].trim().to_string();
                return (
                    content,
                    if comment.is_empty() {
                        None
                    } else {
                        Some(comment)
                    },
                );
            }
        }
        (line.trim().to_string(), None)
    }

    /// Check if a line is empty or only whitespace
    fn is_empty_line(&self, line: &str) -> bool {
        line.trim().is_empty()
    }
}

impl CommandParser for DefaultCommandParser {
    fn parse_file(&self, path: &Path) -> Result<Vec<ParsedCommand>, CommandError> {
        // Check if file exists
        if !path.exists() {
            return Err(CommandError::FileNotFound {
                path: path.display().to_string(),
            });
        }

        // Check if file is too large
        let metadata = std::fs::metadata(path).map_err(|e| CommandError::ReadFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;

        if metadata.len() > self.max_file_size {
            return Err(CommandError::TooLarge {
                path: path.display().to_string(),
                size: metadata.len(),
                limit: self.max_file_size,
            });
        }

        // Read file content
        let content = std::fs::read_to_string(path).map_err(|e| CommandError::ReadFailed {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;

        // Check if file is empty
        if content.trim().is_empty() {
            return Err(CommandError::EmptyFile {
                path: path.display().to_string(),
            });
        }

        self.parse_string(&content)
    }

    fn parse_string(&self, content: &str) -> Result<Vec<ParsedCommand>, CommandError> {
        let mut commands = Vec::new();

        for (line_number, line) in content.lines().enumerate() {
            let line_number = line_number + 1;

            if self.is_empty_line(line) {
                continue;
            }

            let (content_part, description) = self.extract_comment(line);

            if content_part.is_empty() {
                continue;
            }

            commands.push(ParsedCommand {
                line_number,
                content: content_part.clone(),
                description,
            });
        }

        Ok(commands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_string_basic() {
        let parser = DefaultCommandParser::default();
        let content = "command1\ncommand2\ncommand3";
        let commands = parser.parse_string(content).unwrap();

        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0].content, "command1");
        assert_eq!(commands[1].content, "command2");
        assert_eq!(commands[2].content, "command3");
    }

    #[test]
    fn test_parse_string_with_comments() {
        let parser = DefaultCommandParser::default();
        let content = "command1 // description\ncommand2 # another desc";
        let commands = parser.parse_string(content).unwrap();

        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].content, "command1");
        assert_eq!(commands[0].description, Some("description".to_string()));
        assert_eq!(commands[1].content, "command2");
        assert_eq!(commands[1].description, Some("another desc".to_string()));
    }

    #[test]
    fn test_parse_string_skip_empty_lines() {
        let parser = DefaultCommandParser::default();
        let content = "command1\n\n\ncommand2\n   \ncommand3";
        let commands = parser.parse_string(content).unwrap();

        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0].content, "command1");
        assert_eq!(commands[1].content, "command2");
        assert_eq!(commands[2].content, "command3");
    }

    #[test]
    fn test_parse_string_skip_comment_only_lines() {
        let parser = DefaultCommandParser::default();
        let content = "command1\n// just a comment\ncommand2\n# another comment only";
        let commands = parser.parse_string(content).unwrap();

        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].content, "command1");
        assert_eq!(commands[1].content, "command2");
    }

    #[test]
    fn test_parse_string_line_numbers() {
        let parser = DefaultCommandParser::default();
        let content = "command1\n\ncommand2";
        let commands = parser.parse_string(content).unwrap();

        assert_eq!(commands[0].line_number, 1);
        assert_eq!(commands[1].line_number, 3);
    }

    #[test]
    fn test_parsed_command_preview() {
        let cmd = ParsedCommand {
            line_number: 1,
            content: "a very long command string that exceeds limit".to_string(),
            description: None,
        };

        let preview = cmd.preview(20);
        assert_eq!(preview, "a very long comma...");

        let short_preview = cmd.preview(100);
        assert_eq!(
            short_preview,
            "a very long command string that exceeds limit"
        );
    }

    #[test]
    fn test_parse_file_not_found() {
        let parser = DefaultCommandParser::default();
        let result = parser.parse_file(std::path::Path::new("/nonexistent/file.txt"));

        assert!(result.is_err());
        match result {
            Err(CommandError::FileNotFound { path }) => {
                assert!(path.contains("nonexistent"));
            }
            Err(_) => panic!("Expected FileNotFound error"),
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_parse_file_empty() {
        use std::fs::File;
        use std::io::Write;

        let parser = DefaultCommandParser::default();
        let test_file = std::path::Path::new("/tmp/test_empty_commands.txt");

        File::create(test_file)
            .unwrap()
            .write_all(b"   \n\n   ")
            .unwrap();
        let result = parser.parse_file(test_file);

        assert!(result.is_err());
        match result {
            Err(CommandError::EmptyFile { .. }) => (),
            _ => panic!("Expected EmptyFile error"),
        }

        std::fs::remove_file(test_file).unwrap();
    }
}
