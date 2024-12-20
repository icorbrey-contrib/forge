use std::collections::HashMap;

mod fs;
mod model;
use fs::{FSFileInfo, FSRead, FSSearch, FSls};
mod think;
use serde_json::Value;
use think::Think;

// TODO: use a more type-safe API instead of the MCP interface
#[async_trait::async_trait]
pub(crate) trait ToolTrait {
    type Input;
    type Output;
    fn id(&self) -> ToolId;
    fn description(&self) -> String;
    async fn call(&self, input: Self::Input) -> Result<Self::Output, String>;
}

struct SerdeTool<T>(T);

impl<T> SerdeTool<T> {
    fn import(tool: T) -> Box<dyn ToolTrait<Input = Value, Output = Value> + Sync + 'static>
    where
        T: ToolTrait + Sync + 'static,
        T::Input: serde::de::DeserializeOwned,
        T::Output: serde::Serialize,
    {
        Box::new(Self(tool))
    }
}

#[async_trait::async_trait]
impl<T: ToolTrait + Sync> ToolTrait for SerdeTool<T>
where
    T::Input: serde::de::DeserializeOwned,
    T::Output: serde::Serialize,
{
    type Input = Value;
    type Output = Value;

    fn id(&self) -> ToolId {
        self.0.id()
    }

    fn description(&self) -> String {
        self.0.description()
    }

    async fn call(&self, input: Self::Input) -> Result<Self::Output, String> {
        let input: T::Input = serde_json::from_value(input).map_err(|e| e.to_string())?;
        let output: T::Output = self.0.call(input).await?;
        Ok(serde_json::to_value(output).map_err(|e| e.to_string())?)
    }
}

pub struct ToolEngine {
    tools: HashMap<ToolId, Box<dyn ToolTrait<Input = Value, Output = Value> + Sync>>,
}

#[derive(Debug, Clone)]
pub struct JsonSchema(Value);

impl JsonSchema {
    pub(crate) fn from_value(value: Value) -> Self {
        JsonSchema(value)
    }

    pub fn into_value(self) -> Value {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct Tool {
    pub id: ToolId,
    pub description: String,
    pub input_schema: JsonSchema,
    pub output_schema: Option<JsonSchema>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ToolId(String);

impl ToolId {
    pub fn into_string(self) -> String {
        self.0
    }
}

impl ToolEngine {
    pub async fn call(&self, tool_id: ToolId, input: Value) -> Result<Value, String> {
        todo!()
    }

    pub fn list(&self) -> Vec<Tool> {
        todo!()
    }
}

impl Default for ToolEngine {
    fn default() -> Self {
        let mut tools = HashMap::new();

        tools.insert(FSRead.id(), SerdeTool::import(FSRead));
        tools.insert(FSSearch.id(), SerdeTool::import(FSSearch));
        tools.insert(FSls.id(), SerdeTool::import(FSls));
        tools.insert(FSFileInfo.id(), SerdeTool::import(FSFileInfo));

        let think = Think::default();
        tools.insert(think.id(), SerdeTool::import(think));

        Self { tools }
    }
}
