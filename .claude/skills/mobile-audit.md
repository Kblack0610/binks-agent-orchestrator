---
name: mobile-audit
description: Audit mobile UI designs using RICO dataset patterns and best practices
compat:
  binks_agent: true
io:
  input: "screenshot path, directory, or Playwright capture"
  output: "design audit report + inbox notification"
tools:
  - rico_mcp::*
  - playwright_mcp::browser_snapshot
  - playwright_mcp::browser_take_screenshot
  - filesystem_mcp::read_file
  - inbox_mcp::write_inbox
  - memory_mcp::learn
---

# Mobile Design Audit Skill

This skill analyzes mobile UI screenshots using the RICO dataset (66K+ Android screens) to provide design recommendations, identify similar patterns, and audit for best practices.

## Workflow Overview

Two modes of operation:
1. **Single Screen Mode** - Analyze one screenshot
2. **Batch/Flow Mode** - Analyze multiple screens as a UI flow

### Core Capabilities
- Find similar UI patterns from 66K+ Android screens
- Identify UI components and layout patterns
- Provide design best practices and accessibility guidance
- Analyze flow consistency across multiple screens

---

## Single Screen Mode

### Step 1: Load Screenshot

**Actions:**
- If file path provided: Read image using Claude Vision
- If URL provided: Capture via Playwright
- If directory provided: Switch to Batch Mode

**Example Input:**
```bash
# File path
/mobile-audit ~/screenshots/login-screen.png

# Playwright capture
/mobile-audit https://example.com/mobile --capture

# Batch mode
/mobile-audit ~/screenshots/app-flow/
```

### Step 2: Extract UI Features (Claude Vision)

Analyze the screenshot to identify:
- Component types (buttons, inputs, images, text, etc.)
- Layout structure (centered, grid, list, etc.)
- Visual hierarchy (primary action, secondary elements)
- Color scheme and typography patterns

**Prompt Template:**
```
Analyze this mobile UI screenshot and identify:

1. Component Types - List all UI elements visible (Button, TextInput, Image, Icon, List, etc.)
2. Layout Pattern - Describe the overall layout (centered form, list view, grid, etc.)
3. Visual Hierarchy - What's the primary action? Secondary elements?
4. Component Count - Rough count of each element type
5. Design Pattern - What type of screen is this? (Login, Home, Settings, Detail, etc.)

Format your response as JSON:
{
  "components": ["Button", "TextInput", "Image", ...],
  "layout": "centered form with logo at top",
  "primary_action": "Sign In button",
  "pattern_type": "Login Screen",
  "estimated_component_types": [0, 1, 3, 5]  // RICO component type IDs
}
```

### Step 3: Find Similar Patterns (RICO Search)

Use extracted features to find similar screens in RICO dataset.

**Actions:**
1. Get pattern guidance:
   ```bash
   rico_mcp::get_pattern_guidance(
     pattern_name="Login Screen",
     include_accessibility=true
   )
   ```

2. If layout vector available, search by vector:
   ```bash
   rico_mcp::search_by_vector(
     vector=[...64 floats...],
     top_k=5,
     min_similarity=0.7
   )
   ```

3. Otherwise, check dataset status and sample similar apps:
   ```bash
   rico_mcp::get_dataset_status()
   ```

### Step 4: Analyze Design Quality

Compare against RICO patterns and best practices:

**Checklist:**
- [ ] Component layout follows Material Design guidelines
- [ ] Touch targets are at least 48dp
- [ ] Visual hierarchy is clear
- [ ] Form fields have appropriate labels
- [ ] Primary action is prominent
- [ ] Navigation is consistent
- [ ] Error states are handled

**RICO Component Reference:**
| ID | Component Type | Common Usage |
|----|----------------|--------------|
| 0 | Text | Labels, content |
| 1 | Image | Media, avatars |
| 2 | Icon | Actions, indicators |
| 3 | Text Button | Primary/secondary actions |
| 4 | Toolbar | App bar, navigation |
| 5 | List Item | Data display |
| 6 | Input | Forms, search |
| 7 | Background Image | Branding |
| 8 | Card | Content grouping |
| 9 | Web View | Embedded content |

### Step 5: Generate Report

Create a comprehensive audit report.

**Report Format:**
```markdown
# Mobile UI Audit Report

**Screenshot:** login-screen.png
**Pattern Type:** Login Screen
**Audit Date:** 2026-02-09

## Summary

### Design Score: 8.5/10

**Strengths:**
- Clear visual hierarchy with prominent CTA
- Clean, minimal design
- Follows login screen best practices

**Areas for Improvement:**
- Add password visibility toggle
- Consider biometric login option
- Improve contrast ratio on secondary text

## Component Analysis

| Component | Count | Notes |
|-----------|-------|-------|
| Text Input | 2 | Username, Password |
| Text Button | 2 | Sign In, Forgot Password |
| Image | 1 | Logo |

## Similar RICO Screens

1. **Screen #12345** (Similarity: 0.89)
   - App: Instagram
   - Notable: Similar centered form layout

2. **Screen #23456** (Similarity: 0.85)
   - App: Facebook
   - Notable: Same button placement pattern

## Best Practices Checklist

- [x] Clear primary action
- [x] Minimal form fields
- [ ] Password visibility toggle
- [ ] Biometric login option
- [x] Error state handling
- [x] Keyboard support

## Accessibility Notes

- Touch targets: OK (48dp+)
- Color contrast: Review secondary text
- Screen reader: Add labels to inputs
- Focus indicators: Add visible focus states

## Recommendations

1. **High Priority**
   - Add password visibility toggle button
   - Increase contrast on "Forgot Password" link

2. **Medium Priority**
   - Consider adding biometric authentication
   - Add loading state for submit button

3. **Low Priority**
   - Add subtle animation on logo
   - Consider dark mode support
```

### Step 6: Save and Notify

**Actions:**
1. Save report to filesystem:
   ```bash
   filesystem_mcp::write_file(
     path="~/.notes/audits/mobile/2026-02-09-login-screen.md",
     content="[report content]"
   )
   ```

2. Notify via inbox:
   ```bash
   inbox_mcp::write_inbox(
     message="Mobile UI audit complete for login-screen.png. Score: 8.5/10. View report at ~/.notes/audits/mobile/",
     priority="normal",
     tags=["mobile-audit", "ui-design"],
     source="mobile-audit"
   )
   ```

3. Store learnings (optional):
   ```bash
   memory_mcp::learn(
     entity="audit:login-screen-2026-02-09",
     entity_type="audit",
     facts=[
       {key: "pattern", value: "Login Screen"},
       {key: "score", value: "8.5"},
       {key: "issues", value: "missing password toggle"}
     ]
   )
   ```

---

## Batch/Flow Mode

Analyze multiple screens as a cohesive UI flow.

### Step 1: Load Screenshot Directory

**Actions:**
- List all PNG/JPG files in directory
- Sort by filename (assume naming convention: 01-login.png, 02-home.png, etc.)
- Validate minimum 2 screens for flow analysis

**Example:**
```bash
/mobile-audit ~/screenshots/checkout-flow/
# Finds: 01-cart.png, 02-shipping.png, 03-payment.png, 04-confirm.png
```

### Step 2: Per-Screen Analysis

Run Steps 2-4 from Single Screen Mode for each screenshot:
- Extract UI features with Vision
- Find similar RICO patterns
- Analyze design quality

### Step 3: Flow Analysis

Analyze consistency across all screens:

**Actions:**
1. Collect all screen IDs from RICO matches
2. Compute flow cohesion:
   ```bash
   rico_mcp::analyze_flow(
     screen_ids=[12345, 23456, 34567, 45678],
     analyze_flow=true
   )
   ```

**Flow Cohesion Metrics:**
- **High (>0.8)**: Consistent UI across flow
- **Medium (0.6-0.8)**: Mostly consistent with some variation
- **Low (<0.6)**: Inconsistent, needs improvement

### Step 4: Cross-Screen Report

**Report Additions for Flow Mode:**

```markdown
## Flow Analysis

**Total Screens:** 4
**Flow Cohesion Score:** 0.82 (High - consistent UI flow)

### Screen Sequence
1. Cart (Score: 8.0/10)
2. Shipping (Score: 8.5/10)
3. Payment (Score: 7.5/10) ⚠️
4. Confirmation (Score: 9.0/10)

### Consistency Analysis

**Consistent Across All Screens:**
- Navigation bar (Toolbar component)
- Primary button styling
- Typography hierarchy

**Inconsistencies Detected:**
- Payment screen: Different button placement
- Cart screen: Missing back navigation

### Flow Recommendations

1. **Navigation**
   - Add consistent back navigation on all screens
   - Consider progress indicator for checkout flow

2. **Component Consistency**
   - Standardize button placement across screens
   - Use same input field styling throughout

3. **State Coverage**
   - Missing: Error states for form validation
   - Missing: Loading states for async operations
   - Consider: Empty cart state
```

---

## Trigger Modes

### Manual Trigger
```bash
# Claude CLI
/mobile-audit screenshot.png
/mobile-audit ~/screenshots/flow/ --batch

# Binks agent
binks mobile-audit screenshot.png
```

### Playwright Integration
```bash
# Capture and audit live app
/mobile-audit https://app.example.com --capture

# Capture multiple screens in sequence
/mobile-audit https://app.example.com/login https://app.example.com/home --flow
```

---

## Configuration

Settings in ~/.binks/config.toml:

```toml
[mobile_audit]
# Default similarity threshold
min_similarity = 0.7

# Number of similar screens to find
top_k = 5

# Include accessibility notes
include_accessibility = true

# Report output directory
report_dir = "~/.notes/audits/mobile"

# Notify on completion
notify_enabled = true
inbox_priority = "normal"

# Memory integration
store_learnings = true
```

---

## Error Handling

**Screenshot not found:**
- Report: "File not found: [path]"
- Suggest: "Check path or use Playwright capture"

**RICO dataset not loaded:**
- Check: `rico_mcp::get_dataset_status()`
- Suggest: "Run scripts/download-rico.sh to set up dataset"

**Vision extraction fails:**
- Fallback: Use basic image analysis
- Report: "Limited analysis - Vision extraction failed"

**Flow analysis with <2 screens:**
- Report: "Need at least 2 screens for flow analysis"
- Suggest: "Use single screen mode instead"

---

## Example Workflow

```bash
$ /mobile-audit ~/screenshots/login-v2.png

> Loading screenshot: login-v2.png
> Analyzing with Claude Vision...
>
> Detected Components:
>   - Text Input (2): Username, Password
>   - Text Button (2): Sign In, Forgot Password
>   - Image (1): Logo
>   - Checkbox (1): Remember Me
>
> Pattern Type: Login Screen
>
> Finding similar RICO patterns...
> Found 5 similar screens (similarity: 0.72-0.89)
>
> Checking design best practices...
> ✓ Clear primary action
> ✓ Minimal form fields
> ✗ Missing password visibility toggle
> ✓ Touch targets OK (48dp+)
>
> Design Score: 8.5/10
>
> Report saved: ~/.notes/audits/mobile/2026-02-09-login-v2.md
> Notification sent to inbox
>
> Recommendations:
> 1. Add password visibility toggle
> 2. Consider biometric login option
> 3. Review contrast on secondary text
```

---

## Best Practices

1. **Start with single screens:** Audit individual screens before analyzing flows
2. **Use consistent naming:** Name screenshots in sequence (01-xxx, 02-xxx) for flow analysis
3. **Review RICO matches:** Similar screens can inspire improvements
4. **Check accessibility:** Always enable include_accessibility flag
5. **Store learnings:** Use memory integration to track recurring issues
6. **Iterate on design:** Re-audit after making changes to verify improvements
