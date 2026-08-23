use cel_interpreter::{Context, Program};
use serde_json::Value;
use tracing::debug;

use crate::types::RuleContext;

#[derive(Debug, thiserror::Error)]
pub enum CelError {
    #[error("CEL parse error: {0}")]
    ParseError(String),
    #[error("CEL evaluation error: {0}")]
    EvaluationError(String),
    #[error("CEL context error: {0}")]
    ContextError(String),
}

pub struct CelEvaluator {
    programs: std::collections::HashMap<String, Program>,
}

impl CelEvaluator {
    pub fn new() -> Self {
        Self {
            programs: std::collections::HashMap::new(),
        }
    }

    pub fn compile(&mut self, rule_id: &str, expression: &str) -> Result<(), CelError> {
        let program = Program::compile(expression)
            .map_err(|e| CelError::ParseError(format!("Failed to compile '{}': {}", expression, e)))?;
        
        self.programs.insert(rule_id.to_string(), program);
        debug!("Compiled CEL expression for rule: {}", rule_id);
        Ok(())
    }

    pub fn evaluate(&self, rule_id: &str, context: &RuleContext) -> Result<bool, CelError> {
        let program = self.programs.get(rule_id)
            .ok_or_else(|| CelError::ContextError(format!("Rule {} not compiled", rule_id)))?;

        let mut cel_context = Context::default();

        let logs_value: Vec<Value> = context.logs.iter().cloned().collect();
        let _ = cel_context.add_variable("logs", logs_value);

        let _ = cel_context.add_variable("window_start", context.window_start.clone());
        let _ = cel_context.add_variable("window_end", context.window_end.clone());

        if let Some(ref group_key) = context.group_key {
            let _ = cel_context.add_variable("group_key", group_key.clone());
        }

        let result = program.execute(&cel_context)
            .map_err(|e| CelError::EvaluationError(format!("Evaluation failed: {}", e)))?;

        match result {
            cel_interpreter::Value::Bool(b) => Ok(b),
            cel_interpreter::Value::Int(i) => Ok(i != 0),
            cel_interpreter::Value::UInt(u) => Ok(u != 0),
            _ => Ok(false),
        }
    }
}
