#pragma once

/*
 * Creates a new event_base and injects a single-threaded tokio runtime into
 * it as the backend.
 */
extern struct event_base* tokio_event_base_new(void);
