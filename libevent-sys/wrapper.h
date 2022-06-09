#include <event.h>

#include <event2/event.h>       // The primary libevent header
#include <event2/thread.h>      // Functions for use by multithreaded programs
#include <event2/buffer.h>      // and event2/bufferevent.h Buffer management for network reading and writing
#include <event2/bufferevent.h> // and event2/bufferevent.h Buffer management for network reading and writing
#include <event2/util.h>        // Utility functions for portable nonblocking network code
#include <event2/dns.h>         // Asynchronous DNS resolution
#include <event2/http.h>        // An embedded libevent-based HTTP server
#include <event2/rpc.h>         // A framework for creating RPC servers and clients


#ifdef EVENT__HAVE_OPENSSL
#include <event2/bufferevent_ssl.h>
#endif

#ifdef EVENT__HAVE_PTHREADS
#include <event2/thread.h>
#endif
