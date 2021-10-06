CARGO_FLAGS = --release --features tokio_backend
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
	$(CC) $(CFLAGS) -o $@ $(LIBS) $<

$(OUT_DIR)/liblibevent.a: FORCE
	cargo build $(CARGO_FLAGS)

$(OUT_DIR)/bench-kqueue: $(SAMPLE_DIR)/bench.c $(OUT_DIR)/liblibevent.a
	$(CC) $(CFLAGS) -o $(OUT_DIR)/bench-kqueue $(LIBS) $(SAMPLE_DIR)/bench.c

$(OUT_DIR)/bench-tokio: $(SAMPLE_DIR)/bench.c $(OUT_DIR)/liblibevent.a
	$(CC) $(CFLAGS) -o $(OUT_DIR)/bench-tokio $(LIBS) $(SAMPLE_DIR)/bench.c -DUSE_TOKIO

plot-bench: $(OUT_DIR)/bench-kqueue $(OUT_DIR)/bench-tokio
	$(OUT_DIR)/bench-kqueue > $(OUT_DIR)/kqueue.csv
	$(OUT_DIR)/bench-tokio > $(OUT_DIR)/tokio.csv
	./plot-bench.py

prof-bench: $(OUT_DIR)/bench-kqueue $(OUT_DIR)/bench-tokio
	sudo flamegraph -o kqueue-flamegraph.svg target/release/bench-kqueue
	sudo flamegraph -o tokio-flamegraph.svg target/release/bench-tokio

clippy: FORCE
	cargo clippy $(CARGO_FLAGS)

run-hello-world: $(SAMPLE_DIR)/hello-world
	RUST_BACKTRACE=1 RUST_LOG=debug ./$<

FORCE: ;

clean:
	$(RM) $(SAMPLES_BIN)
