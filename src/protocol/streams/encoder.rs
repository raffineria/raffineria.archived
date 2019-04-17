use bytes::BytesMut;
use tokio::codec;

use crate::protocol::streams::Error;
use crate::protocol::{Command, Schema};

pub struct Encoder {
    schema: Schema,
}

impl Encoder {
    pub fn new(schema: Schema) -> Self {
        Self { schema }
    }
}

impl codec::Encoder for Encoder {
    type Item = Command;
    type Error = Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let avro_value = avro_rs::to_value(item)?;
        assert!(avro_value.validate(&self.schema));
        let output = avro_rs::to_avro_datum(&self.schema, avro_value)?;
        trace!("sz: {}; output: {:?}", output.len(), output);

        dst.extend(enc_sz(output.len()).iter());
        dst.extend(output);

        Ok(())
    }
}

fn enc_sz(sz: usize) -> [u8; 4] {
    use std::mem::transmute;
    unsafe { transmute((sz as u32).to_be()) }
}
