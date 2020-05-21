# Matches libevent's version requirements
cmake_minimum_required(VERSION 3.1)

project(helloc)

# libevent headers imported via `cmake::Config::cflags("-I...")`

include_directories(./src/ffi)

add_library(helloc STATIC src/ffi/helloc.c)

install(TARGETS helloc DESTINATION .)
