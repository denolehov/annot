# Implementation Plan: User Authentication

## Goal
Add secure user authentication with JWT tokens and refresh token rotation.

## Approach

### Phase 1: Core Auth
1. Create `User` model with password hashing (argon2)
2. Implement `/auth/login` endpoint
3. Implement `/auth/register` endpoint
4. Add JWT token generation with 15-minute expiry

### Phase 2: Token Refresh
1. Add `RefreshToken` model with rotation tracking
2. Implement `/auth/refresh` endpoint
3. Add token revocation on logout
4. Handle concurrent refresh race conditions

### Phase 3: Middleware
1. Create `AuthGuard` middleware
2. Extract user from JWT in request context
3. Add role-based access control helpers

## Open Questions
- [ ] Should we support OAuth providers (Google, GitHub)?
- [ ] What's the refresh token expiry? 7 days? 30 days?
- [ ] Do we need device-based token management?

## Files to Create
- `src/models/user.rs`
- `src/models/refresh_token.rs`
- `src/handlers/auth.rs`
- `src/middleware/auth.rs`
- `src/utils/jwt.rs`
