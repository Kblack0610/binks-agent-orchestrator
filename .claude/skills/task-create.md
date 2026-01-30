# Task Create Skill

This skill decomposes a plan file into individual tasks in the task database.

## Workflow

1. **List available plans** - Show plans in `~/.agent/plans/{project}/` or `.claude/plans/`
2. **Select plan** - User chooses which plan to decompose
3. **Parse plan** - Extract tasks from markdown structure
4. **Create tasks** - Insert each task into database via task-mcp
5. **Link dependencies** - Add dependency relationships between tasks
6. **Display summary** - Show created tasks ready for grabbing

## Tools Used

- `mcp__filesystem__list_dir` - List available plan files
- `mcp__filesystem__read_file` - Read plan content
- `mcp__task__create_task` - Create individual tasks
- `mcp__task__add_dependency` - Link task dependencies
- `mcp__task__list_tasks` - Display created tasks

**Note:** This skill currently performs manual plan parsing. Once Phase 3 is complete and plan-parser-mcp is available, it will use:
- `mcp__plan-parser__parse_plan` - Parse markdown into task definitions
- `mcp__plan-parser__validate_plan` - Validate plan syntax

## Plan Parsing Rules

Plans follow this structure:

```markdown
# Plan Title

## Task 1 [P0]
Description of task 1

## Task 2 [P1] @implementer
Description of task 2 (depends: task-1)

## Task 3 [P2]
Description of task 3 (depends: task-1, task-2)
```

**Priority Extraction:**
- `[P0]` - Critical (priority 0)
- `[P1]` - High (priority 1)
- `[P2]` - Normal (priority 2)
- `[P3]` - Low (priority 3)
- No marker - Normal (priority 2)

**Keywords for priority:**
- "CRITICAL", "URGENT", "BLOCKER" → P0
- "HIGH PRIORITY", "IMPORTANT" → P1
- "LOW PRIORITY", "NICE TO HAVE" → P3

**Agent Assignment:**
- `@agent-name` - Assign to specific agent (implementer, planner, tester, reviewer)
- Default: implementer

**Dependencies:**
- `(depends: task-1)` - Single dependency
- `(depends: task-1, task-2)` - Multiple dependencies
- Reference by task ID or sequential position

## Task Creation

I'll help you decompose a plan file into tasks.

### Step 1: List Available Plans

Let me check for available plans:

[Use mcp__filesystem__list_dir for ~/.agent/plans/binks-agent/active/]
[Use mcp__filesystem__list_dir for ~/.agent/plans/binks-agent/planning/]
[Use mcp__filesystem__list_dir for .claude/plans/]

Available plans:

**Active Plans (ready to work on):**
1. ~/.agent/plans/binks-agent/active/{plan-file}.md
2. ...

**Proposed Plans (not yet approved):**
1. ~/.agent/plans/binks-agent/planning/{plan-file}.md
2. ...

**Local Plans:**
1. .claude/plans/{plan-file}.md
2. ...

Which plan would you like to decompose? (provide path or number)

### Step 2: Read and Validate Plan

Reading plan: {selected_plan}

[Use mcp__filesystem__read_file to read plan content]

**Plan:** {title}
**Location:** {path}
**Tasks found:** {count}

Parsing structure:
- Extracting task headings (##)
- Detecting priorities ([P0], [P1], etc.)
- Finding agent assignments (@agent-name)
- Identifying dependencies ((depends: ...))

**Parsed Tasks:**

1. **Task 1** [P0] @implementer
   - Description: {summary}
   - Dependencies: None

2. **Task 2** [P1] @implementer
   - Description: {summary}
   - Dependencies: task-1

3. **Task 3** [P2] @tester
   - Description: {summary}
   - Dependencies: task-1, task-2

Proceed with creating {count} tasks?

### Step 3: Create Tasks in Database

Creating tasks via task-mcp...

[For each parsed task:
  Use mcp__task__create_task with:
  - title: {task_title}
  - description: {task_description}
  - priority: {priority_level}
  - agent: {assigned_agent}
  - metadata: {"plan": "{plan_path}", "section": "{section}"}
]

Created tasks:
- task-1: {title} (ID: {db_id})
- task-2: {title} (ID: {db_id})
- task-3: {title} (ID: {db_id})

### Step 4: Link Dependencies

Setting up task dependencies...

[For each task with dependencies:
  Use mcp__task__add_dependency with:
  - task_id: {dependent_task_id}
  - dependency_id: {prerequisite_task_id}
]

Dependencies established:
- task-2 depends on task-1
- task-3 depends on task-1, task-2

### Step 5: Display Summary

**Plan Decomposition Complete**

Created {count} tasks from {plan_name}:

[Use mcp__task__list_tasks to display all tasks from this plan]

**Priority Breakdown:**
- P0 (Critical): {p0_count}
- P1 (High): {p1_count}
- P2 (Normal): {p2_count}
- P3 (Low): {p3_count}

**Agent Assignments:**
- @implementer: {implementer_count}
- @planner: {planner_count}
- @tester: {tester_count}
- @reviewer: {reviewer_count}

**Next Steps:**
1. Review task list: `mcp__task__list_tasks`
2. Grab first task: `/task-grab`
3. Execute task workflow

Ready to start? Use `/task-grab` to begin work on the highest priority task.

## Error Handling

**Plan not found:**
- Check file path
- Verify plan location (active vs planning vs local)
- Use `mcp__filesystem__list_dir` to browse

**Invalid plan structure:**
- Plans must have ## headings for tasks
- Each task should have description content
- Currently requires manual validation (will use plan-parser-mcp validation in Phase 3)

**Task creation failures:**
- Check database connectivity
- Verify task-mcp is available
- Ensure required fields are present

**Dependency resolution issues:**
- Dependencies must reference existing tasks
- Circular dependencies not allowed
- Use task IDs or sequential references

## Future Enhancement

Once plan-parser-mcp is implemented (Phase 3), this skill will:
- Use structured parsing instead of manual extraction
- Validate plan syntax before task creation
- Support advanced dependency graph analysis
- Handle malformed priorities gracefully
- Provide detailed validation errors

Current version performs manual parsing as an interim solution.

## Examples

### Basic Usage

```
User: /task-create