# Fixtures

Sample files for testing and previewing annot's different review modes.

## Usage

```bash
# Open a file for annotation
pnpm tauri dev -- -- fixtures/files/simple.rs

# Open a diff
pnpm tauri dev -- -- --diff fixtures/diffs/simple-add.diff

# Open content (review_content mode)
pnpm tauri dev -- -- --content fixtures/content/plan.md
```

## Directory Structure

```
fixtures/
├── files/        # Sample source files for review_file mode
├── diffs/        # Sample unified diffs for review_diff mode
└── content/      # Sample markdown for review_content mode
```

## Files

| File | Description |
|------|-------------|
| `files/simple.rs` | Basic Rust function (~30 lines) |
| `files/component.tsx` | React component with props, state, effects |
| `files/empty.txt` | Empty file edge case |
| `diffs/simple-add.diff` | Additions only |
| `diffs/simple-remove.diff` | Deletions only |
| `diffs/mixed-changes.diff` | Both additions and deletions |
| `content/plan.md` | Sample implementation plan |
| `content/review.md` | Sample code review feedback |
