use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiElement {
    pub class: String,
    pub resource_id: Option<String>,
    pub text: Option<String>,
    pub content_desc: Option<String>,
    pub bounds: Bounds,
    pub clickable: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bounds {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Bounds {
    pub fn center(&self) -> (i32, i32) {
        ((self.left + self.right) / 2, (self.top + self.bottom) / 2)
    }
}

/// Dump UI hierarchy and parse it
pub async fn dump_ui(device: &str) -> Result<Vec<UiElement>> {
    let remote_path = "/sdcard/adb_mcp_ui.xml";

    let output = Command::new("adb")
        .args(["-s", device, "shell", "uiautomator", "dump", remote_path])
        .output()
        .await
        .context("Failed to dump UI")?;

    if !output.status.success() {
        anyhow::bail!(
            "uiautomator dump failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let xml_data = Command::new("adb")
        .args(["-s", device, "exec-out", "cat", remote_path])
        .output()
        .await
        .context("Failed to pull UI dump")?
        .stdout;

    let _ = Command::new("adb")
        .args(["-s", device, "shell", "rm", "-f", remote_path])
        .output()
        .await;

    let xml_str = String::from_utf8_lossy(&xml_data);
    parse_ui_hierarchy(&xml_str)
}

fn parse_ui_hierarchy(xml: &str) -> Result<Vec<UiElement>> {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    let mut elements = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) if e.name().as_ref() == b"node" => {
                if let Some(element) = parse_node(e) {
                    elements.push(element);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                debug!("XML parse error: {}", e);
                break;
            }
            _ => {}
        }
    }

    Ok(elements)
}

fn parse_node(e: &quick_xml::events::BytesStart) -> Option<UiElement> {
    let mut class = String::new();
    let mut resource_id = None;
    let mut text = None;
    let mut content_desc = None;
    let mut bounds_str = String::new();
    let mut clickable = false;
    let mut enabled = true;

    for attr in e.attributes().filter_map(|a| a.ok()) {
        let key = String::from_utf8_lossy(attr.key.as_ref());
        let value = String::from_utf8_lossy(&attr.value);

        match key.as_ref() {
            "class" => class = value.to_string(),
            "resource-id" => {
                if !value.is_empty() {
                    resource_id = Some(value.to_string());
                }
            }
            "text" => {
                if !value.is_empty() {
                    text = Some(value.to_string());
                }
            }
            "content-desc" => {
                if !value.is_empty() {
                    content_desc = Some(value.to_string());
                }
            }
            "bounds" => bounds_str = value.to_string(),
            "clickable" => clickable = value == "true",
            "enabled" => enabled = value == "true",
            _ => {}
        }
    }

    let bounds = parse_bounds(&bounds_str)?;

    Some(UiElement {
        class,
        resource_id,
        text,
        content_desc,
        bounds,
        clickable,
        enabled,
    })
}

fn parse_bounds(s: &str) -> Option<Bounds> {
    // Format: [left,top][right,bottom]
    let parts: Vec<i32> = s
        .replace('[', "")
        .replace(']', ",")
        .split(',')
        .filter_map(|p| p.parse().ok())
        .collect();

    if parts.len() >= 4 {
        Some(Bounds {
            left: parts[0],
            top: parts[1],
            right: parts[2],
            bottom: parts[3],
        })
    } else {
        None
    }
}

/// Find elements matching criteria
pub fn find_elements<'a>(
    elements: &'a [UiElement],
    text: Option<&str>,
    resource_id: Option<&str>,
    class: Option<&str>,
) -> Vec<&'a UiElement> {
    elements
        .iter()
        .filter(|e| {
            let text_match = text.map_or(true, |t| {
                e.text.as_deref() == Some(t)
                    || e.content_desc.as_deref() == Some(t)
                    || e.text
                        .as_ref()
                        .is_some_and(|et| et.to_lowercase().contains(&t.to_lowercase()))
            });

            let id_match = resource_id.map_or(true, |id| {
                e.resource_id
                    .as_ref()
                    .is_some_and(|rid| rid.contains(id))
            });

            let class_match = class.map_or(true, |c| e.class.contains(c));

            text_match && id_match && class_match
        })
        .collect()
}

/// Get the current foreground activity
pub async fn get_current_activity(device: &str) -> Result<String> {
    let output = Command::new("adb")
        .args([
            "-s",
            device,
            "shell",
            "dumpsys",
            "activity",
            "activities",
            "|",
            "grep",
            "-E",
            "mResumedActivity|mCurrentFocus",
        ])
        .output()
        .await
        .context("Failed to get current activity")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.is_empty() {
        let output = Command::new("adb")
            .args(["-s", device, "shell", "dumpsys", "activity", "activities"])
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("mResumedActivity") || line.contains("mCurrentFocus") {
                return Ok(line.trim().to_string());
            }
        }
    }

    Ok(stdout.trim().to_string())
}

/// Wait for a specific activity to appear
pub async fn wait_for_activity(
    device: &str,
    activity_pattern: &str,
    timeout_ms: u64,
) -> Result<bool> {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(timeout_ms);

    while start.elapsed() < timeout {
        let current = get_current_activity(device).await?;
        if current.contains(activity_pattern) {
            return Ok(true);
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    Ok(false)
}
