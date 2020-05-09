CMAKE_MINIMUM_REQUIRED(VERSION 2.8)
project(helloc)

# libevent headers imported via `cmake::Config::cflags("-I...")`

include_directories(./src/ffi)

add_library(helloc STATIC src/ffi/helloc.c)

install(TARGETS helloc DESTINATION .)
