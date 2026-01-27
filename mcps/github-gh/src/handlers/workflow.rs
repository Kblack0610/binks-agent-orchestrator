//! Workflow and run handler implementations

use rmcp::model::{CallToolResult, Content};
use rmcp::ErrorData as McpError;

use crate::gh::{execute_gh_action, execute_gh_json, execute_gh_raw};
use crate::params::{
    RunCancelParams, RunListParams, RunLogParams, RunRerunParams, RunViewParams,
    WorkflowListParams, WorkflowRunParams,
};
use crate::types::{Workflow, WorkflowRun};

use super::gh_to_mcp_error;

/// List workflows in a repository
pub async fn workflow_list(params: WorkflowListParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["workflow", "list", "-R", &params.repo];

    if params.all == Some(true) {
        args.push("-a");
    }

    let workflows: Vec<Workflow> = execute_gh_json(&args, Workflow::list_fields())
        .await
        .map_err(gh_to_mcp_error)?;

    let json = serde_json::to_string(&workflows)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

/// Trigger a workflow run
pub async fn workflow_run(params: WorkflowRunParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["workflow", "run", &params.workflow, "-R", &params.repo];

    let ref_str;
    let inputs_str;

    if let Some(ref ref_name) = params.ref_name {
        ref_str = ref_name.clone();
        args.extend(["--ref", &ref_str]);
    }
    if let Some(ref inputs) = params.inputs {
        inputs_str = inputs.clone();
        args.extend(["-f", &inputs_str]);
    }

    let output = execute_gh_action(&args).await.map_err(gh_to_mcp_error)?;
    let msg = if output.is_empty() {
        format!("Workflow '{}' triggered successfully", params.workflow)
    } else {
        output
    };
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// List workflow runs
pub async fn run_list(params: RunListParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["run", "list", "-R", &params.repo];

    let workflow_str;
    let branch_str;
    let status_str;
    let limit_str;

    if let Some(ref workflow) = params.workflow {
        workflow_str = workflow.clone();
        args.extend(["-w", &workflow_str]);
    }
    if let Some(ref branch) = params.branch {
        branch_str = branch.clone();
        args.extend(["-b", &branch_str]);
    }
    if let Some(ref status) = params.status {
        status_str = status.clone();
        args.extend(["-s", &status_str]);
    }
    if let Some(limit) = params.limit {
        limit_str = limit.to_string();
        args.extend(["-L", &limit_str]);
    }

    let runs: Vec<WorkflowRun> = execute_gh_json(&args, WorkflowRun::list_fields())
        .await
        .map_err(gh_to_mcp_error)?;

    let json =
        serde_json::to_string(&runs).map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

/// View workflow run details
pub async fn run_view(params: RunViewParams) -> Result<CallToolResult, McpError> {
    let run_id_str = params.run_id.to_string();
    let args = vec!["run", "view", &run_id_str, "-R", &params.repo];

    let run: WorkflowRun = execute_gh_json(&args, WorkflowRun::view_fields())
        .await
        .map_err(gh_to_mcp_error)?;

    let json =
        serde_json::to_string(&run).map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

/// Cancel a workflow run
pub async fn run_cancel(params: RunCancelParams) -> Result<CallToolResult, McpError> {
    let run_id_str = params.run_id.to_string();
    let args = vec!["run", "cancel", &run_id_str, "-R", &params.repo];

    let output = execute_gh_action(&args).await.map_err(gh_to_mcp_error)?;
    let msg = if output.is_empty() {
        format!("Run {} cancelled successfully", params.run_id)
    } else {
        output
    };
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// View logs for a workflow run or specific job
pub async fn run_log(params: RunLogParams) -> Result<CallToolResult, McpError> {
    let run_id_str = params.run_id.to_string();
    let mut args = vec!["run", "view", &run_id_str, "-R", &params.repo];

    let job_str;
    let attempt_str;

    // Add --log or --log-failed flag
    if params.failed_only == Some(true) {
        args.push("--log-failed");
    } else {
        args.push("--log");
    }

    // Add job filter if specified
    if let Some(ref job) = params.job {
        job_str = job.clone();
        args.extend(["--job", &job_str]);
    }

    // Add attempt number if specified
    if let Some(attempt) = params.attempt {
        attempt_str = attempt.to_string();
        args.extend(["--attempt", &attempt_str]);
    }

    let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;

    if output.trim().is_empty() {
        let msg = if params.failed_only == Some(true) {
            format!("No failed step logs found for run {}", params.run_id)
        } else {
            format!("No logs found for run {}", params.run_id)
        };
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    } else {
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }
}

/// Rerun a workflow run - optionally only failed jobs
pub async fn run_rerun(params: RunRerunParams) -> Result<CallToolResult, McpError> {
    let run_id_str = params.run_id.to_string();
    let mut args = vec!["run", "rerun", &run_id_str, "-R", &params.repo];

    if params.failed == Some(true) {
        args.push("--failed");
    }
    if params.debug == Some(true) {
        args.push("--debug");
    }

    let output = execute_gh_action(&args).await.map_err(gh_to_mcp_error)?;
    let msg = if output.is_empty() {
        if params.failed == Some(true) {
            format!("Rerun triggered for failed jobs of run {}", params.run_id)
        } else {
            format!("Rerun triggered for all jobs of run {}", params.run_id)
        }
    } else {
        output
    };
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}
