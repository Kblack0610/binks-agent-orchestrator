//! Tests for agent-registry-mcp repository

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use super::super::repository::{AgentFilter, AgentRegistryRepository, ClaimFilter, NewAgent};
    use super::super::types::AgentStatus;

    /// Create an in-memory test repository
    fn create_test_repo() -> AgentRegistryRepository {
        use std::path::PathBuf;
        let db_path = PathBuf::from(":memory:");
        AgentRegistryRepository::new(db_path).unwrap()
    }

    /// Create a default test agent input
    fn test_agent(name: &str) -> NewAgent {
        NewAgent {
            name: name.to_string(),
            agent_type: "claude_code".to_string(),
            pid: Some(12345),
            port: Some(8080),
            working_directory: Some("/home/user/project".to_string()),
            active_project: Some("binks-agent-orchestrator".to_string()),
            capabilities: Some(vec!["code_review".to_string(), "planning".to_string()]),
            ttl_seconds: 300,
            metadata: None,
        }
    }

    #[test]
    fn test_register_and_get_agent() {
        let repo = create_test_repo();

        let agent = repo.register_agent(test_agent("test-agent-1")).unwrap();
        assert_eq!(agent.name, "test-agent-1");
        assert_eq!(agent.agent_type, "claude_code");
        assert_eq!(agent.status, AgentStatus::Active);
        assert_eq!(agent.ttl_seconds, 300);
        assert!(agent.capabilities.is_some());

        // Get by ID
        let fetched = repo.get_agent(&agent.agent_id).unwrap().unwrap();
        assert_eq!(fetched.agent_id, agent.agent_id);
        assert_eq!(fetched.name, "test-agent-1");
    }

    #[test]
    fn test_list_agents_with_filters() {
        let repo = create_test_repo();

        // Register multiple agents
        let a1 = repo.register_agent(test_agent("agent-1")).unwrap();
        let _a2 = repo.register_agent(NewAgent {
            name: "agent-2".to_string(),
            agent_type: "binks".to_string(),
            active_project: Some("other-project".to_string()),
            ..test_agent("agent-2")
        }).unwrap();

        // List all
        let all = repo
            .list_agents(AgentFilter::default())
            .unwrap();
        assert_eq!(all.len(), 2);

        // Filter by type
        let binks_only = repo
            .list_agents(AgentFilter {
                agent_type: Some("binks".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(binks_only.len(), 1);
        assert_eq!(binks_only[0].agent_type, "binks");

        // Filter by project
        let project_filter = repo
            .list_agents(AgentFilter {
                active_project: Some("other-project".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(project_filter.len(), 1);

        // Deregistered agents are excluded by default
        repo.deregister_agent(&a1.agent_id).unwrap();
        let after_dereg = repo
            .list_agents(AgentFilter::default())
            .unwrap();
        assert_eq!(after_dereg.len(), 1);

        // Include stale/deregistered
        let with_stale = repo
            .list_agents(AgentFilter {
                include_stale: true,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(with_stale.len(), 2);
    }

    #[test]
    fn test_heartbeat_updates_timestamp() {
        let repo = create_test_repo();

        let agent = repo.register_agent(test_agent("heartbeat-test")).unwrap();
        let original_heartbeat = agent.last_heartbeat.clone();

        // Small delay to ensure timestamp differs
        std::thread::sleep(std::time::Duration::from_millis(10));

        let updated = repo
            .heartbeat(&agent.agent_id, None, None, None)
            .unwrap();

        assert!(updated.last_heartbeat >= original_heartbeat);
    }

    #[test]
    fn test_heartbeat_updates_status_and_project() {
        let repo = create_test_repo();

        let agent = repo.register_agent(test_agent("heartbeat-update")).unwrap();
        assert_eq!(agent.status, AgentStatus::Active);

        let updated = repo
            .heartbeat(
                &agent.agent_id,
                Some("busy"),
                Some("new-project"),
                None,
            )
            .unwrap();

        assert_eq!(updated.status, AgentStatus::Busy);
        assert_eq!(updated.active_project.as_deref(), Some("new-project"));
    }

    #[test]
    fn test_deregister_releases_claims() {
        let repo = create_test_repo();

        let agent = repo.register_agent(test_agent("dereg-test")).unwrap();

        // Claim port and resource
        let port_result = repo.claim_port(&agent.agent_id, 3000, Some("api")).unwrap();
        assert!(port_result.success);

        let resource_result = repo
            .claim_resource(
                &agent.agent_id,
                "directory",
                "/home/user/project",
                true,
                Some("editing"),
            )
            .unwrap();
        assert!(resource_result.success);

        // Deregister
        repo.deregister_agent(&agent.agent_id).unwrap();

        // Port should be free
        let port_holder = repo.who_has_port(3000).unwrap();
        assert!(port_holder.is_none());

        // Agent should be deregistered
        let agent = repo.get_agent(&agent.agent_id).unwrap().unwrap();
        assert_eq!(agent.status, AgentStatus::Deregistered);
        assert!(agent.deregistered_at.is_some());
    }

    #[test]
    fn test_port_claim_conflict() {
        let repo = create_test_repo();

        let agent1 = repo.register_agent(test_agent("port-agent-1")).unwrap();
        let agent2 = repo
            .register_agent(NewAgent {
                name: "port-agent-2".to_string(),
                ..test_agent("port-agent-2")
            })
            .unwrap();

        // Agent 1 claims port
        let result1 = repo
            .claim_port(&agent1.agent_id, 8080, Some("dev-server"))
            .unwrap();
        assert!(result1.success);

        // Agent 2 tries to claim the same port
        let result2 = repo
            .claim_port(&agent2.agent_id, 8080, Some("api"))
            .unwrap();
        assert!(!result2.success);
        assert!(result2.conflict.is_some());

        let conflict = result2.conflict.unwrap();
        assert_eq!(conflict.held_by_agent_id, agent1.agent_id);
        assert_eq!(conflict.held_by_name, "port-agent-1");
    }

    #[test]
    fn test_port_claim_idempotent() {
        let repo = create_test_repo();

        let agent = repo.register_agent(test_agent("idem-test")).unwrap();

        // Claim port twice
        let result1 = repo.claim_port(&agent.agent_id, 9090, None).unwrap();
        assert!(result1.success);

        let result2 = repo.claim_port(&agent.agent_id, 9090, None).unwrap();
        assert!(result2.success);
        assert!(result2.conflict.is_none());
    }

    #[test]
    fn test_who_has_port() {
        let repo = create_test_repo();

        let agent = repo.register_agent(test_agent("who-port-test")).unwrap();
        repo.claim_port(&agent.agent_id, 5000, Some("debug")).unwrap();

        let result = repo.who_has_port(5000).unwrap();
        assert!(result.is_some());

        let (claim, holder) = result.unwrap();
        assert_eq!(claim.port, 5000);
        assert_eq!(claim.purpose.as_deref(), Some("debug"));
        assert_eq!(holder.agent_id, agent.agent_id);

        // Unclaimed port
        let empty = repo.who_has_port(9999).unwrap();
        assert!(empty.is_none());
    }

    #[test]
    fn test_resource_claim_exclusive_conflict() {
        let repo = create_test_repo();

        let agent1 = repo.register_agent(test_agent("res-agent-1")).unwrap();
        let agent2 = repo
            .register_agent(NewAgent {
                name: "res-agent-2".to_string(),
                ..test_agent("res-agent-2")
            })
            .unwrap();

        // Agent 1 claims a directory exclusively
        let result1 = repo
            .claim_resource(&agent1.agent_id, "directory", "/tmp/project", true, None)
            .unwrap();
        assert!(result1.success);

        // Agent 2 tries exclusive claim on the same directory
        let result2 = repo
            .claim_resource(&agent2.agent_id, "directory", "/tmp/project", true, None)
            .unwrap();
        assert!(!result2.success);
        assert!(result2.conflict.is_some());

        // Agent 2 also can't get a shared claim (blocked by exclusive)
        let result3 = repo
            .claim_resource(&agent2.agent_id, "directory", "/tmp/project", false, None)
            .unwrap();
        assert!(!result3.success);
    }

    #[test]
    fn test_resource_claim_shared_no_conflict() {
        let repo = create_test_repo();

        let agent1 = repo.register_agent(test_agent("shared-1")).unwrap();
        let agent2 = repo
            .register_agent(NewAgent {
                name: "shared-2".to_string(),
                ..test_agent("shared-2")
            })
            .unwrap();

        // Both agents claim shared access
        let result1 = repo
            .claim_resource(
                &agent1.agent_id,
                "project",
                "binks-orchestrator",
                false,
                Some("reading"),
            )
            .unwrap();
        assert!(result1.success);

        let result2 = repo
            .claim_resource(
                &agent2.agent_id,
                "project",
                "binks-orchestrator",
                false,
                Some("reading"),
            )
            .unwrap();
        assert!(result2.success);
    }

    #[test]
    fn test_release_all_for_agent() {
        let repo = create_test_repo();

        let agent = repo.register_agent(test_agent("release-all")).unwrap();

        // Claim multiple resources
        repo.claim_port(&agent.agent_id, 3000, None).unwrap();
        repo.claim_port(&agent.agent_id, 3001, None).unwrap();
        repo.claim_resource(&agent.agent_id, "directory", "/tmp/a", true, None)
            .unwrap();

        let (ports, resources) = repo.release_all_for_agent(&agent.agent_id).unwrap();
        assert_eq!(ports, 2);
        assert_eq!(resources, 1);

        // Verify port is free
        assert!(repo.who_has_port(3000).unwrap().is_none());
    }

    #[test]
    fn test_who_is_working_on() {
        let repo = create_test_repo();

        let agent1 = repo
            .register_agent(NewAgent {
                active_project: Some("binks-agent-orchestrator".to_string()),
                ..test_agent("worker-1")
            })
            .unwrap();

        let _agent2 = repo
            .register_agent(NewAgent {
                name: "worker-2".to_string(),
                active_project: Some("other-project".to_string()),
                ..test_agent("worker-2")
            })
            .unwrap();

        // Also claim a resource on the same project
        repo.claim_resource(
            &agent1.agent_id,
            "project",
            "binks-agent-orchestrator",
            false,
            None,
        )
        .unwrap();

        let result = repo
            .who_is_working_on("binks-agent-orchestrator", None)
            .unwrap();

        assert_eq!(result.agents.len(), 1);
        assert_eq!(result.agents[0].name, "worker-1");
        assert_eq!(result.claims.len(), 1);
    }

    #[test]
    fn test_list_claims() {
        let repo = create_test_repo();

        let agent = repo.register_agent(test_agent("claims-test")).unwrap();

        repo.claim_port(&agent.agent_id, 4000, Some("api")).unwrap();
        repo.claim_resource(&agent.agent_id, "branch", "feat/new-feature", true, None)
            .unwrap();

        let (ports, resources) = repo
            .list_claims(ClaimFilter {
                agent_id: Some(agent.agent_id.clone()),
                active_only: true,
                ..Default::default()
            })
            .unwrap();

        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port, 4000);
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].resource_identifier, "feat/new-feature");
    }

    #[test]
    fn test_stale_detection_and_cleanup() {
        let repo = create_test_repo();

        // Register an agent with a very short TTL
        let agent = repo
            .register_agent(NewAgent {
                ttl_seconds: 0, // Immediately stale
                ..test_agent("stale-test")
            })
            .unwrap();

        // Claim a port
        repo.claim_port(&agent.agent_id, 7000, None).unwrap();

        // Dry run should find the stale agent
        let dry_result = repo.cleanup_stale(true).unwrap();
        assert_eq!(dry_result.expired_agents.len(), 1);
        assert!(dry_result.dry_run);
        assert_eq!(dry_result.released_port_claims, 0); // Dry run doesn't release

        // Port should still be claimed
        assert!(repo.who_has_port(7000).unwrap().is_some());

        // Actual cleanup
        let result = repo.cleanup_stale(false).unwrap();
        assert_eq!(result.expired_agents.len(), 1);
        assert_eq!(result.released_port_claims, 1);
        assert!(!result.dry_run);

        // Agent is now stale
        let agent = repo.get_agent(&agent.agent_id).unwrap().unwrap();
        assert_eq!(agent.status, AgentStatus::Stale);

        // Port is free (stale agent's claims released, so who_has_port won't find it)
        assert!(repo.who_has_port(7000).unwrap().is_none());
    }

    #[test]
    fn test_update_agent_fields() {
        let repo = create_test_repo();

        let agent = repo.register_agent(test_agent("update-test")).unwrap();

        repo.update_agent_fields(
            &agent.agent_id,
            Some("busy"),
            Some("new-project"),
            Some("/new/dir"),
            Some(&["testing".to_string()]),
            Some("{\"key\": \"value\"}"),
        )
        .unwrap();

        let updated = repo.get_agent(&agent.agent_id).unwrap().unwrap();
        assert_eq!(updated.status, AgentStatus::Busy);
        assert_eq!(updated.active_project.as_deref(), Some("new-project"));
        assert_eq!(updated.working_directory.as_deref(), Some("/new/dir"));
        assert_eq!(
            updated.capabilities,
            Some(vec!["testing".to_string()])
        );
        assert_eq!(updated.metadata.as_deref(), Some("{\"key\": \"value\"}"));
    }

    #[test]
    fn test_port_released_after_agent_stale() {
        let repo = create_test_repo();

        // Agent 1 registers with 0 TTL and claims a port
        let agent1 = repo
            .register_agent(NewAgent {
                ttl_seconds: 0,
                ..test_agent("stale-port-1")
            })
            .unwrap();
        repo.claim_port(&agent1.agent_id, 6000, None).unwrap();

        // Cleanup marks agent1 as stale
        repo.cleanup_stale(false).unwrap();

        // Agent 2 should be able to claim the port now
        let agent2 = repo
            .register_agent(NewAgent {
                name: "port-claimer-2".to_string(),
                ..test_agent("port-claimer-2")
            })
            .unwrap();

        let result = repo.claim_port(&agent2.agent_id, 6000, None).unwrap();
        assert!(result.success);
    }
}
