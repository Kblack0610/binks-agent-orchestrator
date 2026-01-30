//! Tests for task-mcp repository

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use super::super::repository::{NewTask, TaskFilter, TaskRepository};
    use super::super::types::TaskStatus;

    /// Create an in-memory test repository
    fn create_test_repo() -> TaskRepository {
        // Use :memory: for in-memory SQLite database
        use std::path::PathBuf;
        let db_path = PathBuf::from(":memory:");
        TaskRepository::new(db_path).unwrap()
    }

    #[test]
    fn test_create_and_get_task() {
        let repo = create_test_repo();

        let new_task = NewTask {
            title: "Test Task".to_string(),
            description: "Test Description".to_string(),
            priority: Some(50),
            plan_source: None,
            plan_section: None,
            assigned_to: None,
            parent_task_id: None,
            metadata: None,
        };

        // Create task
        let task = repo.create_task(new_task).unwrap();
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.description, "Test Description");
        assert_eq!(task.priority, 50);
        assert_eq!(task.status, TaskStatus::Pending);

        // Get task by ID
        let fetched = repo.get_task(&task.id).unwrap().unwrap();
        assert_eq!(fetched.id, task.id);
        assert_eq!(fetched.title, "Test Task");
    }

    #[test]
    fn test_list_tasks_with_filters() {
        let repo = create_test_repo();

        // Create multiple tasks
        for i in 0..5 {
            let new_task = NewTask {
                title: format!("Task {}", i),
                description: format!("Description {}", i),
                priority: Some(i * 10),
                plan_source: Some("test-plan".to_string()),
                plan_section: None,
                assigned_to: None,
                parent_task_id: None,
                metadata: None,
            };
            repo.create_task(new_task).unwrap();
        }

        // List all tasks
        let filter = TaskFilter::default();
        let tasks = repo.list_tasks(filter).unwrap();
        assert_eq!(tasks.len(), 5);

        // Filter by plan source
        let filter = TaskFilter {
            plan_source: Some("test-plan".to_string()),
            ..Default::default()
        };
        let tasks = repo.list_tasks(filter).unwrap();
        assert_eq!(tasks.len(), 5);

        // Filter by status
        let filter = TaskFilter {
            status: Some("pending".to_string()),
            ..Default::default()
        };
        let tasks = repo.list_tasks(filter).unwrap();
        assert_eq!(tasks.len(), 5);
    }

    #[test]
    fn test_update_task_status() {
        let repo = create_test_repo();

        let new_task = NewTask {
            title: "Test Task".to_string(),
            description: "Test Description".to_string(),
            priority: Some(50),
            plan_source: None,
            plan_section: None,
            assigned_to: None,
            parent_task_id: None,
            metadata: None,
        };

        let task = repo.create_task(new_task).unwrap();
        assert_eq!(task.status, TaskStatus::Pending);

        // Update status to in_progress
        repo.update_task_fields(
            &task.id,
            Some(TaskStatus::InProgress),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let updated = repo.get_task(&task.id).unwrap().unwrap();
        assert_eq!(updated.status, TaskStatus::InProgress);
        assert!(updated.started_at.is_some());
    }

    #[test]
    fn test_add_and_list_dependencies() {
        let repo = create_test_repo();

        // Create two tasks
        let task1 = repo
            .create_task(NewTask {
                title: "Task 1".to_string(),
                description: "First task".to_string(),
                priority: Some(50),
                plan_source: None,
                plan_section: None,
                assigned_to: None,
                parent_task_id: None,
                metadata: None,
            })
            .unwrap();

        let task2 = repo
            .create_task(NewTask {
                title: "Task 2".to_string(),
                description: "Second task".to_string(),
                priority: Some(50),
                plan_source: None,
                plan_section: None,
                assigned_to: None,
                parent_task_id: None,
                metadata: None,
            })
            .unwrap();

        // Add dependency: task2 depends on task1
        repo.add_dependency(&task2.id, &task1.id).unwrap();

        // List dependencies
        let deps = repo.get_dependencies(&task2.id).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].depends_on_task_id, task1.id);

        // Check blocking tasks
        let blocking = repo.check_blocking_tasks(&task2.id).unwrap();
        assert_eq!(blocking.len(), 1);
        assert_eq!(blocking[0].id, task1.id);
    }

    #[test]
    fn test_grab_next_task() {
        let repo = create_test_repo();

        // Create a task
        let new_task = NewTask {
            title: "Available Task".to_string(),
            description: "Ready to grab".to_string(),
            priority: Some(100),
            plan_source: None,
            plan_section: None,
            assigned_to: None,
            parent_task_id: None,
            metadata: None,
        };
        repo.create_task(new_task).unwrap();

        // Grab task
        let task = repo.grab_next_task("test-agent", None).unwrap().unwrap();
        assert_eq!(task.title, "Available Task");
        assert_eq!(task.assigned_to.as_deref(), Some("test-agent"));
        assert_eq!(task.status, TaskStatus::InProgress);

        // Try to grab again - should be None
        let result = repo.grab_next_task("test-agent", None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_concurrent_access() {
        use std::thread;

        let repo = create_test_repo();

        // Clone repo for thread safety
        let repo1 = repo.clone();
        let repo2 = repo.clone();

        // Spawn two threads that create tasks concurrently
        let handle1 = thread::spawn(move || {
            for i in 0..10 {
                let new_task = NewTask {
                    title: format!("Thread1 Task {}", i),
                    description: "From thread 1".to_string(),
                    priority: Some(50),
                    plan_source: None,
                    plan_section: None,
                    assigned_to: None,
                    parent_task_id: None,
                    metadata: None,
                };
                repo1.create_task(new_task).unwrap();
            }
        });

        let handle2 = thread::spawn(move || {
            for i in 0..10 {
                let new_task = NewTask {
                    title: format!("Thread2 Task {}", i),
                    description: "From thread 2".to_string(),
                    priority: Some(50),
                    plan_source: None,
                    plan_section: None,
                    assigned_to: None,
                    parent_task_id: None,
                    metadata: None,
                };
                repo2.create_task(new_task).unwrap();
            }
        });

        handle1.join().unwrap();
        handle2.join().unwrap();

        // Verify all tasks were created
        let filter = TaskFilter::default();
        let tasks = repo.list_tasks(filter).unwrap();
        assert_eq!(tasks.len(), 20);
    }
}
