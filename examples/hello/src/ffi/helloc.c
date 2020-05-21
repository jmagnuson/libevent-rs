#include "helloc.h"

static void timer_cb_temporary(evutil_socket_t fd, short event, void *ptr)
{
  struct event* ev = (struct event*)ptr;
  static size_t counter = 0;

  printf("hi from temporary callback\n");
  if (++counter > 30)
  {
    event_del(ev);
    event_free(ev);
  }
}

static void timer_cb_forever(evutil_socket_t fd, short event, void *ptr)
{
  struct event* ev = (struct event*)ptr;

  printf("hi from forever callback\n");
}

int helloc_init(struct event_base *base)
{
  if (base == NULL)
  {
    return -1;
  }

  struct timeval one_sec = { 1, 0 };
  struct timeval hundred_ms = { 0, 100*1000 };
  struct event *ev, *ev2;
  ev = event_new(base, -1, EV_PERSIST, timer_cb_forever, event_self_cbarg());
  event_add(ev, &one_sec);
  ev2 = event_new(base, -1, EV_PERSIST, timer_cb_temporary, event_self_cbarg());
  event_add(ev2, &hundred_ms);

  return 0;
}

int helloc_destroy(struct event_base* base)
{
  if (base == NULL)
  {
    return -1;
  }

  event_base_free(base);

  return 0;
}

