# Steering vs Selection: A Philosophical Exploration

## The Distinction

**Selection** implies:
- Options exist, pre-formed
- Human picks from a menu
- Clerical, passive
- "Which of these?"

**Steering** implies:
- Direction exists, but destination is emergent
- Human shapes trajectory
- Generative, active
- "More like this, less like that"

The user's original framing was "choose one option in one section" - that sounds like selection. But your reframe suggests we should think about it differently.

---

## What Does "Steering" Mean in This Context?

If we think about a document with multiple possible directions, steering isn't:
- "Give me Version A of the architecture"

Steering is:
- "The architecture is going in the wrong direction - it's too monolithic. I want more separation of concerns."
- "This is close, but the authentication approach feels heavy. Lighter touch here."
- "Yes to the data model direction, but with more emphasis on query patterns."

Steering provides **vectors**, not **selections**.

---

## The Gradient Model

Think of AI-generated alternatives not as discrete options but as points in a space:

```
                    More Distributed
                          │
                          │    Version B
                          │       ◆
                          │
  Simple ─────────────────┼─────────────────── Complex
                          │
              Version A   │
                 ◆        │
                          │
                    More Monolithic
```

Steering says: "Move toward (More Distributed, Medium Complexity)"

The AI doesn't need pre-generated options at those coordinates - it can generate something NEW at the steered position.

---

## Implications for annot

### What annot already does well

Annotations provide located, reasoned feedback. That IS steering:
- "This function is too complex" = steer toward simplicity
- "[# SECURITY] Review this for injection" = steer toward security concerns
- "The tone here is too formal" = steer toward casual

The human doesn't pick from tone options - they indicate a direction.

### Where this breaks down

The "multi-version" request assumes:
1. AI generates multiple complete alternatives upfront
2. Human picks one

But that's wasteful and constraining. Why generate 3 full architectures if the human only wants to steer one aspect?

### A different framing

What if instead of "pick A or B", the interaction is:

1. AI presents a plan/document
2. Human steers: "Make the auth section more OAuth2-like"
3. AI regenerates that section
4. Human steers: "Good, but now the data model feels inconsistent"
5. AI adjusts
6. ...until convergence

This is **iterative steering**, not **upfront selection**.

---

## The Forking Model

Perhaps "multiple versions" isn't about choice but about **exploring the space**:

1. AI presents a document
2. Human says: "Fork this into two directions - one more monolithic, one more distributed"
3. AI generates both forks
4. Human reviews both, annotates differences
5. Human steers: "Take the distributed fork but soften the boundary between services X and Y"
6. AI generates convergent version

The forks aren't options to pick from - they're **probes into the possibility space** that help human understand tradeoffs.

---

## What annot could become

If steering (not selection) is the primitive:

### Directional Tags

Instead of `[# PICK]`, imagine:
- `[# MORE]` - amplify this quality
- `[# LESS]` - reduce this aspect
- `[# LIKE file.rs#L42-58]` - make it more like this reference
- `[# CONTRAST]` - want the opposite approach here

### Comparison Mode (not Selection Mode)

Instead of "which version?", the UI shows:
- Here's what we have
- Here's an alternative direction
- Annotate what you prefer about each
- AI synthesizes based on steering signals

### The Output Changes

Instead of:
```
SELECTIONS:
  architecture: Version B
```

The output is:
```
STEERING:
  architecture (lines 10-40):
    > [# MORE] distributed, event-driven
    > [# LESS] shared database coupling
    > [# LIKE infra/kafka.rs#L20-35] event bus pattern
  
  data_model (lines 45-80):
    > [# KEEP] current approach is good
```

---

## Questions This Raises

1. **Is "show me alternatives" ever the right move?**
   - Sometimes you want to see options to understand the space
   - But the goal isn't picking - it's understanding enough to steer

2. **How explicit should steering be?**
   - Directional tags (MORE, LESS, LIKE)
   - Natural language ("more distributed")
   - Comparative annotation on alternatives

3. **Does this require iterative conversation?**
   - If steering generates new content, annot becomes conversational
   - That changes its identity from "review tool" to "co-authoring substrate"

4. **What's the minimal change that enables steering vocabulary?**
   - Maybe just new tags: MORE, LESS, LIKE, CONTRAST
   - The semantics are baked into the tag definitions

---

## A Concrete Scenario

**User's request**: "AI proposes different versions for each section, users choose"

**Reframed as steering**:

1. AI generates document with inline alternatives (not full versions):
   ```markdown
   ## Authentication
   
   We'll use JWT tokens for authentication.
   
   <!-- alternative: OAuth2 delegation to identity provider -->
   <!-- alternative: Session-based with Redis store -->
   ```

2. User annotates with steering:
   ```
   Line 3: [# PREFER] JWT for simplicity
   Line 5: [# CONSIDER] for enterprise customers
   ```

3. Output to AI:
   ```
   STEERING:
     Prefer JWT-based auth for main flow
     Consider OAuth2 as future option for enterprise
   ```

4. AI synthesizes: Generates JWT implementation with hooks for future OAuth2 migration

The user didn't "select Version A" - they steered toward a position that might not have existed in any pre-generated option.

---

## Where This Leaves Us

The design question shifts from:
- "How do we present options and capture selection?"

To:
- "How do we help humans articulate direction, and what vocabulary/tools support that?"

This feels more aligned with annot's generative principle - humans add information (direction, reasoning) rather than just selecting from pre-formed options.

---

## Discussion

Does this reframing resonate? Or is there a case for pure selection that I'm missing?

What does "steering" look like concretely in your workflows?