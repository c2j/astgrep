use super::*;

#[tokio::test]
async fn test_playground() {
    let result = playground().await;
    assert!(result.is_ok());

    let html = result.unwrap().0;
    assert!(html.contains("astgrep Playground"));
    assert!(html.contains("analyzeCode"));
}
