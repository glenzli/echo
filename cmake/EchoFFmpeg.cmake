# FFmpeg is shared by the native engine and desktop link. Refresh pkg-config's
# derived cache on configure: package-manager upgrades can leave old Cellar
# libraries present even though their embedded install names no longer resolve.
include_guard(GLOBAL)
find_package(PkgConfig REQUIRED)
get_cmake_property(echo_dependency_cache CACHE_VARIABLES)
foreach(echo_dependency_entry IN LISTS echo_dependency_cache)
    if(echo_dependency_entry MATCHES "^(FFmpeg_|pkgcfg_lib_FFmpeg_|__pkg_config_.*FFmpeg)")
        unset("${echo_dependency_entry}" CACHE)
    endif()
endforeach()
pkg_check_modules(FFmpeg REQUIRED IMPORTED_TARGET GLOBAL libavformat libavcodec libavutil libswresample)
