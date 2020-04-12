
#include <event2/event_struct.h>

#include "helloc.h"

static size_t counter = 0;

static void timer_cb(evutil_socket_t fd, short event, void *ptr)
{
  struct event* ev = (struct event*)ptr;

  printf("hi from callback\n");
  if (++counter > 30)
  {
    event_del(ev);
  }
}

static void timer_cb_forever(evutil_socket_t fd, short event, void *ptr)
{
  struct event* ev = (struct event*)ptr;

  printf("hi from forever callback\n");
}

static void break_loop_cb(evutil_socket_t fd, short event, void *ptr)
{
  struct event* ev = (struct event*)ptr;

  printf("breaking event loop\n");

  event_base_loopbreak((struct event_base*)(ev->ev_base));
}

int helloc_init(struct event_base *base)
{
  if (base != NULL)
  {
    printf("base ain't null after init\n");
  }

  struct timeval one_sec = { 1, 0 };
  struct timeval hundred_ms = { 0, 100*1000 };
  struct event *ev, *ev2;
  ev = event_new(base, -1, EV_PERSIST, timer_cb_forever, event_self_cbarg());
  event_add(ev, &one_sec);
  ev2 = event_new(base, -1, EV_PERSIST, timer_cb, event_self_cbarg());
  event_add(ev2, &hundred_ms);

  return 0;
}

int helloc_destroy(struct event_base* base)
{
  event_base_free(base);

  if (base == NULL)
  {
    printf("base became null\n");
  }

  return 0;
}

