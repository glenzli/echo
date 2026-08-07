# Echo desktop application assembly: the Qt Quick shell over the Rust
# memory engine and C++ audio engine.

cmake_minimum_required(VERSION 3.28)

find_package(Qt6 6.5 REQUIRED COMPONENTS Quick Multimedia)
find_package(PkgConfig REQUIRED)
find_package(Iconv REQUIRED)

# The Rust desktop bridge is built by Cargo with its own target directory so
# the generated CXX ABI header can be synced into a stable include root.
find_program(ECHO_CARGO_EXECUTABLE cargo REQUIRED)
if(CMAKE_BUILD_TYPE STREQUAL "Release")
    set(ECHO_CARGO_PROFILE release)
    set(ECHO_CARGO_PROFILE_ARGUMENT --release)
else()
    set(ECHO_CARGO_PROFILE debug)
    set(ECHO_CARGO_PROFILE_ARGUMENT)
endif()

set(
    ECHO_CARGO_TARGET_DIRECTORY
    "${CMAKE_CURRENT_BINARY_DIR}/cargo-target"
)

file(
    GLOB_RECURSE ECHO_RUST_BRIDGE_INPUTS
    CONFIGURE_DEPENDS
    "${PROJECT_SOURCE_DIR}/Cargo.toml"
    "${PROJECT_SOURCE_DIR}/Cargo.lock"
    "${PROJECT_SOURCE_DIR}/crates/*/Cargo.toml"
    "${PROJECT_SOURCE_DIR}/crates/*/build.rs"
    "${PROJECT_SOURCE_DIR}/crates/*/src/*.rs"
    "${PROJECT_SOURCE_DIR}/crates/*/src/**/*.rs"
    "${PROJECT_SOURCE_DIR}/cpp/echo-audio/include/*.hpp"
    "${PROJECT_SOURCE_DIR}/cpp/echo-audio/src/*.cpp"
    "${PROJECT_SOURCE_DIR}/cpp/echo-audio/src/bridge/*.cpp"
    "${PROJECT_SOURCE_DIR}/cpp/echo-audio/src/bridge/*.hpp"
)

set(
    ECHO_DESKTOP_RUST_LIBRARY
    "${ECHO_CARGO_TARGET_DIRECTORY}/${ECHO_CARGO_PROFILE}/libecho_desktop_bridge.a"
)
set(
    ECHO_CXXBRIDGE_INCLUDE_DIRECTORY
    "${ECHO_CARGO_TARGET_DIRECTORY}/cxxbridge"
)

if(UNIX)
    # `cmake -E env` closes GNU Make's jobserver descriptors before launching
    # Cargo. Use the system `env` executable on Unix so JOB_SERVER_AWARE can
    # preserve bounded parallelism.
    find_program(ECHO_ENV_EXECUTABLE env REQUIRED)
    set(ECHO_CARGO_ENV "${ECHO_ENV_EXECUTABLE}")
else()
    set(ECHO_CARGO_ENV "${CMAKE_COMMAND}" -E env)
endif()
if(APPLE)
    # Homebrew's Qt 6.11 frameworks currently target macOS 26.0. Match the
    # Rust static library so the final app does not raise mixed-minimum
    # warnings.
    list(APPEND ECHO_CARGO_ENV MACOSX_DEPLOYMENT_TARGET=26.0)
endif()

add_custom_command(
    OUTPUT "${ECHO_DESKTOP_RUST_LIBRARY}"
    BYPRODUCTS
        "${ECHO_CXXBRIDGE_INCLUDE_DIRECTORY}/echo-desktop-bridge/src/lib.rs.h"
    COMMAND
        ${ECHO_CARGO_ENV} "${ECHO_CARGO_EXECUTABLE}" build
        --package echo-desktop-bridge
        ${ECHO_CARGO_PROFILE_ARGUMENT}
        --target-dir "${ECHO_CARGO_TARGET_DIRECTORY}"
    COMMAND
        "${CMAKE_COMMAND}"
        "-DECHO_CARGO_TARGET_DIRECTORY=${ECHO_CARGO_TARGET_DIRECTORY}"
        "-DECHO_CARGO_PROFILE=${ECHO_CARGO_PROFILE}"
        "-DECHO_CXXBRIDGE_INCLUDE_DIRECTORY=${ECHO_CXXBRIDGE_INCLUDE_DIRECTORY}"
        -P "${CMAKE_CURRENT_SOURCE_DIR}/cmake/sync_cxxbridge_headers.cmake"
    DEPENDS ${ECHO_RUST_BRIDGE_INPUTS}
    WORKING_DIRECTORY "${PROJECT_SOURCE_DIR}"
    COMMENT "Building the Rust desktop bridge"
    JOB_SERVER_AWARE TRUE
    VERBATIM
)
add_custom_target(echo-desktop-rust-build DEPENDS "${ECHO_DESKTOP_RUST_LIBRARY}")
add_library(echo-desktop-rust STATIC IMPORTED GLOBAL)
set_target_properties(
    echo-desktop-rust
    PROPERTIES IMPORTED_LOCATION "${ECHO_DESKTOP_RUST_LIBRARY}"
)
add_dependencies(echo-desktop-rust echo-desktop-rust-build)

# The audio engine's dynamic dependencies arrive through Cargo's static
# archive; CMake owns the FFmpeg link for the final executable.
pkg_check_modules(FFmpeg REQUIRED IMPORTED_TARGET libavformat libavcodec libavutil libswresample)

set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)

qt_add_executable(
    echo-desktop
    src/main.cpp
    src/desktop_backend.cpp
    src/desktop_backend.hpp
    src/playback_controller.cpp
    src/playback_controller.hpp
    src/ui_preferences.cpp
    src/ui_preferences.hpp
    src/model_preferences.cpp
    src/model_preferences.hpp
    src/mac_titlebar.hpp
)
if(APPLE)
    target_sources(echo-desktop PRIVATE src/mac_titlebar.mm)
endif()
set_source_files_properties(qml/Theme.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)
qt_add_qml_module(
    echo-desktop
    URI EchoDesktop
    VERSION 0.1
    QML_FILES
        qml/Main.qml
        qml/EchoButton.qml
        qml/EchoIcon.qml
        qml/EchoIconButton.qml
        qml/EchoSettingsDialog.qml
        qml/Theme.qml
        qml/WaveformView.qml
    RESOURCES
        icons/play.svg
        icons/pause.svg
        icons/stop.svg
        icons/refresh.svg
        icons/tune.svg
        icons/waveform.svg
        icons/mic.svg
)

target_compile_features(echo-desktop PUBLIC cxx_std_20)
target_include_directories(
    echo-desktop
    PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src"
    PRIVATE
        "${ECHO_CXXBRIDGE_INCLUDE_DIRECTORY}"
        "${PROJECT_SOURCE_DIR}/cpp/echo-audio/include"
)
target_link_libraries(
    echo-desktop
    PRIVATE
        Qt6::Quick
        Qt6::Multimedia
        echo-desktop-rust
        PkgConfig::FFmpeg
        ${CMAKE_DL_LIBS}
        Iconv::Iconv
)

add_dependencies(echo-desktop echo-desktop-rust-build)

if(MSVC)
    target_compile_options(echo-desktop PRIVATE /W4 /permissive-)
else()
    target_compile_options(
        echo-desktop
        PRIVATE
            -Wall
            -Wextra
            -Wpedantic
    )
endif()
