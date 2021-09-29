CFLAGS = -Wall -Werror
SAMPLE_DIR = sample
OUT_DIR = target/debug
LIBS = \
	-L$(OUT_DIR) \
	-llibevent
SAMPLES_SRC = \
	$(SAMPLE_DIR)/dns-example.c \
	$(SAMPLE_DIR)/event-read-fifo.c \
	$(SAMPLE_DIR)/hello-world.c \
	$(SAMPLE_DIR)/time-test.c
SAMPLES_BIN = $(patsubst %.c,%,$(SAMPLES_SRC))

all: $(SAMPLES_BIN)

%: %.c $(OUT_DIR)/liblibevent.a
	$(CC) $(CFLAGS) -o $@ $(LIBS) $<

$(OUT_DIR)/liblibevent.a: FORCE
	RUSTFLAGS="--cfg tokio_unstable" cargo build --features tracing_subscriber,tokio_backend

run-hello-world: $(SAMPLE_DIR)/hello-world
	RUST_BACKTRACE=1 RUST_LOG=debug ./$<

FORCE: ;

clean:
	$(RM) $(SAMPLES_BIN)
