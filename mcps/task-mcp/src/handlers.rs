//! Handler implementations for task-mcp tools
//!
//! Each handler converts MCP params to repository types, calls the repository,
//! and converts results to CallToolResult with proper error handling.

use mcp_common::{internal_error, invalid_params, json_success, CallToolResult, McpError};
use serde_json::json;
use std::str::FromStr;

use crate::params::*;
use crate::repository::{NewTask, TaskFilter, TaskRepository};
use crate::types::{BlockingCheckResponse, DependencyListResponse, TaskListResponse, TaskStatus};

// ============================================================================
// CRUD Operations
// ============================================================================

pub async fn create_task(
    repo: &TaskRepository,
    params: CreateTaskParams,
) -> Result<CallToolResult, McpError> {
    let new_task = NewTask {
        title: params.title,
        description: params.description,
        priority: params.priority,
        plan_source: params.plan_source,
        plan_section: params.plan_section,
        assigned_to: params.assigned_to,
        parent_task_id: params.parent_task_id,
        metadata: params.metadata,
    };

    let task = repo
        .create_task(new_task)
        .map_err(|e| internal_error(format!("Failed to create task: {}", e)))?;

    json_success(&task)
}

pub async fn get_task(
    repo: &TaskRepository,
    params: GetTaskParams,
) -> Result<CallToolResult, McpError> {
    if params.id.len() < 8 {
        return Err(invalid_params(
            "Task ID or prefix must be at least 8 characters",
        ));
    }

    let task = repo
        .get_task(&params.id)
        .map_err(|e| internal_error(format!("Failed to get task: {}", e)))?;

    match task {
        Some(task) => json_success(&task),
        None => Err(invalid_params(format!(
            "Task not found with ID or prefix: {}",
            params.id
        ))),
    }
}

pub async fn list_tasks(
    repo: &TaskRepository,
    params: ListTasksParams,
) -> Result<CallToolResult, McpError> {
    let filter = TaskFilter {
        status: params.status,
        plan_source: params.plan_source,
        assigned_to: params.assigned_to,
        min_priority: params.min_priority,
        limit: params.limit,
    };

    let tasks = repo
        .list_tasks(filter)
        .map_err(|e| internal_error(format!("Failed to list tasks: {}", e)))?;

    let response = TaskListResponse {
        total: tasks.len(),
        tasks,
    };

    json_success(&response)
}

pub async fn update_task(
    repo: &TaskRepository,
    params: UpdateTaskParams,
) -> Result<CallToolResult, McpError> {
    // Convert status string to enum if provided
    let status = if let Some(status_str) = &params.status {
        Some(
            TaskStatus::from_str(status_str)
                .map_err(|e| invalid_params(format!("Invalid status: {}", e)))?,
        )
    } else {
        None
    };

    repo.update_task_fields(
        &params.id,
        status,
        params.branch_name.as_deref(),
        params.pr_url.as_deref(),
        params.assigned_to.as_deref(),
        params.priority,
        params.metadata.as_deref(),
    )
    .map_err(|e| internal_error(format!("Failed to update task: {}", e)))?;

    // Fetch updated task to return
    let task = repo
        .get_task(&params.id)
        .map_err(|e| internal_error(format!("Failed to get updated task: {}", e)))?
        .ok_or_else(|| invalid_params(format!("Task not found: {}", params.id)))?;

    json_success(&task)
}

// ============================================================================
// Dependency Management
// ============================================================================

pub async fn add_dependency(
    repo: &TaskRepository,
    params: AddDependencyParams,
) -> Result<CallToolResult, McpError> {
    repo.add_dependency(&params.task_id, &params.depends_on_task_id)
        .map_err(|e| internal_error(format!("Failed to add dependency: {}", e)))?;

    json_success(&json!({
        "success": true,
        "task_id": params.task_id,
        "depends_on_task_id": params.depends_on_task_id,
        "message": format!("Task {} now depends on {}", params.task_id, params.depends_on_task_id)
    }))
}

pub async fn list_dependencies(
    repo: &TaskRepository,
    params: ListDependenciesParams,
) -> Result<CallToolResult, McpError> {
    let dependencies = repo
        .get_dependencies(&params.task_id)
        .map_err(|e| internal_error(format!("Failed to get dependencies: {}", e)))?;

    let blocking_tasks = repo
        .check_blocking_tasks(&params.task_id)
        .map_err(|e| internal_error(format!("Failed to get blocking tasks: {}", e)))?;

    let blocked_tasks = repo
        .get_blocked_tasks(&params.task_id)
        .map_err(|e| internal_error(format!("Failed to get blocked tasks: {}", e)))?;

    let response = DependencyListResponse {
        dependencies,
        blocking_tasks,
        blocked_tasks,
    };

    json_success(&response)
}

pub async fn check_blocking_tasks(
    repo: &TaskRepository,
    params: CheckBlockingTasksParams,
) -> Result<CallToolResult, McpError> {
    let blocking_tasks = repo
        .check_blocking_tasks(&params.task_id)
        .map_err(|e| internal_error(format!("Failed to check blocking tasks: {}", e)))?;

    let blocking_task_ids: Vec<String> = blocking_tasks.iter().map(|t| t.id.clone()).collect();

    let response = BlockingCheckResponse {
        is_blocked: !blocking_tasks.is_empty(),
        blocking_task_ids,
        blocking_tasks,
    };

    json_success(&response)
}

// ============================================================================
// Execution Tracking
// ============================================================================

pub async fn record_execution(
    repo: &TaskRepository,
    params: RecordExecutionParams,
) -> Result<CallToolResult, McpError> {
    let execution_id = repo
        .record_execution(
            &params.task_id,
            "task-mcp", // agent_name - could be made configurable
            &params.status,
            params.run_id.as_deref(),
            params.error_message.as_deref(),
        )
        .map_err(|e| internal_error(format!("Failed to record execution: {}", e)))?;

    json_success(&json!({
        "success": true,
        "execution_id": execution_id,
        "task_id": params.task_id,
        "status": params.status,
        "run_id": params.run_id,
    }))
}

pub async fn link_to_run(
    repo: &TaskRepository,
    params: LinkToRunParams,
) -> Result<CallToolResult, McpError> {
    repo.link_to_run(&params.task_id, &params.run_id)
        .map_err(|e| internal_error(format!("Failed to link task to run: {}", e)))?;

    json_success(&json!({
        "success": true,
        "task_id": params.task_id,
        "run_id": params.run_id,
        "message": format!("Task {} linked to run {}", params.task_id, params.run_id)
    }))
}

// ============================================================================
// Query Operations
// ============================================================================

pub async fn tasks_by_status(
    repo: &TaskRepository,
    params: TasksByStatusParams,
) -> Result<CallToolResult, McpError> {
    let filter = TaskFilter {
        status: Some(params.status),
        plan_source: None,
        assigned_to: None,
        min_priority: None,
        limit: params.limit,
    };

    let tasks = repo
        .list_tasks(filter)
        .map_err(|e| internal_error(format!("Failed to list tasks by status: {}", e)))?;

    let response = TaskListResponse {
        total: tasks.len(),
        tasks,
    };

    json_success(&response)
}

pub async fn tasks_by_plan(
    repo: &TaskRepository,
    params: TasksByPlanParams,
) -> Result<CallToolResult, McpError> {
    let filter = TaskFilter {
        status: None,
        plan_source: Some(params.plan_source),
        assigned_to: None,
        min_priority: None,
        limit: params.limit,
    };

    let tasks = repo
        .list_tasks(filter)
        .map_err(|e| internal_error(format!("Failed to list tasks by plan: {}", e)))?;

    let response = TaskListResponse {
        total: tasks.len(),
        tasks,
    };

    json_success(&response)
}

pub async fn grab_next_task(
    repo: &TaskRepository,
    params: GrabNextTaskParams,
) -> Result<CallToolResult, McpError> {
    let task = repo
        .grab_next_task(&params.agent_name, params.status_filter.as_deref())
        .map_err(|e| internal_error(format!("Failed to grab next task: {}", e)))?;

    match task {
        Some(task) => json_success(&task),
        None => json_success(&json!({
            "message": "No available tasks",
            "agent_name": params.agent_name,
            "status_filter": params.status_filter,
        })),
    }
}

// ============================================================================
// Memory Integration
// ============================================================================

pub async fn sync_to_memory(
    repo: &TaskRepository,
    params: SyncToMemoryParams,
) -> Result<CallToolResult, McpError> {
    // Fetch the task
    let task = repo
        .get_task(&params.task_id)
        .map_err(|e| internal_error(format!("Failed to get task: {}", e)))?
        .ok_or_else(|| invalid_params(format!("Task not found: {}", params.task_id)))?;

    // Build memory entity representation
    let mut facts = vec![
        json!({
            "key": "title",
            "value": task.title,
            "confidence": 1.0,
            "source": "task-mcp"
        }),
        json!({
            "key": "description",
            "value": task.description,
            "confidence": 1.0,
            "source": "task-mcp"
        }),
        json!({
            "key": "status",
            "value": task.status.as_str(),
            "confidence": 1.0,
            "source": "task-mcp"
        }),
        json!({
            "key": "priority",
            "value": task.priority.to_string(),
            "confidence": 1.0,
            "source": "task-mcp"
        }),
        json!({
            "key": "created_at",
            "value": task.created_at,
            "confidence": 1.0,
            "source": "task-mcp"
        }),
    ];

    // Add optional fields
    if let Some(plan_source) = &task.plan_source {
        facts.push(json!({
            "key": "plan_source",
            "value": plan_source,
            "confidence": 1.0,
            "source": "task-mcp"
        }));
    }

    if let Some(assigned_to) = &task.assigned_to {
        facts.push(json!({
            "key": "assigned_to",
            "value": assigned_to,
            "confidence": 1.0,
            "source": "task-mcp"
        }));
    }

    if let Some(branch_name) = &task.branch_name {
        facts.push(json!({
            "key": "branch_name",
            "value": branch_name,
            "confidence": 1.0,
            "source": "task-mcp"
        }));
    }

    if let Some(pr_url) = &task.pr_url {
        facts.push(json!({
            "key": "pr_url",
            "value": pr_url,
            "confidence": 1.0,
            "source": "task-mcp"
        }));
    }

    // Build relations
    let mut relations = vec![];

    if let Some(parent_id) = &task.parent_task_id {
        relations.push(json!({
            "to_entity": format!("task:{}", parent_id),
            "relation_type": "subtask_of"
        }));
    }

    // Optionally include dependencies
    if params.include_dependencies.unwrap_or(false) {
        let dependencies = repo
            .get_dependencies(&params.task_id)
            .map_err(|e| internal_error(format!("Failed to get dependencies: {}", e)))?;

        for dep in dependencies {
            relations.push(json!({
                "to_entity": format!("task:{}", dep.depends_on_task_id),
                "relation_type": "depends_on"
            }));
        }
    }

    // Return the memory representation (caller would use memory-mcp to store this)
    let memory_entity = json!({
        "entity": format!("task:{}", task.id),
        "entity_type": "task",
        "facts": facts,
        "relations": relations,
        "instructions": format!(
            "This entity represents task '{}'. To store in memory-mcp, call: mcp__memory__learn with this entity data.",
            task.title
        )
    });

    json_success(&memory_entity)
}
