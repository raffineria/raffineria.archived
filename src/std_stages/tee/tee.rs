use crate::protocol::Schema;

#[derive(Debug)]
pub struct Tee {
    pub schema: Schema,
    pub outlets_count: usize,
}

impl Tee {
    pub fn new(schema: Schema, outlets_count: usize) -> Self {
        Self {
            schema,
            outlets_count,
        }
    }
}
