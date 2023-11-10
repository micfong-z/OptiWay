cmake_minimum_required(VERSION 3.16)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED True)

project(Optiway VERSION 0.5.1)

add_library(libfloyd
        include/utilities.hpp)

target_include_directories(libfloyd PRIVATE include)
target_include_directories(libfloyd PRIVATE target/debug/build/optiway-17302af89a67c6de/out/cxxbridge/include/)

target_sources(libfloyd PRIVATE
        src/floyd.cpp.bak
  src/multi-objective-agent.cpp
  src/utilities.cpp
)
