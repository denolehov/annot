#!/bin/bash
# Generates a large markdown file with code blocks in many languages.
# Used for testing startup performance with syntax highlighting.

set -euo pipefail

OUTPUT="${1:-test-fixtures/large.md}"
TARGET_LINES="${2:-3000}"

cat > "$OUTPUT" << 'HEADER'
# Large Test File for Performance Testing

This file contains many code blocks in different languages to test syntax highlighting performance.

HEADER

echo "Generated: $(date -Iseconds)" >> "$OUTPUT"
echo "" >> "$OUTPUT"

section=0
while [ "$(wc -l < "$OUTPUT")" -lt "$TARGET_LINES" ]; do
    section=$((section + 1))

    cat >> "$OUTPUT" << EOF
---

## Section $section: Rust

\`\`\`rust
/// A generic result type for operations that can fail.
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct Config {
    pub name: String,
    pub value: i64,
    pub enabled: bool,
}

impl Config {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            value: 42,
            enabled: true,
        }
    }
}
\`\`\`

## Section $section: TypeScript

\`\`\`typescript
interface User {
  id: string;
  name: string;
  email: string;
}

async function fetchUser(id: string): Promise<User | null> {
  const response = await fetch(\`/api/users/\${id}\`);
  if (!response.ok) return null;
  return response.json();
}

const processUsers = <T extends User>(users: T[]): Map<string, T> => {
  return new Map(users.map(u => [u.id, u]));
};
\`\`\`

## Section $section: Python

\`\`\`python
from dataclasses import dataclass
from typing import List

@dataclass
class Task:
    id: int
    name: str
    completed: bool = False

class TaskManager:
    def __init__(self):
        self._tasks: List[Task] = []

    async def add_task(self, name: str) -> Task:
        task = Task(id=len(self._tasks), name=name)
        self._tasks.append(task)
        return task
\`\`\`

## Section $section: Go

\`\`\`go
package main

import (
    "context"
    "sync"
)

type Worker struct {
    id     int
    jobs   <-chan Job
    result chan<- Result
    mu     sync.Mutex
}

func (w *Worker) Start(ctx context.Context) {
    go func() {
        for {
            select {
            case job := <-w.jobs:
                w.result <- w.process(job)
            case <-ctx.Done():
                return
            }
        }
    }()
}
\`\`\`

## Section $section: SQL

\`\`\`sql
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    name VARCHAR(100) NOT NULL
);

WITH ranked AS (
    SELECT user_id, order_total,
           ROW_NUMBER() OVER (PARTITION BY user_id ORDER BY order_total DESC) as rank
    FROM orders
)
SELECT * FROM ranked WHERE rank = 1;
\`\`\`

## Section $section: YAML

\`\`\`yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: api-server
spec:
  replicas: 3
  template:
    spec:
      containers:
        - name: api
          image: myapp:v1.2.3
          ports:
            - containerPort: 8080
\`\`\`

## Section $section: Mermaid

\`\`\`mermaid
graph TD
    A[Start] --> B{Auth?}
    B -->|Yes| C[Dashboard]
    B -->|No| D[Login]
    D --> E{Valid?}
    E -->|Yes| C
    E -->|No| D
\`\`\`

EOF
done

lines=$(wc -l < "$OUTPUT")
size=$(du -h "$OUTPUT" | cut -f1)
echo ""
echo "Generated: $OUTPUT"
echo "Lines: $lines"
echo "Size: $size"
