# Spec: Clipboard Mode

## Goal

Enable terminal-less annotation workflow: copy LLM response → launch annot via Spotlight → annotate → close → paste annotated response back.

## Design: Wrapper & Worker

```
~/Applications/annot.app/
└── Contents/
    ├── Info.plist           # CFBundleExecutable: annot-launcher
    ├── MacOS/
    │   ├── annot-launcher   # Wrapper — spawns annot with --clipboard
    │   └── annot            # Worker — the real Tauri binary
    └── Resources/

~/.local/bin/annot → symlink to ~/Applications/annot.app/Contents/MacOS/annot
```

## Implementation

### 1. Cargo Configuration

```toml
# src-tauri/Cargo.toml
[[bin]]
name = "annot"
path = "src/main.rs"

[[bin]]
name = "annot-launcher"
path = "src/bin/launcher.rs"
```

### 2. The Launcher

```rust
// src-tauri/src/bin/launcher.rs
use std::{env, process::Command};

fn main() {
    if let Err(e) = run() {
        eprintln!("annot launcher error: {e}");
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let exe = env::current_exe()?;
    let macos_dir = exe.parent().ok_or("no parent")?;
    let worker = macos_dir.join("annot");

    Command::new(&worker)
        .arg("--clipboard")
        .spawn()?;

    Ok(())
}
```

### 3. CLI Changes

```rust
// main.rs - add to Cli struct
#[arg(long, help = "Read input from clipboard, write output to clipboard")]
clipboard: bool,
```

### 4. Input Mode

```rust
// input.rs
pub enum InputMode {
    File { path: PathBuf },
    Stdin { label: String },
    Clipboard { label: String, content: String },  // NEW — carries payload
}

pub enum CliSource {
    File { path: PathBuf },
    Stdin { label: String },
    Clipboard { label: String },  // NEW
}
```

### 5. Detection Logic

```rust
pub fn detect(file: Option<PathBuf>, label: String, clipboard: bool)
    -> Result<(InputMode, Option<String>), AnnotError>
{
    if let Some(path) = file {
        let warning = if !io::stdin().is_terminal() {
            Some("Warning: stdin ignored when file provided".into())
        } else {
            None
        };
        Ok((InputMode::File { path }, warning))
    } else if clipboard {
        // Read from system clipboard (eagerly — avoids race with later reads)
        let content = arboard::Clipboard::new()
            .and_then(|mut cb| cb.get_text())
            .map_err(|e| AnnotError::Validation(format!("clipboard error: {e}")))?;

        if content.trim().is_empty() {
            return Err(AnnotError::Validation("clipboard is empty".into()));
        }

        Ok((InputMode::Clipboard { label, content }, None))
    } else if !io::stdin().is_terminal() {
        Ok((InputMode::Stdin { label }, None))
    } else {
        Err(AnnotError::Validation(
            "no input provided\nUsage: annot <file> | <cmd> | annot | annot --clipboard".into()
        ))
    }
}
```

### 6. Clipboard Safety

In clipboard mode, preserve the user's clipboard on cancel:

```rust
// In Review or a dedicated ClipboardSession struct
pub struct ClipboardState {
    original: String,  // Saved on startup
}

impl ClipboardState {
    pub fn capture() -> Result<Self, arboard::Error> {
        let original = arboard::Clipboard::new()?.get_text()?;
        Ok(Self { original })
    }

    pub fn restore(&self) -> Result<(), arboard::Error> {
        arboard::Clipboard::new()?.set_text(&self.original)
    }

    pub fn write_result(&self, text: &str) -> Result<(), arboard::Error> {
        arboard::Clipboard::new()?.set_text(text)
    }
}
```

**Lifecycle:**
- **Startup** (clipboard mode): `ClipboardState::capture()` → store in Review
- **Submit/Finish**: `write_result()` → show toast "Copied to clipboard"
- **Cancel** (Cmd+Q, Escape, window close without submit): `restore()` or leave unchanged

### 7. Output Routing

```rust
// commands.rs - in finish_review
let output_mode = if review.is_mcp() {
    OutputMode::Mcp
} else if review.is_clipboard() {
    OutputMode::Clipboard
} else {
    OutputMode::Cli
};

let result = format_output(&review, output_mode);

if review.is_clipboard() {
    review.clipboard_state()
        .write_result(&result.text)
        .map_err(|e| format!("clipboard write failed: {e}"))?;
    // Frontend shows "Copied to clipboard" toast
} else if !result.text.is_empty() {
    print!("{}", result.text);
}
```

### 8. Window Focus (macOS)

```rust
// main.rs - call after window creation when in clipboard mode
#[cfg(target_os = "macos")]
fn activate_app() {
    use cocoa::appkit::NSApp;
    use objc::{msg_send, sel, sel_impl};
    unsafe {
        let app = NSApp();
        let _: () = msg_send![app, activateIgnoringOtherApps: true];
    }
}
```

## Build Pipeline

Tauri's signing has edge cases with multiple binaries. Use a split pipeline:

### GitHub Actions Workflow

```yaml
- name: Build Tauri app
  env:
    APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}
    APPLE_ID: ${{ secrets.APPLE_ID }}
    APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}
    APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
  run: pnpm tauri build

- name: Build launcher binary
  run: |
    cd src-tauri
    cargo build --release --bin annot-launcher

- name: Assemble and re-sign bundle
  env:
    APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}
  run: |
    BUNDLE="src-tauri/target/release/bundle/macos/annot.app"

    # Copy launcher into bundle
    cp src-tauri/target/release/annot-launcher "$BUNDLE/Contents/MacOS/"

    # Update Info.plist to use launcher as entry point
    /usr/libexec/PlistBuddy -c "Set :CFBundleExecutable annot-launcher" \
      "$BUNDLE/Contents/Info.plist"

    # Re-sign the entire bundle (inside-out)
    codesign --force --options runtime --sign "$APPLE_SIGNING_IDENTITY" \
      "$BUNDLE/Contents/MacOS/annot"
    codesign --force --options runtime --sign "$APPLE_SIGNING_IDENTITY" \
      "$BUNDLE/Contents/MacOS/annot-launcher"
    codesign --force --options runtime --sign "$APPLE_SIGNING_IDENTITY" \
      "$BUNDLE"

- name: Re-notarize bundle
  env:
    APPLE_ID: ${{ secrets.APPLE_ID }}
    APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}
    APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
  run: |
    BUNDLE="src-tauri/target/release/bundle/macos/annot.app"

    # Create zip for notarization
    ditto -c -k --keepParent "$BUNDLE" annot-notarize.zip

    # Submit and wait
    xcrun notarytool submit annot-notarize.zip \
      --apple-id "$APPLE_ID" \
      --password "$APPLE_PASSWORD" \
      --team-id "$APPLE_TEAM_ID" \
      --wait

    # Staple the ticket
    xcrun stapler staple "$BUNDLE"
```

## Install Script Changes

```bash
# scripts/install.sh
APP_DIR="$HOME/Applications"
mkdir -p "$APP_DIR"  # ~/Applications may not exist

# ... download and extract ...

mv annot.app "$APP_DIR/"
ln -sf "$APP_DIR/annot.app/Contents/MacOS/annot" "$BIN_DIR/annot"
```

## Decisions

| Decision | Rationale |
|----------|-----------|
| Launcher pattern | Explicit intent, no ambiguity |
| Split build pipeline | Tauri signing unreliable for multi-binary |
| Manual re-notarization | Required after modifying signed bundle |
| Install to ~/Applications | Spotlight indexes reliably |
| No LSUIElement | App appears in Cmd-Tab |
| Default label `response.md` | Optimizes for LLM use case |

## Implementation Checklist

1. [ ] `src-tauri/Cargo.toml` — Add launcher binary target
2. [ ] `src-tauri/src/bin/launcher.rs` — Implement launcher
3. [ ] `src-tauri/src/main.rs` — Add `--clipboard` flag, add `activate_app()`
4. [ ] `src-tauri/src/input.rs` — Add `InputMode::Clipboard { label, content }`, `CliSource::Clipboard`
5. [ ] `src-tauri/src/review.rs` — Add `ClipboardState` for original clipboard preservation
6. [ ] `src-tauri/src/commands.rs` — Route output to clipboard, restore on cancel
7. [ ] Frontend — "Copied to clipboard" toast on successful finish
8. [ ] `.github/workflows/release.yml` — Split build + assemble + re-sign + re-notarize
9. [ ] `scripts/install.sh` — Change to ~/Applications, add mkdir -p
10. [ ] **Test**: Window focus when spawned from dying launcher
11. [ ] **Test**: Cancel preserves original clipboard

## Scope

**In**: `--clipboard` flag, launcher binary, proper signing pipeline, ~/Applications install

**Out**: Image clipboard, rich text, auto-detect (using explicit flag instead)
