use bytes::BytesMut;
use tokio::codec;

use crate::protocol::streams::Error;
use crate::protocol::{Command, Schema};

const SIZE_LEN: usize = 4;

pub struct Decoder {
    schema: Schema,
}

impl Decoder {
    pub fn new(schema: Schema) -> Self {
        Self { schema }
    }
}

impl codec::Decoder for Decoder {
    type Item = Command;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < SIZE_LEN {
            Ok(None)
        } else {
            let (sz_bytes, rest) = src.split_at(4);
            let sz = dec_sz(sz_bytes);
            // trace!("sz: {}: sz-bytes: {:?}; rest: {:?}", sz, sz_bytes, rest);
            if rest.len() < sz {
                Ok(None)
            } else {
                use bytes::IntoBuf;

                let mut data_bytes = src.split_to(SIZE_LEN + sz);
                data_bytes.advance(SIZE_LEN);

                let avro_value =
                    avro_rs::from_avro_datum(&self.schema, &mut data_bytes.into_buf(), None)?;
                assert!(avro_value.validate(&self.schema));
                let item = avro_rs::from_value::<Command>(&avro_value)?;

                Ok(Some(item))
            }
        }
    }
}

fn dec_sz(bytes: &[u8]) -> usize {
    assert!(bytes.len() == 4);
    (bytes[0] as usize) * 256 * 256 * 256
        + (bytes[1] as usize) * 256 * 256
        + (bytes[2] as usize) * 256
        + (bytes[3] as usize)
}
