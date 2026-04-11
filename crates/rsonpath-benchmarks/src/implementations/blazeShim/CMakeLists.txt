cmake_minimum_required(VERSION 3.14)
project(blazeShim C CXX)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Blaze must be installed or CMAKE_PREFIX_PATH set to find it
find_package(Blaze REQUIRED COMPONENTS evaluator compiler)

add_library(blazeShim SHARED blaze_bridge.cc)
target_link_libraries(blazeShim PRIVATE sourcemeta::blaze::evaluator sourcemeta::blaze::compiler)