pub use serde_json::Value as JsonValue;
use std::io;

use crate::spec::GraphSpec;

use super::FormatSupport;

impl FormatSupport<JsonValue> for GraphSpec {
    type SaveError = failure::Error;
    type LoadError = failure::Error;

    fn save(&self) -> Result<JsonValue, Self::SaveError> {
        serde_json::to_value(self.clone()).map_err(|reason| reason.into())
    }

    fn load(data: &JsonValue) -> Result<Self, Self::LoadError> {
        serde_json::from_value(data.clone()).map_err(|reason| reason.into())
    }
}

pub fn read<R: io::Read>(r: R) -> Result<GraphSpec, failure::Error> {
    let json: JsonValue = serde_json::from_reader(r)?;
    GraphSpec::load(&json)
}
pub fn write<W: io::Write>(w: W, gs: GraphSpec) -> Result<(), failure::Error> {
    let json: JsonValue = gs.save()?;
    serde_json::to_writer(w, &json).map_err(|reason| reason.into())
}
