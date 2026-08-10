find_package(PkgConfig REQUIRED)

pkg_check_modules(FFmpeg REQUIRED IMPORTED_TARGET libavformat libavcodec libavutil libswresample)

add_library(
    echo-audio STATIC
    src/adjustment.cpp
    src/decode.cpp
    src/dynamics_processor.cpp
    src/k_weighting_filter.cpp
    src/low_cut_filter.cpp
    src/loudness_gain_advisor.cpp
    src/loudness_meter.cpp
    src/offline_loudness_analyzer.cpp
    src/output_guard.cpp
    src/output_limiter.cpp
    src/playback.cpp
    src/parametric_equalizer.cpp
    src/waveform.cpp
)
add_library(Echo::Audio ALIAS echo-audio)

target_include_directories(echo-audio PUBLIC include)

target_compile_features(echo-audio PUBLIC cxx_std_20)

target_link_libraries(echo-audio PUBLIC PkgConfig::FFmpeg)

function(echo_audio_enable_warnings target)
    if(MSVC)
        target_compile_options(${target} PRIVATE /W4 /permissive-)
    else()
        target_compile_options(
            ${target}
            PRIVATE
                -Wall
                -Wextra
                -Wpedantic
                -Wconversion
                -Wsign-conversion
        )
    endif()
endfunction()

echo_audio_enable_warnings(echo-audio)

if(BUILD_TESTING)
    add_executable(
        echo-adjustment-engine-test
        tests/adjustment_engine_test.cpp
    )
    target_link_libraries(echo-adjustment-engine-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-adjustment-engine-test)
    add_test(NAME echo-adjustment-engine-test COMMAND echo-adjustment-engine-test)

    add_executable(
        echo-dynamics-processor-test
        tests/dynamics_processor_test.cpp
    )
    target_link_libraries(echo-dynamics-processor-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-dynamics-processor-test)
    add_test(NAME echo-dynamics-processor-test COMMAND echo-dynamics-processor-test)

    add_executable(
        echo-low-cut-filter-test
        tests/low_cut_filter_test.cpp
    )
    target_link_libraries(echo-low-cut-filter-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-low-cut-filter-test)
    add_test(NAME echo-low-cut-filter-test COMMAND echo-low-cut-filter-test)

    add_executable(
        echo-loudness-gain-advisor-test
        tests/loudness_gain_advisor_test.cpp
    )
    target_link_libraries(echo-loudness-gain-advisor-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-loudness-gain-advisor-test)
    add_test(
        NAME echo-loudness-gain-advisor-test
        COMMAND echo-loudness-gain-advisor-test
    )

    add_executable(
        echo-loudness-meter-test
        tests/loudness_meter_test.cpp
    )
    target_link_libraries(echo-loudness-meter-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-loudness-meter-test)
    add_test(NAME echo-loudness-meter-test COMMAND echo-loudness-meter-test)

    add_executable(
        echo-offline-loudness-analyzer-test
        tests/offline_loudness_analyzer_test.cpp
    )
    target_link_libraries(echo-offline-loudness-analyzer-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-offline-loudness-analyzer-test)
    add_test(
        NAME echo-offline-loudness-analyzer-test
        COMMAND echo-offline-loudness-analyzer-test
    )

    add_executable(
        echo-output-guard-test
        tests/output_guard_test.cpp
    )
    target_link_libraries(echo-output-guard-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-output-guard-test)
    add_test(NAME echo-output-guard-test COMMAND echo-output-guard-test)

    add_executable(
        echo-output-limiter-test
        tests/output_limiter_test.cpp
    )
    target_link_libraries(echo-output-limiter-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-output-limiter-test)
    add_test(NAME echo-output-limiter-test COMMAND echo-output-limiter-test)

    add_executable(
        echo-parametric-equalizer-test
        tests/parametric_equalizer_test.cpp
    )
    target_link_libraries(echo-parametric-equalizer-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-parametric-equalizer-test)
    add_test(NAME echo-parametric-equalizer-test COMMAND echo-parametric-equalizer-test)

    add_executable(
        echo-waveform-engine-test
        tests/waveform_engine_test.cpp
    )
    target_link_libraries(echo-waveform-engine-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-waveform-engine-test)
    add_test(NAME echo-waveform-engine-test COMMAND echo-waveform-engine-test)

    add_executable(
        echo-playback-engine-test
        tests/playback_engine_test.cpp
    )
    target_link_libraries(echo-playback-engine-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-playback-engine-test)
    add_test(NAME echo-playback-engine-test COMMAND echo-playback-engine-test)
endif()
