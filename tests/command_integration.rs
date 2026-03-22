use embedded_debugger::command::{CommandParser, DefaultCommandParser};

mod common;
use common::*;

#[test]
fn test_parse_real_command_file() {
    // Given: a temporary directory and a command file with multiple commands
    let temp_dir = create_test_dir();
    let content = r#"
reset
init 0x1000

send 0x1234
read 4

verify
shutdown
"#;
    let file_path = create_test_file(&temp_dir, "commands.txt", content);

    // When: parsing the command file using DefaultCommandParser
    let parser = DefaultCommandParser::default();
    let commands = parser.parse_file(&file_path).expect("Failed to parse file");

    // Then: all commands should be parsed correctly with expected content
    assert_eq!(commands.len(), 6);
    assert_eq!(commands[0].content, "reset");
    assert_eq!(commands[1].content, "init 0x1000");
    assert_eq!(commands[2].content, "send 0x1234");
    assert_eq!(commands[3].content, "read 4");
    assert_eq!(commands[4].content, "verify");
    assert_eq!(commands[5].content, "shutdown");
}

#[test]
fn test_parse_file_with_comments() {
    // Given: a temporary directory and a file with inline comments
    let temp_dir = create_test_dir();
    let content = r#"
command1  // Inline comment
command2  # Another comment
"#;
    let file_path = create_test_file(&temp_dir, "comments.txt", content);

    // When: parsing the file with comments
    let parser = DefaultCommandParser::default();
    let commands = parser.parse_file(&file_path).expect("Failed to parse file");

    // Then: commands should be extracted and comments should be parsed as descriptions
    assert_eq!(commands.len(), 2);
    assert_eq!(commands[0].content, "command1");
    assert_eq!(commands[0].description, Some("Inline comment".to_string()));
    assert_eq!(commands[1].content, "command2");
    assert_eq!(commands[1].description, Some("Another comment".to_string()));
}
