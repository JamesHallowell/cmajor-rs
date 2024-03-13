cmake_minimum_required(VERSION 3.25)

project(cmajor-static)

file(
  DOWNLOAD
  https://github.com/cpm-cmake/CPM.cmake/releases/download/v0.38.7/CPM.cmake
  ${CMAKE_CURRENT_BINARY_DIR}/cmake/CPM.cmake
  EXPECTED_HASH
    SHA256=83e5eb71b2bbb8b1f2ad38f1950287a057624e385c238f6087f94cdfc44af9c5)
include(${CMAKE_CURRENT_BINARY_DIR}/cmake/CPM.cmake)

set(FETCHCONTENT_QUIET FALSE)

cpmaddpackage(
  NAME
  cmajor
  GITHUB_REPOSITORY
  cmajor-lang/cmajor
  GIT_TAG
  1.0.2375
  GIT_SHALLOW
  TRUE
  GIT_PROGRESS
  TRUE
  GIT_CONFIG
  fetch.parallel=0)

set(CMAJ_VERSION="1.0")

add_compile_definitions(
  $<$<CONFIG:Debug>:DEBUG=1> CMAJOR_DLL=0 CMAJ_ENABLE_WEBVIEW_DEV_TOOLS=1
  CHOC_ASSERT=must_include_assertion_header_before_any_choc_headers)

make_cmaj_library(
  LIBRARY_NAME
  cmajor-for-rust
  INCLUDE_PLAYBACK
  ENABLE_PERFORMER_LLVM
  ENABLE_PERFORMER_WEBVIEW
  ENABLE_PERFORMER_CPP
  ENABLE_CODEGEN_LLVM_WASM
  ENABLE_CODEGEN_BINARYEN
  ENABLE_CODEGEN_CPP)

add_library(
  cmajor-static STATIC
  ${cmajor_SOURCE_DIR}/tools/CmajDLL/Source/cmaj_ExportedFunctions.cpp)
target_compile_definitions(cmajor-static PUBLIC CMAJ_VERSION="1.0")
target_link_libraries(cmajor-static PUBLIC cmajor-for-rust)
target_compile_features(cmajor-static PUBLIC cxx_std_17)
target_include_directories(cmajor-static PUBLIC ${cmajor_SOURCE_DIR}/include)
