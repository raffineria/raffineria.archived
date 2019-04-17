use avro_rs::schema::{Documentation, Name, RecordField, RecordFieldOrder, Schema};
use std::collections::HashMap;

fn doc_none() -> Documentation {
    Documentation::default()
}

fn record_field(name: &str, idx: usize, schema: Schema) -> RecordField {
    RecordField {
        name: name.to_owned(),
        doc: doc_none(),
        default: None,
        schema,
        order: RecordFieldOrder::Ignore,
        position: idx,
    }
}

fn mk_name(name: &str) -> Name {
    Name::new(name)
}

pub fn record_schema(name: &str, fields: Vec<(&str, Schema)>) -> Schema {
    let lookup = fields
        .iter()
        .enumerate()
        .map(|(idx, &(name, _))| (name.to_owned(), idx))
        .collect::<HashMap<String, usize>>();

    let fields = fields
        .into_iter()
        .enumerate()
        .map(|(idx, (name, schema))| record_field(name, idx, schema))
        .collect::<Vec<_>>();

    Schema::Record {
        name: mk_name(name),
        doc: doc_none(),
        fields,
        lookup,
    }
}
