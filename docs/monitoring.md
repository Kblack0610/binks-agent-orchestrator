# Monitoring System

Autonomous repository monitoring with GitHub polling, inbox reports, and notifications.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Cron / systemd timer                     │
│                    (every 15-30 min)                        │
└────────────────────────────┬────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                     MONITOR AGENT                           │
│  - Poll GitHub for issues/PRs                               │
│  - Check workflow status (CI/CD)                            │
│  - Write reports to ~/.notes/inbox                          │
│  - Send Slack/Discord notifications                         │
└─────────────────────────────────────────────────────────────┘
```

---

## Quick Start

### Run Once (Testing/Cron)

```bash
./agent/target/release/agent monitor --once --repos owner/repo
```

### Run Continuously (Live View)

```bash
./agent/target/release/agent monitor --repos owner/repo --interval 300
```

Output appears directly in your terminal.

### Watch Inbox Updates

In a separate terminal:
```bash
tail -f ~/.notes/inbox/$(date +%Y-%m-%d).md
```

---

## Options

| Flag | Description | Default |
|------|-------------|---------|
| `--repos` | Comma-separated repos (owner/repo format) | Required |
| `--once` | Run single cycle then exit | Continuous |
| `--interval` | Seconds between cycles | 300 |
| `-s, --system` | Custom system prompt | None |

---

## What It Checks

For each repository:
- **Open issues** - Count and titles
- **Open PRs** - Count and titles
- **Workflow runs** - Failed or in-progress CI/CD

Results are written to the inbox with timestamps and tags.

---

## Inbox Format

Location: `~/.notes/inbox/YYYY-MM-DD.md`

```markdown
# Inbox - 2026-01-17

## 2026-01-17 14:30:00 [monitor] #monitor #status
Repo check: owner/repo
3 open issues, 1 open PRs, 1 failed workflows

---

## 2026-01-17 14:30:01 [monitor] #monitor #status
Monitor cycle completed. Checked 2 repos.
```

---

## Notifications

Configure webhooks for Slack/Discord alerts:

```bash
export SLACK_WEBHOOK_URL=https://hooks.slack.com/services/...
export DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/...
```

---

## Cron Setup

Add to crontab (`crontab -e`):

```bash
# Run every 15 minutes
*/15 * * * * /path/to/agent monitor --once --repos owner/repo1,owner/repo2

# Or use a wrapper script with logging
*/15 * * * * /path/to/scripts/run-monitor.sh >> /var/log/binks-monitor.log 2>&1
```

Example wrapper script:
```bash
#!/bin/bash
cd /path/to/binks-agent-orchestrator
export OLLAMA_URL=http://192.168.1.4:11434
export OLLAMA_MODEL=llama3.1:8b
./agent/target/release/agent monitor --once --repos owner/repo
```

---

## Systemd Service (Alternative)

Create `/etc/systemd/system/binks-monitor.service`:

```ini
[Unit]
Description=Binks Repository Monitor
After=network.target

[Service]
Type=simple
User=youruser
WorkingDirectory=/path/to/binks-agent-orchestrator
Environment=OLLAMA_URL=http://192.168.1.4:11434
Environment=OLLAMA_MODEL=llama3.1:8b
ExecStart=/path/to/agent monitor --repos owner/repo --interval 300
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable binks-monitor
sudo systemctl start binks-monitor
sudo journalctl -u binks-monitor -f  # View logs
```
