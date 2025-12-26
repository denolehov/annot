# Spec: Unified Coordinate System

## Goal

Simplify the annotation coordinate system by embedding target paths directly in line origins, eliminating the need for separate locus types or ambiguous `file_index`/`file_path` parameters.

## Design

### LineOrigin (3 variants)

```typescript
LineOrigin =
  | { type: 'source'; path: string; line: number }
  | { type: 'diff'; path: string; old_line: number | null; new_line: number | null }
  | { type: 'virtual' }
```

Every annotatable line carries its target `path`. For ephemeral content, `path` is the label.

### LineSemantics (portal_id moved here)

```typescript
PortalSemantics = { 
  kind: 'header' | 'content' | 'footer'; 
  portal_id: string;
  // header also has: label, path, range
}
```

### IPC Command

```rust
fn upsert_annotation(
    path: String,        // from origin.path
    start_line: u32,
    end_line: u32,
    content: Vec<ContentNode>,
) -> Result<(), String>
```

### Frontend Validation (validateRange)

1. All lines in range must have non-virtual origin
2. All lines must share same `origin.path`
3. All lines must share same `semantics.portal_id` (if any)

### Backend Validation

- Validate `path` exists in `review.files` (registered during content load)

## Decisions

- **Merge 'document' and 'external' into 'source'**: Both represent source file lines with a path. The distinction was artificial.
- **Move portal_id to semantics**: Portal-ness is a presentation concern, not an origin concern. Origin answers "where from?", semantics answers "how presented?"
- **Use path as the sole target identifier**: No more `file_index` vs `file_path` ambiguity. Diff files use their path from diff metadata.
- **Ephemeral content path = label**: Keeps the model uniform without special cases.

## Scope

### In
- Rename `LineOrigin::Document` → `LineOrigin::Source`
- Remove `LineOrigin::External`, merge into `Source`
- Add `path` field to `LineOrigin::Source` and `LineOrigin::Diff`
- Move `portal_id` from origin to `PortalSemantics`
- Update `upsert_annotation` command signature: `(path, start_line, end_line, content)`
- Update frontend `rangeToSourceCoords` → `validateRange` with new logic
- Update backend `resolve_target_mut` to use path directly

### Out
- Output format changes (uses FileKey internally, unchanged)
- UI changes (validation logic only)
- Tag/exit-mode system (unrelated)