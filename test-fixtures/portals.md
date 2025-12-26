# Portal Demo

This markdown file demonstrates the portal feature - inline code embeds from external files.

## Example: Rust Code Portal

Here's the main entry point of the app:

[main](../src-tauri/src/main.rs#L1-L20)

## Example: Multiple Portals

Here's the portal module:

[portal.rs](../src-tauri/src/portal.rs#L1-L50)

And here's some validation logic:

[validation](../src-tauri/src/portal.rs#L100-L130)

## Example: Portal Without Label

[](../src-tauri/src/lib.rs#L1-L15)

## Example: Single Line Portal

Check out this function: [generate_id](../src-tauri/src/state.rs#L123-L126)

## Regular Content

This is regular markdown content between portals.

- Bullet point 1
- Bullet point 2
- Bullet point 3

## Code Block (Not a Portal)

```rust
// This is a regular code block, NOT a portal
fn example() {
    println!("Hello, world!");
}
```

## Testing Notes

1. Portal content should be syntax highlighted
2. Annotations on portal lines should be stored on the source file
3. Selection should not cross portal boundaries
4. Output should group annotations by file
