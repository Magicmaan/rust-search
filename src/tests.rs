// use crate::database::*;
// use crate::index;
// use libsql::{Connection, Result};
// use timed::timed;

// #[cfg(test)]
// mod tests {
//     use crate::SearchEngine;

//     #[tokio::test]
//     async fn test_create_search_engine() {
//         let engine = SearchEngine::new(Some("test_search.db"), Some(true)).await;
//         assert!(engine.is_ok());
//     }

//     #[tokio::test]
//     async fn test_create_search_engine_default() {
//         let engine = SearchEngine::new(Some("".trim()), Some(true)).await;
//         assert!(engine.is_ok());
//     }

//     #[tokio::test]
//     async fn test_index_search_engine() {
//         index().await.unwrap();
//     }

//     #[tokio::test]
//     async fn test_search_query() {
//         let engine = SearchEngine::new(Some("search.db"), Some(true))
//             .await
//             .unwrap();
//         let query = "config";
//         let results = engine.search(query).await;
//         assert!(results.is_ok());
//         assert!(!results.unwrap().is_empty());
//     }

//     #[tokio::test]
//     async fn test_search_query_empty() {
//         let engine = SearchEngine::new(Some("test_search.db"), Some(true))
//             .await
//             .unwrap();
//         let query = "";
//         let results = engine.search(query).await;
//         assert!(results.is_ok());
//         assert!(results.unwrap().is_empty());
//     }

//     #[tokio::test]
//     async fn test_search_query_sql_erroneous() {
//         let engine = SearchEngine::new(Some("test_search.db"), Some(true))
//             .await
//             .unwrap();
//         let query = "SELECT * FROM non_existent_table";
//         let results = engine.search(query).await;
//         assert!(results.is_err());
//     }

//     #[tokio::test]
//     async fn test_search_query_unclosed_quotes() {
//         let engine = SearchEngine::new(Some("test_search.db"), Some(true))
//             .await
//             .unwrap();
//         let query = "'unclosed string";
//         let results = engine.search(query).await;
//         assert!(results.is_err());
//     }

//     #[tokio::test]
//     async fn test_search_query_double_quotes() {
//         let engine = SearchEngine::new(Some("test_search.db"), Some(true))
//             .await
//             .unwrap();
//         let query = "\"unclosed double quote";
//         let results = engine.search(query).await;
//         assert!(results.is_err());
//     }

//     #[tokio::test]
//     async fn test_search_query_semicolon_injection() {
//         let engine = SearchEngine::new(Some("test_search.db"), Some(true))
//             .await
//             .unwrap();
//         let query = "test; DROP TABLE test_search;";
//         let results = engine.search(query).await;
//         assert!(results.is_err());
//     }

//     #[tokio::test]
//     async fn test_search_query_backtick() {
//         let engine = SearchEngine::new(Some("test_search.db"), Some(true))
//             .await
//             .unwrap();
//         let query = "`backtick";
//         let results = engine.search(query).await;
//         assert!(results.is_err());
//     }

//     #[tokio::test]
//     async fn test_search_query_parentheses_mismatch() {
//         let engine = SearchEngine::new(Some("test_search.db"), Some(true))
//             .await
//             .unwrap();
//         let query = "(unclosed parenthesis";
//         let results = engine.search(query).await;
//     }
// }
