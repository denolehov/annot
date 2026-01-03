import React, { useState, useEffect, useCallback } from 'react';

interface User {
  id: string;
  name: string;
  email: string;
  avatar?: string;
}

interface UserCardProps {
  userId: string;
  onSelect?: (user: User) => void;
  compact?: boolean;
}

/**
 * Displays a user card with avatar, name, and email.
 * Fetches user data on mount and handles loading/error states.
 */
export function UserCard({ userId, onSelect, compact = false }: UserCardProps) {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function fetchUser() {
      try {
        setLoading(true);
        setError(null);

        const response = await fetch(`/api/users/${userId}`);
        if (!response.ok) {
          throw new Error(`Failed to fetch user: ${response.status}`);
        }

        const data = await response.json();
        if (!cancelled) {
          setUser(data);
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : 'Unknown error');
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    fetchUser();

    return () => {
      cancelled = true;
    };
  }, [userId]);

  const handleClick = useCallback(() => {
    if (user && onSelect) {
      onSelect(user);
    }
  }, [user, onSelect]);

  if (loading) {
    return <div className="user-card loading">Loading...</div>;
  }

  if (error) {
    return <div className="user-card error">{error}</div>;
  }

  if (!user) {
    return null;
  }

  return (
    <div
      className={`user-card ${compact ? 'compact' : ''}`}
      onClick={handleClick}
      role="button"
      tabIndex={0}
    >
      {user.avatar && (
        <img
          src={user.avatar}
          alt={`${user.name}'s avatar`}
          className="avatar"
        />
      )}
      <div className="info">
        <h3 className="name">{user.name}</h3>
        {!compact && <p className="email">{user.email}</p>}
      </div>
    </div>
  );
}

export default UserCard;
