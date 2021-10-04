CFLAGS = -Wall -Werror
SAMPLE_DIR = sample
OUT_DIR = target/release
LIBS = \
	-L$(OUT_DIR) \
	-llibevent
SAMPLES_SRC = \
	$(SAMPLE_DIR)/bench.c \
	$(SAMPLE_DIR)/dns-example.c \
	$(SAMPLE_DIR)/event-read-fifo.c \
	$(SAMPLE_DIR)/hello-world.c \
	$(SAMPLE_DIR)/tokio-time-test.c
SAMPLES_BIN = $(patsubst %.c,%,$(SAMPLES_SRC))

all: $(SAMPLES_BIN)

%: %.c $(OUT_DIR)/liblibevent.a
	@$(CC) $(CFLAGS) -o $@ $(LIBS) $<

$(OUT_DIR)/liblibevent.a: FORCE
	@cargo build --features tracing_subscriber,tokio_backend --release

bench:
	cargo build --features tracing_subscriber,tokio_backend --release
	@$(CC) $(CFLAGS) -o $(OUT_DIR)/bench-kqueue $(LIBS) $(SAMPLE_DIR)/bench.c
	@$(CC) $(CFLAGS) -o $(OUT_DIR)/bench-tokio $(LIBS) $(SAMPLE_DIR)/bench.c -DUSE_TOKIO
	$(OUT_DIR)/bench-kqueue > $(OUT_DIR)/kqueue.csv
	$(OUT_DIR)/bench-tokio > $(OUT_DIR)/tokio.csv
	./plot-bench.py

run-hello-world: $(SAMPLE_DIR)/hello-world
	RUST_BACKTRACE=1 RUST_LOG=debug ./$<

FORCE: ;

clean:
	$(RM) $(SAMPLES_BIN)
