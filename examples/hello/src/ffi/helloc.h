#ifndef HELLOC_H
#define HELLOC_H

#include <event2/event.h>

int helloc_init(struct event_base* base);
int base_fd(const struct event_base* base);
int helloc_destroy(struct event_base* base);

#endif //HELLOC_H
