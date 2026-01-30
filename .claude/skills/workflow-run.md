# Workflow Run Skill

This skill executes any multi-agent workflow defined in TOML format via workflow-mcp.

## Workflow

1. **List available workflows** - Show workflows in `.binks/workflows/`
2. **Select workflow** - User chooses which workflow to execute
3. **Gather inputs** - Prompt for required workflow inputs
4. **Execute workflow** - Start workflow execution via workflow-mcp
5. **Monitor progress** - Poll and display workflow status
6. **Handle checkpoints** - Interactive approval at decision points
7. **Display results** - Show workflow output and agent responses

## Tools Used

- `mcp__filesystem__list_dir` - List available workflows
- `mcp__filesystem__read_file` - Read workflow TOML to show details
- `mcp__workflow__list_workflows` - List workflows via workflow-mcp
- `mcp__workflow__execute_workflow` - Start workflow execution
- `mcp__workflow__get_execution_status` - Poll workflow progress
- `mcp__workflow__resume_from_checkpoint` - Handle checkpoints

## Workflow Execution

I'll help you execute a workflow.

### Step 1: List Available Workflows

Let me show you the available workflows:

[Use mcp__filesystem__list_dir for .binks/workflows/]

Available workflows in `.binks/workflows/`:

1. **task-execution.toml** - End-to-end task execution workflow
   - Steps: 4 (plan → implement → test → pr)
   - Agents: planner, implementer, tester, reviewer
   - Checkpoints: plan_review, implementation_review

2. **code-review.toml** - Code review workflow
   - Steps: 3 (analyze → review → feedback)
   - Agents: reviewer, security-checker
   - Checkpoints: security_review

3. **{custom}.toml** - Custom workflows
   ...

Which workflow would you like to execute? (provide name or number)

### Step 2: Show Workflow Details

Reading workflow: {selected_workflow}

[Use mcp__filesystem__read_file to read TOML]

**Workflow:** {name}
**Description:** {description}
**Total Steps:** {step_count}

**Steps:**
1. {step_1.role} - {step_1.goal}
2. {step_2.role} - {step_2.goal}
3. ...

**Checkpoints:**
- {checkpoint_1} after step {n}
- {checkpoint_2} after step {m}

**Agents Required:**
- {agent_1}: {description}
- {agent_2}: {description}

### Step 3: Gather Required Inputs

This workflow requires the following inputs:

**Task Description:**
What would you like this workflow to accomplish?

[Prompt user for task description]

**Additional Context** (optional):
Any specific requirements, constraints, or background information?

[Prompt for optional context]

### Step 4: Execute Workflow

Starting workflow execution...

[Use mcp__workflow__execute_workflow with:
- workflow: {workflow_name}
- task: {user_task}
- context: {additional_context}
]

**Workflow Started**
- Execution ID: {execution_id}
- Workflow: {workflow_name}
- Task: {task_description}

### Step 5: Monitor Progress

Monitoring workflow execution...

[Use mcp__workflow__get_execution_status in polling loop every 2-3 seconds]

**Current Status:**
- Step: {current_step}/{total_steps}
- Status: {status}
- Current Agent: {current_agent_role}
- Progress: [{progress_bar}] {percent}%

**Step Details:**
- Role: {current_step.role}
- Goal: {current_step.goal}
- Status: {step_status}

[Display agent activity indicators during execution]

### Step 6: Handle Checkpoints

When workflow reaches checkpoint (status="waiting_for_approval"):

**━━━ Checkpoint: {checkpoint_name} ━━━**

{checkpoint_message}

**Output from {agent_role}:**
```
{agent_output}
```

**Decision Required:**
- ✓ Approve - Continue to next step
- ✗ Reject - Provide feedback and retry this step
- ⊗ Abort - Cancel workflow execution

Your choice:

[Prompt user for decision]

[If approved:]
Continuing workflow...
[Use mcp__workflow__resume_from_checkpoint with approved=true]

[If rejected:]
What feedback should be provided to the agent?
[Prompt for feedback text]

Retrying with feedback...
[Use mcp__workflow__resume_from_checkpoint with approved=false, feedback={text}]

[If aborted:]
Cancelling workflow execution...
[Handle abort - workflow-mcp may need abort_execution tool]

### Step 7: Workflow Completion

When workflow completes (status="completed"):

**━━━ Workflow Complete ━━━**

**Summary:**
- Workflow: {workflow_name}
- Execution ID: {execution_id}
- Duration: {elapsed_time}
- Steps Completed: {completed_steps}/{total_steps}
- Checkpoints: {checkpoint_count} (all approved)

**Final Results:**

[Extract results from workflow context based on workflow type]

For task-execution workflow:
- Plan: {context.plan}
- Implementation: {context.changes}
- Tests: {context.test_results}
- PR: {context.pr_url}

For code-review workflow:
- Analysis: {context.analysis}
- Review Comments: {context.review}
- Security Issues: {context.security_findings}

**Agent Contributions:**

1. {agent_1_role}: {agent_1_output_summary}
2. {agent_2_role}: {agent_2_output_summary}
3. ...

Workflow artifacts and full transcript available via workflow-mcp execution ID: {execution_id}

### Error Handling

**Workflow failed (status="failed"):**

**━━━ Workflow Failed ━━━**

**Error Details:**
- Step: {failed_step}
- Agent: {failed_agent}
- Error: {error_message}

**Options:**
1. Review error and retry workflow
2. Modify workflow definition
3. Check agent configurations
4. Contact support with execution ID: {execution_id}

**Workflow not found:**

Could not find workflow: {requested_name}

Available workflows:
[List workflows again]

Please specify a valid workflow name or path to TOML file.

**Invalid workflow definition:**

Workflow TOML validation failed:
- {validation_error_1}
- {validation_error_2}

Please fix the workflow definition and try again.

**MCP connection error:**

Cannot connect to workflow-mcp server.

Troubleshooting:
1. Check that workflow-mcp is configured in .mcp.json
2. Verify workflow-mcp binary is built: `cargo build -p workflow-mcp`
3. Test workflow-mcp directly: `echo '{"method":"tools/list"}' | ./target/debug/workflow-mcp`
4. Check logs for workflow-mcp startup errors

## Workflow Definition Format

Workflows are defined in TOML files under `.binks/workflows/`:

```toml
[workflow]
name = "example-workflow"
description = "An example multi-agent workflow"

[[steps]]
role = "planner"
goal = "Create a plan for the task"
prompt = "You are a planning agent..."
requires_approval = true
checkpoint_message = "Review the plan before proceeding"

[[steps]]
role = "implementer"
goal = "Implement the plan"
prompt = "You are an implementation agent..."
requires_approval = false

[[steps]]
role = "reviewer"
goal = "Review the implementation"
prompt = "You are a review agent..."
requires_approval = true
checkpoint_message = "Approve the implementation"
```

## Examples

### Execute Task Workflow

```
User: /workflow