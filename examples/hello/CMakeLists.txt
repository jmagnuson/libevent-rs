CMAKE_MINIMUM_REQUIRED(VERSION 2.8)
project(helloc)

# copy-pasta from https://docs.conan.io/en/latest/howtos/cmake_launch.html
#if(NOT EXISTS "${CMAKE_BINARY_DIR}/conan.cmake")
#    message(STATUS "Downloading conan.cmake from https://github.com/conan-io/cmake-conan")
#    file(DOWNLOAD "https://raw.githubusercontent.com/conan-io/cmake-conan/master/conan.cmake"
#            "${CMAKE_BINARY_DIR}/conan.cmake")
#endif()

#include(${CMAKE_BINARY_DIR}/conan.cmake)

#conan_cmake_run(CONANFILE conanfile.txt
#        BASIC_SETUP CMAKE_TARGETS
#        BUILD missing)

#include_directories(${CONAN_INCLUDE_DIRS})
include_directories(${LIBEVENT_INCLUDE_PATH})
include_directories(./src/ffi)

add_library(helloc STATIC src/ffi/helloc.c)

#target_link_libraries(helloc ${CONAN_LIBS})

install(TARGETS helloc DESTINATION .)
