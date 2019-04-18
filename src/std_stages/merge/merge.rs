use crate::protocol::Schema;

#[derive(Debug)]
pub struct Merge {
    pub schema: Schema,
    pub inlets_count: usize,
    pub eagerly_complete: bool,
    pub eagerly_fail: bool,
}

impl Merge {
    pub fn new(
        schema: Schema,
        inlets_count: usize,
        eagerly_complete: bool,
        eagerly_fail: bool,
    ) -> Self {
        Self {
            schema,
            inlets_count,
            eagerly_complete,
            eagerly_fail,
        }
    }
}
