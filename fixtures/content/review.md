# Code Review: PR #142 - Add caching layer

## Summary
This PR adds a Redis-based caching layer for API responses.

## What I Like
- Clean separation of cache logic from handlers
- Good use of trait bounds for cache backends
- Comprehensive test coverage

## Concerns

### 1. Cache Invalidation
The current approach invalidates entire cache keys on any mutation. This could cause cache stampedes under high load.

**Suggestion:** Consider probabilistic early expiration or staggered TTLs.

### 2. Error Handling
Cache failures currently bubble up as 500 errors. For a cache layer, we should probably fall through to the database.

```rust
// Current
let data = cache.get(&key).await?;

// Suggested
let data = match cache.get(&key).await {
    Ok(Some(data)) => data,
    Ok(None) | Err(_) => {
        // Log cache miss/error, fall through to DB
        db.fetch(&key).await?
    }
};
```

### 3. Memory Bounds
No max memory limit is configured for the Redis instance. In production, this could lead to OOM issues.

## Minor Nits
- Line 47: Unused import `std::time::Instant`
- Line 89: Consider extracting magic number `3600` to a named constant

## Verdict
**Approve with suggestions** - The core implementation is solid. Address the error handling concern before merging.
