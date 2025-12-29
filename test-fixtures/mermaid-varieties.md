# Mermaid Diagram Varieties

Test file for viewport positioning - tall diagrams should show from top.

## Short Flowchart

```mermaid
flowchart LR
    A --> B --> C
```

## Medium Flowchart

```mermaid
flowchart TD
    A[Start] --> B{Decision}
    B -->|Yes| C[Do something]
    B -->|No| D[Do other thing]
    C --> E[End]
    D --> E
```

## Tall Flowchart (should show from top)

```mermaid
flowchart TD
    A[User Request] --> B{Authenticated?}
    B -->|No| C[Show Login]
    C --> D[Enter Credentials]
    D --> E{Valid?}
    E -->|No| C
    E -->|Yes| F[Create Session]
    B -->|Yes| F
    F --> G[Load Dashboard]
    G --> H{Has Notifications?}
    H -->|Yes| I[Show Badge]
    H -->|No| J[Skip]
    I --> K[Render UI]
    J --> K
    K --> L[Wait for Action]
    L --> M{Action Type}
    M -->|Navigate| N[Load Page]
    M -->|Submit| O[Process Form]
    M -->|Logout| P[Clear Session]
    N --> L
    O --> Q{Success?}
    Q -->|Yes| R[Show Success]
    Q -->|No| S[Show Error]
    R --> L
    S --> L
    P --> C
```

## Sequence Diagram

```mermaid
sequenceDiagram
    participant U as User
    participant F as Frontend
    participant B as Backend
    participant D as Database

    U->>F: Click button
    F->>B: POST /api/action
    B->>D: Query data
    D-->>B: Results
    B-->>F: JSON response
    F-->>U: Update UI
```

## Tall Sequence Diagram (should show from top)

```mermaid
sequenceDiagram
    participant U as User
    participant C as Client
    participant G as API Gateway
    participant A as Auth Service
    participant P as Product Service
    participant O as Order Service
    participant D as Database
    participant Q as Message Queue
    participant N as Notification Service

    U->>C: Browse products
    C->>G: GET /products
    G->>P: Forward request
    P->>D: Query products
    D-->>P: Product list
    P-->>G: Products JSON
    G-->>C: Response
    C-->>U: Display products

    U->>C: Add to cart
    C->>G: POST /cart
    G->>A: Validate token
    A-->>G: Token valid
    G->>O: Add item
    O->>D: Update cart
    D-->>O: Confirmed
    O-->>G: Cart updated
    G-->>C: Success
    C-->>U: Show cart badge

    U->>C: Checkout
    C->>G: POST /checkout
    G->>A: Validate token
    A-->>G: Token valid
    G->>O: Process order
    O->>D: Create order
    D-->>O: Order ID
    O->>Q: Publish event
    Q->>N: Order created
    N->>U: Email confirmation
    O-->>G: Order confirmed
    G-->>C: Success + Order ID
    C-->>U: Show confirmation
```

## Class Diagram

```mermaid
classDiagram
    class Animal {
        +String name
        +int age
        +makeSound()
    }
    class Dog {
        +fetch()
    }
    class Cat {
        +scratch()
    }
    Animal <|-- Dog
    Animal <|-- Cat
```

## State Diagram

```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> Processing: submit
    Processing --> Success: complete
    Processing --> Error: fail
    Error --> Idle: retry
    Success --> [*]
```

## Gantt Chart (wide)

```mermaid
gantt
    title Project Timeline
    dateFormat YYYY-MM-DD
    section Phase 1
        Research      :a1, 2024-01-01, 30d
        Design        :a2, after a1, 20d
    section Phase 2
        Development   :b1, after a2, 60d
        Testing       :b2, after b1, 30d
    section Phase 3
        Deployment    :c1, after b2, 10d
        Maintenance   :c2, after c1, 90d
```

## ER Diagram

```mermaid
erDiagram
    USER ||--o{ ORDER : places
    ORDER ||--|{ LINE_ITEM : contains
    PRODUCT ||--o{ LINE_ITEM : "appears in"
    USER {
        int id PK
        string email
        string name
    }
    ORDER {
        int id PK
        int user_id FK
        date created_at
    }
    PRODUCT {
        int id PK
        string name
        decimal price
    }
    LINE_ITEM {
        int order_id FK
        int product_id FK
        int quantity
    }
```
