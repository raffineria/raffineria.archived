pub use serde_yaml::Value as YamlValue;
use std::io;

use crate::spec::GraphSpec;

use super::FormatSupport;

impl FormatSupport<YamlValue> for GraphSpec {
    type SaveError = failure::Error;
    type LoadError = failure::Error;

    fn save(&self) -> Result<YamlValue, Self::SaveError> {
        serde_yaml::to_value(self.clone()).map_err(|reason| reason.into())
    }
    fn load(data: &YamlValue) -> Result<Self, Self::LoadError> {
        serde_yaml::from_value(data.clone()).map_err(|reason| reason.into())
    }
}

pub fn read<R: io::Read>(r: R) -> Result<GraphSpec, failure::Error> {
    let yaml: YamlValue = serde_yaml::from_reader(r)?;
    GraphSpec::load(&yaml)
}
pub fn write<W: io::Write>(w: W, gs: GraphSpec) -> Result<(), failure::Error> {
    let yaml: YamlValue = gs.save()?;
    serde_yaml::to_writer(w, &yaml).map_err(|reason| reason.into())
}
