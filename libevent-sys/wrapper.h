#include <event.h>

#ifdef EVENT__HAVE_OPENSSL
#include <event2/bufferevent_ssl.h>
#endif

#ifdef EVENT__HAVE_PTHREADS
#include <event2/thread.h>
#endif
