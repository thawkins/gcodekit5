use gcodekit5_gcodeeditor::TextBuffer;

#[test]
fn test_create_empty() {
    let buffer = TextBuffer::new();
    assert_eq!(buffer.len_chars(), 0);
    assert!(buffer.is_empty());
}

#[test]
fn test_create_from_str() {
    let buffer = TextBuffer::from("Hello\nWorld");
    assert_eq!(buffer.len_lines(), 2);
    assert_eq!(buffer.line(0), Some("Hello\n".to_string()));
}

#[test]
fn test_insert() {
    let mut buffer = TextBuffer::from("Hello");
    buffer.insert(5, " World");
    assert_eq!(buffer.to_string(), "Hello World");
}

#[test]
fn test_delete() {
    let mut buffer = TextBuffer::from("Hello World");
    buffer.delete(5..11);
    assert_eq!(buffer.to_string(), "Hello");
}

#[test]
fn test_replace() {
    let mut buffer = TextBuffer::from("Hello World");
    buffer.replace(6..11, "Rust");
    assert_eq!(buffer.to_string(), "Hello Rust");
}

#[test]
fn test_append() {
    let mut buffer = TextBuffer::from("Hello");
    buffer.append(" World");
    assert_eq!(buffer.to_string(), "Hello World");
}

#[test]
fn test_line_col_conversion() {
    let buffer = TextBuffer::from("Line 1\nLine 2\nLine 3");
    let (line, col) = buffer.char_to_line_col(7);
    assert_eq!(line, 1);
    assert_eq!(col, 0);

    let char_idx = buffer.line_col_to_char(1, 0);
    assert_eq!(char_idx, 7);
}
