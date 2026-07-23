cmake_minimum_required(VERSION 3.25)

project(cmajor-rs)

file(
  DOWNLOAD
  https://github.com/cpm-cmake/CPM.cmake/releases/download/v0.38.7/CPM.cmake
  ${CMAKE_CURRENT_BINARY_DIR}/cmake/CPM.cmake
  EXPECTED_HASH
    SHA256=83e5eb71b2bbb8b1f2ad38f1950287a057624e385c238f6087f94cdfc44af9c5)
include(${CMAKE_CURRENT_BINARY_DIR}/cmake/CPM.cmake)

set(FETCHCONTENT_QUIET FALSE)

set(CMAJOR_VERSION 1.0.2562)

cpmaddpackage(
  NAME
  cmajor
  GITHUB_REPOSITORY
  cmajor-lang/cmajor
  GIT_TAG
  ${CMAJOR_VERSION}
  GIT_SHALLOW
  TRUE
  GIT_PROGRESS
  TRUE
  GIT_CONFIG
  fetch.parallel=0)

make_cmaj_library(LIBRARY_NAME cmajor INCLUDE_PLAYBACK ENABLE_PERFORMER_LLVM)

add_library(cmajor-static STATIC main.cpp)
target_link_libraries(cmajor-static PUBLIC cmajor)
target_compile_features(cmajor-static PUBLIC cxx_std_17)
target_include_directories(cmajor-static PUBLIC ${cmajor_SOURCE_DIR}/include)
