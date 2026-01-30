//! Client wrapper for workflow-mcp MCP server
//!
//! This module provides a type-safe interface for interacting with the workflow-mcp
//! MCP server, abstracting the raw MCP tool calls.

use anyhow::{Context, Result};
use serde_json::json;
use std::cell::RefCell;
use std::collections::HashMap;

#[cfg(feature = "mcp")]
use crate::mcp::McpClientPool;

/// Information about a workflow
#[derive(Debug, Clone)]
pub struct WorkflowInfo {
    pub name: String,
    pub description: String,
    pub step_count: usize,
}

/// Status of a workflow execution
#[derive(Debug, Clone)]
pub struct ExecutionStatus {
    pub execution_id: String,
    pub workflow_name: String,
    pub current_step: usize,
    pub status: String,
    pub completed_steps: usize,
    pub context: HashMap<String, String>,
}

/// Client for interacting with workflow-mcp
#[cfg(feature = "mcp")]
pub struct WorkflowClient<'a> {
    pool: &'a RefCell<McpClientPool>,
}

#[cfg(feature = "mcp")]
impl<'a> WorkflowClient<'a> {
    /// Create a new workflow client
    pub fn new(pool: &'a RefCell<McpClientPool>) -> Self {
        Self { pool }
    }

    /// List all available workflows
    pub async fn list_workflows(&mut self) -> Result<Vec<WorkflowInfo>> {
        let result = {
            let mut pool = self.pool.borrow_mut();
            pool.call_tool("list_workflows", Some(json!({})))
                .await
                .context("Failed to call workflow__list_workflows")?
        };

        // Extract text from first content item
        let text = result
            .content
            .first()
            .and_then(|c| {
                if let rmcp::model::RawContent::Text(text) = &c.raw {
                    Some(text.text.as_str())
                } else {
                    None
                }
            })
            .context("No text content in response")?;

        // Parse the response
        let response: serde_json::Value =
            serde_json::from_str(text).context("Failed to parse list_workflows response")?;

        let workflows = response["workflows"]
            .as_array()
            .context("Expected 'workflows' array in response")?;

        let mut workflow_infos = Vec::new();
        for workflow in workflows {
            workflow_infos.push(WorkflowInfo {
                name: workflow["name"]
                    .as_str()
                    .context("Missing workflow name")?
                    .to_string(),
                description: workflow["description"]
                    .as_str()
                    .context("Missing workflow description")?
                    .to_string(),
                step_count: workflow["step_count"]
                    .as_u64()
                    .context("Missing step_count")? as usize,
            });
        }

        Ok(workflow_infos)
    }

    /// Execute a workflow with the given name and task
    pub async fn execute_workflow(
        &mut self,
        workflow_name: &str,
        task: &str,
        context: HashMap<String, String>,
    ) -> Result<String> {
        let args = json!({
            "workflow": workflow_name,
            "task": task,
            "context": context,
        });

        let result = {
            let mut pool = self.pool.borrow_mut();
            pool.call_tool("execute_workflow", Some(args))
                .await
                .context("Failed to call workflow__execute_workflow")?
        };

        // Extract text from first content item
        let text = result
            .content
            .first()
            .and_then(|c| {
                if let rmcp::model::RawContent::Text(text) = &c.raw {
                    Some(text.text.as_str())
                } else {
                    None
                }
            })
            .context("No text content in response")?;

        // Parse the response to get execution_id
        let response: serde_json::Value =
            serde_json::from_str(text).context("Failed to parse execute_workflow response")?;

        let execution_id = response["execution_id"]
            .as_str()
            .context("Missing execution_id in response")?
            .to_string();

        Ok(execution_id)
    }

    /// Get the status of a workflow execution
    pub async fn get_execution_status(&mut self, execution_id: &str) -> Result<ExecutionStatus> {
        let args = json!({
            "execution_id": execution_id,
        });

        let result = {
            let mut pool = self.pool.borrow_mut();
            pool.call_tool("get_execution_status", Some(args))
                .await
                .context("Failed to call workflow__get_execution_status")?
        };

        // Extract text from first content item
        let text = result
            .content
            .first()
            .and_then(|c| {
                if let rmcp::model::RawContent::Text(text) = &c.raw {
                    Some(text.text.as_str())
                } else {
                    None
                }
            })
            .context("No text content in response")?;

        // Parse the response
        let response: serde_json::Value =
            serde_json::from_str(text).context("Failed to parse get_execution_status response")?;

        // Parse context HashMap
        let context_obj = response["context"]
            .as_object()
            .context("Expected 'context' object")?;

        let mut context = HashMap::new();
        for (key, value) in context_obj {
            if let Some(val_str) = value.as_str() {
                context.insert(key.clone(), val_str.to_string());
            }
        }

        Ok(ExecutionStatus {
            execution_id: response["execution_id"]
                .as_str()
                .context("Missing execution_id")?
                .to_string(),
            workflow_name: response["workflow_name"]
                .as_str()
                .context("Missing workflow_name")?
                .to_string(),
            current_step: response["current_step"]
                .as_u64()
                .context("Missing current_step")? as usize,
            status: response["status"]
                .as_str()
                .context("Missing status")?
                .to_string(),
            completed_steps: response["completed_steps"]
                .as_u64()
                .context("Missing completed_steps")? as usize,
            context,
        })
    }

    /// Resume a workflow from a checkpoint
    pub async fn resume_from_checkpoint(
        &mut self,
        execution_id: &str,
        approved: bool,
        feedback: Option<String>,
    ) -> Result<()> {
        let args = json!({
            "execution_id": execution_id,
            "approved": approved,
            "feedback": feedback,
        });

        {
            let mut pool = self.pool.borrow_mut();
            pool.call_tool("resume_from_checkpoint", Some(args))
                .await
                .context("Failed to call workflow__resume_from_checkpoint")?
        };

        Ok(())
    }
}
