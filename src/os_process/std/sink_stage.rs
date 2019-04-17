use futures::prelude::*;
use futures::stream;
use serde::de::DeserializeOwned;

use crate::futures::{SendBoxedSink, SendBoxedStream};
use crate::os_process::Stage;
use crate::protocol::{DataItem, Schema};

pub struct SinkStage<S, I, E>
where
    S: Sink<SinkItem = I, SinkError = E>,
    I: DeserializeOwned,
    E: Into<failure::Error>,
{
    sink: S,
    inlet_schemas: Vec<Schema>,
}

impl<S, I, E> SinkStage<S, I, E>
where
    S: Sink<SinkItem = I, SinkError = E>,
    I: DeserializeOwned,
    E: Into<failure::Error>,
{
    pub fn from_sink(sink: S, schema: &Schema) -> Self {
        let inlet_schemas = vec![schema.clone()];
        Self {
            sink,
            inlet_schemas,
        }
    }
}

impl<S, I, E> Stage for SinkStage<S, I, E>
where
    S: Sink<SinkItem = I, SinkError = E> + Send + 'static,
    I: DeserializeOwned + Send + 'static,
    E: Into<failure::Error>,
{
    fn inlets(&self) -> &Vec<Schema> {
        &self.inlet_schemas
    }

    fn into_streams(
        self,
    ) -> (
        Vec<SendBoxedStream<DataItem, failure::Error>>,
        Vec<SendBoxedSink<DataItem, failure::Error>>,
    ) {
        let e_mapped = self
            .sink
            .sink_map_err(|reason| Into::<failure::Error>::into(reason));
        let v_mapped = e_mapped.with_flat_map(|data_item| {
            stream::iter_result(vec![avro_rs::from_value::<I>(&data_item)])
                .map_err(|reason| Into::<failure::Error>::into(reason))
        });
        let boxed = Box::new(v_mapped);
        (vec![], vec![boxed])
    }
}
