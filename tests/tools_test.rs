use figma_mcp::tools::{
    coalesce, infer_format, resolve_output_path, to_string_slice, write_base64,
};
use serde_json::json;

#[test]
fn test_to_string_slice() {
    let arr = vec![json!("a"), json!("b"), json!("c")];
    let result = to_string_slice(&arr);
    assert_eq!(result, vec!["a", "b", "c"]);
}

#[test]
fn test_to_string_slice_with_non_strings() {
    let arr = vec![json!("a"), json!(42), json!("b"), json!(true)];
    let result = to_string_slice(&arr);
    assert_eq!(result, vec!["a", "b"]);
}

#[test]
fn test_infer_format_png() {
    assert_eq!(infer_format("image.png"), "PNG");
}

#[test]
fn test_infer_format_jpg() {
    assert_eq!(infer_format("image.jpg"), "JPG");
    assert_eq!(infer_format("image.jpeg"), "JPG");
}

#[test]
fn test_infer_format_svg() {
    assert_eq!(infer_format("image.svg"), "SVG");
}

#[test]
fn test_infer_format_pdf() {
    assert_eq!(infer_format("doc.pdf"), "PDF");
}

#[test]
fn test_infer_format_unknown() {
    assert_eq!(infer_format("image.txt"), "");
    assert_eq!(infer_format("noextension"), "");
}

#[test]
fn test_coalesce_first_non_empty() {
    assert_eq!(coalesce("a", "b"), "a");
}

#[test]
fn test_coalesce_first_empty() {
    assert_eq!(coalesce("", "b"), "b");
}

#[test]
fn test_coalesce_both_empty() {
    assert_eq!(coalesce("", ""), "");
}

#[test]
fn test_resolve_output_path_relative() {
    let work_dir = "/tmp/test-workdir";
    let result = resolve_output_path("output/test.png", work_dir);
    assert!(result.is_ok());
    let path = result.unwrap();
    assert!(path.starts_with(work_dir));
}

#[test]
fn test_resolve_output_path_absolute_inside() {
    let work_dir = "/tmp/test-workdir";
    let result = resolve_output_path("/tmp/test-workdir/output/test.png", work_dir);
    assert!(result.is_ok());
}

#[test]
fn test_resolve_output_path_traversal() {
    let work_dir = "/tmp/test-workdir";
    let result = resolve_output_path("../../../etc/passwd", work_dir);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("must be inside the working directory"));
}

#[test]
fn test_write_base64_creates_file() {
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("figma_mcp_test_write_base64.txt");

    // Remove if exists from previous run
    let _ = std::fs::remove_file(&test_file);

    let data = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        b"hello world",
    );

    let result = write_base64(&data, test_file.to_str().unwrap());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 11);

    let content = std::fs::read_to_string(&test_file).unwrap();
    assert_eq!(content, "hello world");

    // Writing again should fail (O_EXCL)
    let result2 = write_base64(&data, test_file.to_str().unwrap());
    assert!(result2.is_err());

    let _ = std::fs::remove_file(&test_file);
}
