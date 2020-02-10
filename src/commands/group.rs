use crate::commands::CompileContext;
use std::collections::HashMap;
use crate::{
    commands::command_util::find_field_from_str,
    errors::{argument_error},
    data::{
        Argument,
        Row,
        Stream,
        ValueType,
        Value,
    },
    stream::{OutputStream, unlimited_streams},
};
use crate::data::{ColumnType, RowsReader};
use crate::errors::{CrushResult, error};
use crate::commands::command_util::find_field;
use crate::stream::{Readable};

pub struct Config {
    input_type: Vec<ColumnType>,
    name: Box<str>,
    column: usize,
}

pub fn parse(input_type: Vec<ColumnType>, arguments: Vec<Argument>) -> CrushResult<Config> {
    if arguments.len() != 1 {
        return Err(argument_error("No comparison key specified"));
    }
    let arg = &arguments[0];
    let name = arg.name.clone().unwrap_or(Box::from("group"));
    match &arg.value {
        Value::Text(cell_name) =>
            Ok(Config {
                column: find_field_from_str(cell_name, &input_type)?,
                input_type,
                name,
            }),
        Value::Field(cell_name) =>
            Ok(Config {
                column: find_field(cell_name, &input_type)?,
                input_type,
                name,
            }),
        _ =>
            Err(argument_error("Bad comparison key")),
    }
}

pub fn run(
    config: Config,
    mut input: impl Readable,
    output: OutputStream,
) -> CrushResult<()> {
    let mut groups: HashMap<Value, OutputStream> = HashMap::new();

    loop {
        match input.read() {
            Ok(row) => {
                let key = row.cells()[config.column].clone();
                let val = groups.get(&key);
                match val {
                    None => {
                        let (output_stream, input_stream) = unlimited_streams(config.input_type.clone());
                        let out_row = Row::new(vec![key.clone(), Value::Stream(Stream { stream: input_stream })]);
                        output.send(out_row)?;
                        output_stream.send(row);
                        groups.insert(key, output_stream);
                    }
                    Some(output_stream) => {
                        output_stream.send(row);
                    }
                }
            }
            Err(_) => break,
        }
    }
    return Ok(());
}

pub fn perform(context: CompileContext) -> CrushResult<()> {
    match context.input.recv()? {
        Value::Stream(s) => {
            let input = s.stream;
            let config = parse(input.get_type().clone(), context.arguments)?;
            let output_type= vec![
                input.get_type()[config.column].clone(),
                ColumnType {
                    name: Some(config.name.clone()),
                    cell_type: ValueType::Stream(input.get_type().clone())
                }
            ];
            let output = context.output.initialize(output_type)?;
            run(config, input, output)
        }
        Value::Rows(r) => {
            let input = RowsReader::new(r);
            let config = parse(input.get_type().clone(), context.arguments)?;
            let output_type= vec![
                input.get_type()[config.column].clone(),
                ColumnType {
                    name: Some(config.name.clone()),
                    cell_type: ValueType::Stream(input.get_type().clone())
                }
            ];
            let output = context.output.initialize(output_type)?;
            run(config, input, output)
        }
        _ => Err(error("Expected a stream")),
    }
}
