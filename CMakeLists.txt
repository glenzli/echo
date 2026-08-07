cmake_minimum_required(VERSION 3.28)

project(
    Echo
    VERSION 0.1.0
    DESCRIPTION "Local-first source-anchored sound memory system"
    LANGUAGES CXX
)

option(ECHO_BUILD_DESKTOP "Build the Qt Quick desktop application" OFF)

if(APPLE)
    # The desktop toolbar fuses with the native title bar, so Objective-C++
    # must be available at the project boundary.
    enable_language(OBJCXX)
endif()

include(CTest)

add_subdirectory(cpp/echo-audio)

if(ECHO_BUILD_DESKTOP)
    add_subdirectory(apps/desktop)
endif()
