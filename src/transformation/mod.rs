//! Transformation module for the DSL engine
//!
//! This module contains the DSL engine for transforming calendar events.

// TODO: Implement the DSL engine for transforming calendar events.
// The DSL should support:
// - Regex operations
// - Control structures (conditionals, branching, function calls)
// - Functional constructs (map, filter, flatmap)
// - Domain types (calendar and event types)

mod dsl;

use anyhow::Result;

use crate::db::models::EventVersion;

/// The result of a transformation operation
#[derive(Debug)]
pub struct TransformationResult {
    pub event: EventVersion,
}

/// The DSL engine for transforming calendar events
pub struct DslEngine {
    env: dsl::Env,
    compiled_expr: Option<dsl::SExpr>,
}

impl DslEngine {
    /// Create a new DSL engine
    pub fn new() -> Self {
        Self {
            env: dsl::Env::new(),
            compiled_expr: None,
        }
    }

    /// Compile a DSL expression
    pub fn compile(&mut self, expression: &str) -> Result<()> {
        self.compiled_expr = Some(dsl::parse(expression)?);
        Ok(())
    }

    /// Execute a compiled DSL expression
    pub fn filter_events(
        &self,
        events: impl Iterator<Item = EventVersion>,
    ) -> Result<Vec<EventVersion>> {
        match &self.compiled_expr {
            Some(filter) => {
                let mut filtered = Vec::new();

                for event in events {
                    // Run filter with the event: (eval filter event)
                    let runtime_expression = dsl::SExpr::List(vec![
                        dsl::SExpr::Symbol("eval".to_string()),
                        filter.clone(),
                        dsl::SExpr::Event(event),
                    ]);
                    let result = dsl::eval(&self.env, &runtime_expression)?;
                    match result {
                        dsl::SExpr::Event(event) => filtered.push(event),
                        dsl::SExpr::List(events) => {
                            for event in events {
                                match event {
                                    dsl::SExpr::Event(event) => filtered.push(event),
                                    _ => {
                                        return Err(anyhow::anyhow!(
                                            "Expected event, got {event:?}",
                                        ))
                                    }
                                }
                            }
                        }
                        _ => return Err(anyhow::anyhow!("Expected event, got {result:?}")),
                    }
                }
                Ok(filtered)
            }
            None => Err(anyhow::anyhow!("DSL expression not compiled")),
        }
    }
}

impl Default for DslEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_compilation() {
        let mut engine = DslEngine::new();
        assert!(engine.compile("(+ 1 2)").is_ok());
    }
}
