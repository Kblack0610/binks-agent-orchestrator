# Task Grab Skill

This skill grabs the next high-priority task from the task database and executes it through the task execution workflow.

## Workflow

1. **List pending tasks** - Show available tasks ready to be grabbed
2. **Grab next task** - Atomically claim the highest priority task with no incomplete dependencies
3. **Create branch** - Create a feature branch for the task: `task/{id}-{slug}`
4. **Execute workflow** - Run the task-execution workflow from `.binks/workflows/task-execution.toml`
5. **Handle checkpoints** - Interactive approval for plan review and implementation review
6. **Complete task** - Mark task as completed, capture branch and PR information

## Tools Used

- `mcp__task__list_tasks` - List pending tasks
- `mcp__task__grab_next_task` - Atomically grab next available task
- `mcp__task__get_task` - Get task details
- `mcp__task__update_task` - Update task status and metadata
- `mcp__git__git_status` - Check git working tree
- `mcp__git__git_branch_list` - List branches
- `mcp__git__git_stash` - Stash uncommitted changes if needed
- `mcp__workflow__execute_workflow` - Execute task workflow
- `mcp__workflow__get_execution_status` - Poll workflow status
- `mcp__workflow__resume_from_checkpoint` - Respond to checkpoints

## Task Execution

I'll help you grab and execute the next task.

### Step 1: Check Current State

Let me first check your git status to ensure we have a clean working tree.

[Use mcp__git__git_status to check current state]

If there are uncommitted changes, I'll offer to stash them:
- Use `mcp__git__git_stash` with action "save"

### Step 2: List Available Tasks

Let me show you the pending tasks:

[Use mcp__task__list_tasks with filter for status=pending]

Display tasks in priority order:
- Priority level (0-3)
- Task ID and title
- Dependencies (if any)
- Only show tasks that are ready (no incomplete dependencies)

### Step 3: Grab Next Task

I'll grab the highest priority task with no incomplete dependencies:

[Use mcp__task__grab_next_task]

This atomically:
- Selects highest priority task with status=pending and no incomplete dependencies
- Updates status to in_progress
- Sets started_at timestamp
- Returns task details

### Step 4: Create Feature Branch

Creating branch for task {id}: task/{id}-{slug}

[Use mcp__git__git_branch_list to check if branch exists]

If branch doesn't exist:
- Create and switch to new branch
- Use naming pattern: `task/{id}-{slug}` where slug is kebab-case from title

If branch exists:
- Confirm with user whether to switch to existing branch or create new one

### Step 5: Execute Workflow

Starting task execution workflow from `.binks/workflows/task-execution.toml`

[Use mcp__workflow__execute_workflow with:
- workflow: "task-execution"
- task: {task.title}
- context: {"task_id": "{id}", "task_description": "{description}"}
]

Workflow started with execution ID: {execution_id}

### Step 6: Monitor Execution

Monitoring workflow progress...

[Use mcp__workflow__get_execution_status in polling loop]

Display current status:
- Workflow step: {current_step}/{total_steps}
- Status: {status}
- Agent: {current_agent}

### Step 7: Handle Checkpoints

When workflow reaches checkpoint (status="waiting_for_approval"):

**Checkpoint: {checkpoint_name}**

{checkpoint_message}

Options:
- Approve - Continue to next step
- Reject - Provide feedback and retry current step
- Abort - Cancel workflow

[Prompt user for decision]

[Use mcp__workflow__resume_from_checkpoint with:
- execution_id: {execution_id}
- approved: {user_decision}
- feedback: {user_feedback} (if provided)
]

### Step 8: Workflow Completion

When workflow completes (status="completed"):

**Task Execution Complete!**

Summary:
- Task: {task.title}
- Workflow: task-execution
- Execution time: {duration}
- Steps completed: {completed_steps}

[Use mcp__task__update_task to set:
- status: "completed"
- completed_at: now
- branch_name: {current_branch}
- pr_url: {pr_url} (if available in workflow context)
]

Task {id} marked as completed.

### Error Handling

If workflow fails (status="failed"):

**Workflow Failed**

Error: {error_message}

[Use mcp__task__update_task to set:
- status: "pending" (reset to allow retry)
- Add note about failure
]

Task reset to pending. You can retry with `/task-grab` or investigate the failure.

If no tasks available:

**No Tasks Available**

All pending tasks either:
- Have incomplete dependencies
- Are already in progress
- No tasks in database

Create tasks with `/task-create` or check dependency status with task-mcp tools.

## Examples

### Basic Usage

```
User: /task-grab