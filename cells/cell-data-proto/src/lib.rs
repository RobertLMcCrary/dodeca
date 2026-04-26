//! RPC protocol for dodeca data cell
//!
//! Defines services for loading and parsing data files (JSON, TOML, YAML).

use facet::Facet;
use facet_value::Value;

#[derive(Debug, Clone, Facet)]
pub struct RpcValue {
    pub bytes: Vec<u8>,
}

impl RpcValue {
    pub fn encode(value: &Value) -> Result<Self, String> {
        facet_postcard::to_vec(value)
            .map(|bytes| Self { bytes })
            .map_err(|e| e.to_string())
    }

    pub fn decode(&self) -> Result<Value, String> {
        facet_postcard::from_slice(&self.bytes).map_err(|e| e.to_string())
    }
}

/// Supported data file formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Facet)]
#[repr(u8)]
pub enum DataFormat {
    Json,
    Toml,
    Yaml,
}

impl DataFormat {
    /// Determine format from file extension
    pub fn from_extension(path: &str) -> Option<Self> {
        let ext = path.rsplit('.').next()?.to_lowercase();
        match ext.as_str() {
            "json" => Some(Self::Json),
            "toml" => Some(Self::Toml),
            "yaml" | "yml" => Some(Self::Yaml),
            _ => None,
        }
    }
}

/// Result of data loading operations
#[derive(Debug, Clone, Facet)]
#[repr(u8)]
pub enum LoadDataResult {
    /// Successfully loaded and parsed data
    Success { value: RpcValue },
    /// Error during loading or parsing
    Error { message: String },
}

/// Data loading service implemented by the cell.
///
/// The host calls these methods to load and parse data files.
#[allow(async_fn_in_trait)]
#[vox::service]
pub trait DataLoader {
    /// Load and parse a data file
    ///
    /// Returns the parsed value, or an error if parsing fails.
    async fn load_data(&self, content: String, format: DataFormat) -> LoadDataResult;
}
