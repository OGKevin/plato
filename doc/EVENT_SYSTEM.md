# Event System Architecture

## Overview

Plato's event system is built around a hierarchical tree of views. Events flow through this tree in two directions: from parent to child (capture phase) and from child to parent (bubbling phase). A global hub coordinates event distribution across the entire tree.

## Core Concepts

### The View Tree

Views are organized as a tree structure where:
- Each view can have zero or more children
- Children are ordered by z-level (rendering order)
- The n-th child has a z-level less than or equal to the n+1-th child
- Higher z-level views are "on top" and receive events first

### Communication Channels

**Hub (Global Channel)**
- A sender/receiver pair for broadcasting events
- Events sent to the hub reach the root view
- Root propagates hub events down the tree

**Bus (Local Channel)**
- A queue (`VecDeque<Event>`) for child-to-parent communication
- Each view collects events from its children's buses
- Unhandled events bubble up to the parent's bus

## Event Flow

### Phase 1: Root-to-Leaf Propagation (Capture Phase)

Events from the hub start at the root and travel down to children:

```
Hub → Root View
        ↓
      View A
        ↓
    View B (z-level: 2) ← checked first
        ↓
    View C (z-level: 1) ← checked second if B doesn't capture
```

**Key points:**
- Children are processed in **reverse order** (highest z-level first)
- A child can **capture** the event by returning `true`
- Capturing stops propagation to siblings and lower z-levels
- If not captured, the event continues to the next child

### Phase 2: Child-to-Parent Bubbling

Views can send events to their parent via the bus:

```
    Leaf View
        ↓ (bus.push_back)
      Parent View
        ↓ (if unhandled)
      Grandparent
        ↓ (if unhandled)
      Root
        ↓ (if still unhandled)
      Hub
```

**Key points:**
- Children send events using `bus.push_back(event)`
- Parents can handle or ignore events from children
- Unhandled events continue bubbling up
- Events reaching the root without being handled go to the hub

### Phase 3: The Hub Feedback Loop

When an event reaches the hub from bubbling, it re-enters the system:

```
┌─────────────────────────────────────┐
│  1. Event enters from Hub           │
│     ↓                                │
│  2. Root-to-Leaf propagation        │
│     ↓                                │
│  3. Child sends to bus              │
│     ↓                                │
│  4. Bubbles to root                 │
│     ↓                                │
│  5. Root's bus → Hub                │
│     ↓                                │
│  6. Back to step 1 (next iteration) │
└─────────────────────────────────────┘
```

This creates a feedback loop where unhandled events continuously propagate until a view captures them.

## Event Processing Details

### The handle_event Function

Every view implements this pattern:

```rust
// Pseudocode
fn handle_event(event, hub, bus) -> bool {
    if has_children {
        let child_bus = new_queue()
        
        // Process children in reverse (highest z-level first)
        for child in children.reverse() {
            if child.handle_event(event, hub, child_bus) {
                captured = true
                break  // Stop propagating to other children
            }
        }
        
        // Handle events from children
        for child_event in child_bus {
            if self.handle(child_event) {
                // Event handled, remove from bus
            } else {
                // Bubble to parent
                bus.push_back(child_event)
            }
        }
        
        // Give this view a chance to handle the original event
        captured || self.handle(event, hub, bus)
    } else {
        self.handle(event, hub, bus)
    }
}
```

### Return Values

- `true`: Event was captured/handled, stop propagating to siblings
- `false`: Event not handled, continue to next sibling or bubble up

### Sending Events

**To parent (local communication):**
```rust
bus.push_back(Event::MyAction)
```

**To hub (global broadcast):**
```rust
hub.send(Event::GlobalAction)
```

## Z-Level Ordering Example

Consider this simple binary tree:

```
       Root
      /    \
     A      B (z=2, higher, on top)
    / \    / \
   C   D  E   F (z=1)
```

When an event arrives at Root:

1. **Right branch first** (B has higher z-level than A)
   - B is checked first
   - If B captures, done
   - If not, check B's children (F, then E in reverse)

2. **Left branch second** (only if B didn't capture)
   - A is checked
   - If A captures, done
   - If not, check A's children (D, then C in reverse)

**Complete traversal order for capture:**
```
Root → B → F → E → A → D → C
```

This ensures views "on top" (higher z-level) get first chance to handle events, which matches the visual stacking order.

## Common Event Patterns

### Pattern 1: Button Click

```
User taps button
    ↓
Button.handle_event() receives tap
    ↓
Button captures (returns true) and sends to bus
    bus.push_back(Event::ButtonAction)
    ↓
Parent.handle_event() processes bus
    ↓
Parent performs action or bubbles further
```

### Pattern 2: Global Broadcast

```
View decides to trigger global action
    ↓
View sends to hub directly
    hub.send(Event::GlobalAction)
    ↓
Hub delivers to root
    ↓
Event propagates root-to-leaf
    ↓
All views get a chance to respond
```

### Pattern 3: Unhandled Event Cycle

```
Child generates event → bus → Parent doesn't handle
    ↓
Bubbles to Grandparent → doesn't handle
    ↓
Reaches Root → Root's bus collected
    ↓
Root's bus sent to Hub
    ↓
Hub re-dispatches to Root
    ↓
Root-to-leaf propagation (new attempt)
    ↓
Some view finally captures it
```

### Pattern 4: Event Transformation

```
Menu receives Event::Select
    ↓
Menu transforms it to Event::PropagateSelect
    ↓
Propagates to children explicitly
    ↓
Children handle the transformed event
```

## Main Event Loop

The application's main loop ties everything together:

```rust
// Pseudocode
loop {
    // 1. Receive event from hub
    event = hub.receive()
    
    // 2. Special cases handled directly by app
    if event.is_special() {
        handle_special(event)
        continue
    }
    
    // 3. Propagate through view tree
    root.handle_event(event, hub, bus)
    
    // 4. Collect bubbled events and send to hub
    while let Some(bubbled_event) = bus.pop() {
        hub.send(bubbled_event)
    }
    
    // 5. Process rendering queue
    render()
}
```

## Implementation Guide

### Adding a New View

When implementing a new view:

1. **Implement handle_event:**
   ```rust
   fn handle_event(&mut self, evt: &Event, hub: &Hub, bus: &mut Bus, ...) -> bool {
       match evt {
           Event::Gesture(GestureEvent::Tap(center)) if self.rect.includes(center) => {
               bus.push_back(self.event.clone());
               true  // Captured
           }
           _ => false  // Not handled
       }
   }
   ```

2. **Choose communication method:**
   - Use **bus** for parent-specific actions (e.g., button click)
   - Use **hub** for global actions (e.g., toggle frontlight, close all menus)

3. **Decide on capturing:**
   - Return `true` if the event is relevant to this view and should stop propagating
   - Return `false` if the event should continue to siblings or bubble up

4. **Handle child events:**
   - Process events from child_bus
   - Decide which to handle and which to bubble further
