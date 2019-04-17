use futures::prelude::*;
use serde::Serialize;

use crate::futures::{SendBoxedSink, SendBoxedStream};
use crate::os_process::Stage;
use crate::protocol::{DataItem, Schema};

pub struct SourceStage<S, I, E>
where
    S: Stream<Item = I, Error = E> + Send + 'static,
    I: Serialize + Send + 'static,
    E: Into<failure::Error>,
{
    source: S,
    outlet_schemas: Vec<Schema>,
}

impl<S, I, E> SourceStage<S, I, E>
where
    S: Stream<Item = I, Error = E> + Send + 'static,
    I: Serialize + Send + 'static,
    E: Into<failure::Error>,
{
    pub fn from_stream(source: S, schema: &Schema) -> Self {
        let outlet_schemas = vec![schema.clone()];
        Self {
            source,
            outlet_schemas,
        }
    }
}

impl<S, I, E> Stage for SourceStage<S, I, E>
where
    S: Stream<Item = I, Error = E> + Send + 'static,
    I: Serialize + Send + 'static,
    E: Into<failure::Error>,
{
    fn outlets(&self) -> &Vec<Schema> {
        &self.outlet_schemas
    }

    fn into_streams(
        self,
    ) -> (
        Vec<SendBoxedStream<DataItem, failure::Error>>,
        Vec<SendBoxedSink<DataItem, failure::Error>>,
    ) {
        let e_mapped = self
            .source
            .map_err(|reason| Into::<failure::Error>::into(reason));
        let v_mapped = e_mapped.and_then(|item| {
            avro_rs::to_value(item).map_err(|reason| Into::<failure::Error>::into(reason))
        });
        let boxed = Box::new(v_mapped);
        (vec![boxed], vec![])
    }
}
