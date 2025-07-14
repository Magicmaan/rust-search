use timed::timed;
use libsql::{Connection, Result};
use crate::database::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test] #[timed]
    fn test_create_database() {
        let result = create_database();
        assert!(result.is_ok(), "Database creation failed: {:?}", result.err());
    }

    #[test] #[timed]
    fn test_insert_files_to_database() {
        let result = insert_files_to_database();
        assert!(result.is_ok(), "File insertion failed: {:?}", result.err());
    }

    #[test] #[timed]
    fn test_query_database() {
        let result = query_database("example query");
        assert!(result.is_ok(), "Database query failed: {:?}", result.err());
    }
}



fn _output_results() -> Result<()> {
    !todo!("Implement output results logic here");
    // print to console
    // output to file
}