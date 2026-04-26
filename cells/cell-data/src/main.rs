//! Dodeca data cell (cell-data)
//!
//! This cell handles loading and parsing data files (JSON, TOML, YAML).

use cell_data_proto::{DataFormat, DataLoader, DataLoaderDispatcher, LoadDataResult, RpcValue};
use facet_value::Value;
use dodeca_cell_runtime::run_cell;
use facet_format::{DeserializeError, FormatDeserializer, FormatParser};

/// Data loader implementation
#[derive(Clone)]
pub struct DataLoaderImpl;

impl DataLoader for DataLoaderImpl {
    async fn load_data(&self, content: String, format: DataFormat) -> LoadDataResult {
        match parse_data(&content, format) {
            Ok(value) => match RpcValue::encode(&value) {
                Ok(value) => LoadDataResult::Success { value },
                Err(message) => LoadDataResult::Error { message },
            },
            Err(e) => LoadDataResult::Error { message: e },
        }
    }
}

/// Parse data using dyn dispatch for reduced monomorphization
fn parse_data(content: &str, format: DataFormat) -> Result<Value, String> {
    match format {
        DataFormat::Json => {
            let mut parser = facet_json::JsonParser::<true>::new(content.as_bytes());
            deserialize_value(&mut parser).map_err(|e| format!("JSON parse error: {e}"))
        }
        DataFormat::Toml => {
            let mut parser = facet_toml::TomlParser::new(content)
                .map_err(|e| format!("TOML parse error: {e}"))?;
            deserialize_value(&mut parser).map_err(|e| format!("TOML parse error: {e}"))
        }
        DataFormat::Yaml => {
            let mut parser = facet_yaml::YamlParser::new(content);
            deserialize_value(&mut parser).map_err(|e| format!("YAML parse error: {e}"))
        }
    }
}

/// Deserialize a Value using dynamic dispatch.
///
/// This function only has one monomorphization regardless of parser type.
fn deserialize_value(parser: &mut dyn FormatParser<'_>) -> Result<Value, DeserializeError> {
    let mut de = FormatDeserializer::new(parser);
    de.deserialize()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_cell!("data", |_handle| DataLoaderDispatcher::new(DataLoaderImpl))
}

#[cfg(test)]
mod tests {
    use super::*;
    use facet_value::DestructuredRef;

    #[test]
    fn parses_array_of_tables_toml_and_rpc_round_trips() {
        let content = r#"
[[integration]]
name = "claude-code"
remote = """claude mcp add --scope user --transport http context7 https://mcp.context7.com/mcp --header "CONTEXT7_API_KEY: YOUR_API_KEY\""""
local = "claude mcp add --scope user context7 -- npx -y @upstash/context7-mcp --api-key YOUR_API_KEY"
"#;

        let value = parse_data(content, DataFormat::Toml).expect("TOML should parse");
        let value = RpcValue::encode(&value)
            .and_then(|value| value.decode())
            .expect("value should round-trip through RpcValue");
        let obj = match value.destructure_ref() {
            DestructuredRef::Object(obj) => obj,
            other => panic!("expected object, got {other:?}"),
        };
        let integrations = obj
            .get("integration")
            .expect("integration key should exist");
        let arr = match integrations.destructure_ref() {
            DestructuredRef::Array(arr) => arr,
            other => panic!("expected array, got {other:?}"),
        };
        assert_eq!(arr.len(), 1);
    }
}
