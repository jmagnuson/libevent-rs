/*
 * XXX This sample code was once meant to show how to use the basic Libevent
 * interfaces, but it never worked on non-Unix platforms, and some of the
 * interfaces have changed since it was first written.  It should probably
 * be removed or replaced with something better.
 *
 * Compile with:
 * cc -I/usr/local/include -o time-test time-test.c -L/usr/local/lib -levent
 */

#include <sys/types.h>

#include <event2/event-config.h>

#include <sys/stat.h>
#include <sys/queue.h>
#include <unistd.h>
#include <time.h>
#ifdef EVENT__HAVE_SYS_TIME_H
#include <sys/time.h>
#endif
#include <fcntl.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <errno.h>

#include <event2/event.h>
#include <event2/event_struct.h>
#include <event2/util.h>

#include "tokio_event_base.h"

struct timeval lasttime;
struct timeval lasttime_sigalrm;

int event_is_persistent;

struct event sigalrm_event;
struct event short_timeout_event;

static double
get_elapsed(struct timeval *lasttime)
{
	struct timeval newtime, difference;
	double elapsed;

	evutil_gettimeofday(&newtime, NULL);
	evutil_timersub(&newtime, lasttime, &difference);
	elapsed = difference.tv_sec +
	    (difference.tv_usec / 1.0e6);
	*lasttime = newtime;

	return elapsed;
}

static void
long_timeout_cb(evutil_socket_t fd, short event, void *arg)
{
	struct timeval newtime, difference;
	struct event *timeout = arg;
	double elapsed;

	evutil_gettimeofday(&newtime, NULL);
	evutil_timersub(&newtime, &lasttime, &difference);
	elapsed = difference.tv_sec +
	    (difference.tv_usec / 1.0e6);

	printf("long_timeout_cb called at %d: %.3f seconds elapsed.\n",
	    (int)newtime.tv_sec, elapsed);
	lasttime = newtime;

	if (! event_is_persistent) {
		struct timeval tv;
		evutil_timerclear(&tv);
		tv.tv_sec = 10;
		event_add(timeout, &tv);
	}
}

static void
short_timeout_cb(evutil_socket_t fd, short event, void *arg)
{
	double elapsed;

	elapsed = get_elapsed(&lasttime_sigalrm);
	printf("short_timeout_cb called at %d: %.3f seconds elapsed.\n",
		   (int)lasttime_sigalrm.tv_sec,
		   elapsed);

	event_add(&sigalrm_event, NULL);
	alarm(1);
}

static void
sigalrm_cb(evutil_socket_t nsignal, short event, void *arg)
{
	struct timeval tv;

	evutil_gettimeofday(&lasttime_sigalrm, NULL);
	printf("siglarm_cb called at %d\n", (int)lasttime_sigalrm.tv_sec);

	evutil_timerclear(&tv);
	event_add(&short_timeout_event, &tv);
}

int
main(int argc, char **argv)
{
	struct event long_timeout_event;
	struct timeval tv;
	struct event_base *base;
	int flags;

	if (argc == 2 && !strcmp(argv[1], "-p")) {
		event_is_persistent = 1;
		flags = EV_PERSIST;
	} else {
		event_is_persistent = 0;
		flags = 0;
	}

	/* Initalize the event library */
	base = tokio_event_base_new();

	/* Initalize one event */
	event_assign(&long_timeout_event, base, -1, flags, long_timeout_cb, &long_timeout_event);
	event_assign(&short_timeout_event, base, -1, flags, short_timeout_cb, NULL);
	event_assign(&sigalrm_event, base, SIGALRM, EV_SIGNAL, sigalrm_cb, NULL);

	evutil_timerclear(&tv);
	tv.tv_sec = 10;
	event_add(&long_timeout_event, &tv);
	event_add(&sigalrm_event, NULL);

	evutil_gettimeofday(&lasttime, NULL);
	alarm(1);

	setbuf(stdout, NULL);
	setbuf(stderr, NULL);

	event_base_dispatch(base);

	return (0);
}

