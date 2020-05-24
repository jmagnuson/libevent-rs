# Designs for Event Handling

Libevent, enabled by C, is extremely flexible in how it allows the developer to
create and interact with events. Thus when wrapping in Rust, special
consideration must be given to how and what gets exposed in the API. A balance
emerges with flexibility, ergonomics, and safety all somewhat at odds with one
another.

## Event setup

One specific and perhaps the most important case is event handling. Typically,
events in libevent are first allocated and initialized using every necessary
parameter except timeout, and then activated on a second call, with timeout as
an optional parameter. Setup of an 1 second timer is shown below:

```c
struct timeval one_sec = { 1, 0 };
struct event *ev;
ev = event_new(base, -1, EV_PERSIST, _timer_cb, event_self_cbarg());
event_add(ev, &one_sec);
```

### Aliased Self 

Firstly, the `event_self_cbarg` is notable because it sets the callback context
to a pointer to the event object itself. But as shown, the caller only receives
the event pointer _after_ `event_new` is called, so the function is necessary
to employ some trickery behind the scenes to make this catch-22 possible. And
having a self-referential handle in the callback context is extremely useful
and widely-used, so that ability is a requirement in any higher-level bindings.

What this also means in Rust terms is that libevent clones the same pointer to
two separate places; one is encapsulated within the callback scope, and one is
returned to the caller to do with as they please. As this is C, memory
management is entirely incumbent on the developer and while the concern isn't
necessarily that the pointer itself is copied, there definitely exists the
potential for racy behavior between two owners that can independently control
the same event.

#### All the Arcs

How might Rust handle this? Wrapping both handles in an `Arc<Mutex>` is
certainly a valid and "safe" option, regardless of the desire for `Sync` and
`Send`. But it comes at the cost of ergonomics to the user, who now has the
added responsibility of handling lock contention in two places. There are also
implications as to how stopping and de-initializing would have to be handled.

A better alternative would be to wrap the returned version in `Arc`, and `Weak`
for the copy passed into the closure. This somewhat addresses the question of
de-initialization if `Drop` is used to stop and/or free the event, since the
idle closure would not count toward ownership. But of course the ergonomics
issues still remains; the user is now forced within the closure to
`upgrade().unwrap().lock().unwrap()` even if they can guarantee that no other
part of the program is accessing the event handle. 

#### Forever alone

Another option is to simply not return anything on `event_new`, isolating the
event handle within the callback function. The tradeoff here is of course
flexibility for safety, and if the event never triggers it might as well be
considered leaked memory, since there is no other \[reasonable\] way to access
the pointer for de-initialization.

#### DIY

The previous two options also have the added benefit of allowing event creation
and activation happen in one call. Libevent likely separates the two for better
dynamic control over the timeout, but that is an edge-case that could be
internalized within a higher-level binding so as not to require two calls for
every new event. Maintaining this ergonomic compromise, however, would allow
for shouldering the burden of event handle sharing onto the developer.

Unlike C, Rust has the concept of closures, which can effortlessly capture
variables into its own "context" without explicitly setting them (as in the
case of `event_self_cbarg`). These would naturally take the place of the
callback function, and if assigned _after_ the event memory was allocated, then
the possibility arises where a developer can decide whether or not they want
shared access to the event, both inside and outside of the callback. Consider:

```rust
fn main() {
// Base type is the event loop handle and spawner of new events.
let mut base = ...;

// Belongs to the outside world.
let event = Arc::new(Mutex::new(base.event_new(params, other, than, closure)));

// Will get moved into the closure.
let closure_event = event.clone();

{
    // unlock long enough to spawn on base.
    let mut unlocked_event = event.lock().unwrap();
    base.spawn(&mut unlocked_event, move |fd, flags| {
        if fd.read().is_err() {
            closure_event.lock().unwrap().do_something();
        }           
    })
}
```

For the purposes of this exercise, the above closure can be considered to be
`!Sync`, meaning that the `Arc<Mutex>` could be trivially swapped out for
`Rc<Refcell>` with no change in the API. But once again, the conundrum of de-
initialization comes into play. If `Drop` were used to stop and free the event,
how would this work if there is more than one copy of the `Arc`? Okay, a minor
foot-gun that encountered, can be replaced by a `Weak`.

The closure that gets passed into `spawn` could
for instance, be stored in the event context itself, and thus make it  

#### Not-as DIY

Creating an event before actually spawning it is a reasonable pattern, which
resembles libevent's API as well as other asynchronous libraries like tokio.
But the expectation that developers must choose their sync guarantees without
the API guiding them is a bit off-putting. Instead, consider:

```rust
fn main() {
// Base type is the event loop handle and spawner of new events.
let mut base = ...;

// Belongs to the outside world.
let event = base.event_new(params, other, than, closure);

let arc_event = base.spawn_sync(event, move |arc_event, fd, flags| {
    if fd.read().is_err() {
        closure_event.lock().unwrap().do_something();
    }           
});
```

Where the sync type is built into the API via `spawn_async`, one of a set of
`spawn*` variants (`spawn -> Event`, `spawn_shared -> Rc<Event>`, etc.); And
since sync type is decoupled from actual `Event` creation, is feasible to also
divorce its creation from the actual base:

```rust
fn main() {
    ...
    let event = Event::new(params, other, than closure);

    let arc_event = base.spawn_sync(event, move |arc_event, fd, flags| { ... });
}
```

One oddity here is that `spawn_sync` takes ownership of event and... gives it
back, just wrapped in the appropriate sync. But the un-spawned event type may
need to be different from the spawned-type anyway, so this is probably a
reasonable contract (similar to builder-pattern).