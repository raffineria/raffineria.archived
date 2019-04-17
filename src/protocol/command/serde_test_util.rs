use avro_rs::types::Value as AvroValue;
use avro_rs::Schema;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;

#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "ValueToAvro")]
    ValueToAvro(#[cause] failure::Error),

    #[fail(display = "AvroToBytes({:?})", _1)]
    AvroToBytes(#[cause] failure::Error, AvroValue),

    #[fail(display = "BytesToAvro")]
    BytesToAvro(#[cause] failure::Error),

    #[fail(display = "AvroToValue")]
    AvroToValue(#[cause] failure::Error),

    #[fail(display = "Value mismatch [exp: {:?}; act: {:?}]", expected, actual)]
    ValuesMismatch { expected: String, actual: String },
}

pub fn run_serde<V: Clone + PartialEq + Debug + Serialize + DeserializeOwned>(
    value_in: V,
    schema: &Schema,
) -> Result<(), Error> {
    use bytes::IntoBuf;

    eprintln!("value_in: {:?}", value_in);
    let avro_value =
        avro_rs::to_value(value_in.clone()).map_err(|reason| Error::ValueToAvro(reason.into()))?;
    eprintln!("avro_value: {:?}", avro_value);

    let bytes = avro_rs::to_avro_datum(schema, avro_value.clone())
        .map_err(|reason| Error::AvroToBytes(reason, avro_value))?;

    let avro_value = avro_rs::from_avro_datum(schema, &mut bytes.into_buf(), Some(schema))
        .map_err(|reason| Error::BytesToAvro(reason))?;

    let value_out = avro_rs::from_value::<V>(&avro_value)
        .map_err(|reason| Error::AvroToValue(reason.into()))?;

    if value_out != value_in {
        Err(Error::ValuesMismatch {
            expected: format!("{:?}", value_in),
            actual: format!("{:?}", value_out),
        })
    } else {
        Ok(())
    }
}
