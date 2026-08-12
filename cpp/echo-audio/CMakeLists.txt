find_package(PkgConfig REQUIRED)

pkg_check_modules(FFmpeg REQUIRED IMPORTED_TARGET libavformat libavcodec libavutil libswresample)

add_library(
    echo-audio STATIC
    src/adaptive_noise_reducer.cpp
    src/analysis_proxy.cpp
    src/algorithmic_reverb.cpp
    src/adjustment.cpp
    src/channel_repair_processor.cpp
    src/decode.cpp
    src/de_click_processor.cpp
    src/de_esser.cpp
    src/de_hum_filter.cpp
    src/de_plosive_processor.cpp
    src/delay_vfx_processor.cpp
    src/digital_degrade_vfx_processor.cpp
    src/drive_vfx_processor.cpp
    src/diffuse_space_reverb.cpp
    src/dynamics_processor.cpp
    src/effect_processing_chain.cpp
    src/effect_mask_plan.cpp
    src/k_weighting_filter.cpp
    src/low_cut_filter.cpp
    src/loudness_gain_advisor.cpp
    src/loudness_meter.cpp
    src/modulated_delay_vfx.cpp
    src/modulation_vfx_processor.cpp
    src/offline_loudness_analyzer.cpp
    src/offline_flac_renderer.cpp
    src/offline_render.cpp
    src/offline_wav_renderer.cpp
    src/output_guard.cpp
    src/output_limiter.cpp
    src/playback.cpp
    src/parametric_equalizer.cpp
    src/phaser_vfx.cpp
    src/scene_vfx_processor.cpp
    src/source_edit_plan.cpp
    src/spring_space_reverb.cpp
    src/transform_vfx_processor.cpp
    src/tremolo_vfx.cpp
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
        echo-adaptive-noise-reducer-test
        tests/adaptive_noise_reducer_test.cpp
    )
    target_link_libraries(echo-adaptive-noise-reducer-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-adaptive-noise-reducer-test)
    add_test(NAME echo-adaptive-noise-reducer-test COMMAND echo-adaptive-noise-reducer-test)

    add_executable(
        echo-channel-repair-processor-test
        tests/channel_repair_processor_test.cpp
    )
    target_link_libraries(echo-channel-repair-processor-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-channel-repair-processor-test)
    add_test(
        NAME echo-channel-repair-processor-test
        COMMAND echo-channel-repair-processor-test
    )

    add_executable(
        echo-de-click-processor-test
        tests/de_click_processor_test.cpp
    )
    target_link_libraries(echo-de-click-processor-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-de-click-processor-test)
    add_test(NAME echo-de-click-processor-test COMMAND echo-de-click-processor-test)

    add_executable(
        echo-de-esser-test
        tests/de_esser_test.cpp
    )
    target_link_libraries(echo-de-esser-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-de-esser-test)
    add_test(NAME echo-de-esser-test COMMAND echo-de-esser-test)

    add_executable(
        echo-de-hum-filter-test
        tests/de_hum_filter_test.cpp
    )
    target_link_libraries(echo-de-hum-filter-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-de-hum-filter-test)
    add_test(NAME echo-de-hum-filter-test COMMAND echo-de-hum-filter-test)

    add_executable(
        echo-de-plosive-processor-test
        tests/de_plosive_processor_test.cpp
    )
    target_link_libraries(echo-de-plosive-processor-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-de-plosive-processor-test)
    add_test(NAME echo-de-plosive-processor-test COMMAND echo-de-plosive-processor-test)

    add_executable(echo-delay-vfx-processor-test tests/delay_vfx_processor_test.cpp)
    target_link_libraries(echo-delay-vfx-processor-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-delay-vfx-processor-test)
    add_test(NAME echo-delay-vfx-processor-test COMMAND echo-delay-vfx-processor-test)

    add_executable(
        echo-digital-degrade-vfx-processor-test
        tests/digital_degrade_vfx_processor_test.cpp
    )
    target_link_libraries(echo-digital-degrade-vfx-processor-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-digital-degrade-vfx-processor-test)
    add_test(
        NAME echo-digital-degrade-vfx-processor-test
        COMMAND echo-digital-degrade-vfx-processor-test
    )

    add_executable(echo-drive-vfx-processor-test tests/drive_vfx_processor_test.cpp)
    target_link_libraries(echo-drive-vfx-processor-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-drive-vfx-processor-test)
    add_test(NAME echo-drive-vfx-processor-test COMMAND echo-drive-vfx-processor-test)

    add_executable(
        echo-analysis-proxy-test
        tests/analysis_proxy_test.cpp
    )
    target_link_libraries(echo-analysis-proxy-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-analysis-proxy-test)
    add_test(NAME echo-analysis-proxy-test COMMAND echo-analysis-proxy-test)

    add_executable(
        echo-algorithmic-reverb-test
        tests/algorithmic_reverb_test.cpp
    )
    target_link_libraries(echo-algorithmic-reverb-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-algorithmic-reverb-test)
    add_test(NAME echo-algorithmic-reverb-test COMMAND echo-algorithmic-reverb-test)

    add_executable(
        echo-diffuse-space-reverb-test
        tests/diffuse_space_reverb_test.cpp
    )
    target_link_libraries(echo-diffuse-space-reverb-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-diffuse-space-reverb-test)
    add_test(NAME echo-diffuse-space-reverb-test COMMAND echo-diffuse-space-reverb-test)

    add_executable(echo-spring-space-reverb-test tests/spring_space_reverb_test.cpp)
    target_link_libraries(echo-spring-space-reverb-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-spring-space-reverb-test)
    add_test(NAME echo-spring-space-reverb-test COMMAND echo-spring-space-reverb-test)

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
        echo-effect-processing-chain-test
        tests/effect_processing_chain_test.cpp
    )
    target_link_libraries(echo-effect-processing-chain-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-effect-processing-chain-test)
    add_test(NAME echo-effect-processing-chain-test COMMAND echo-effect-processing-chain-test)

    add_executable(
        echo-effect-mask-plan-test
        tests/effect_mask_plan_test.cpp
    )
    target_link_libraries(echo-effect-mask-plan-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-effect-mask-plan-test)
    add_test(NAME echo-effect-mask-plan-test COMMAND echo-effect-mask-plan-test)

    add_executable(
        echo-source-edit-plan-test
        tests/source_edit_plan_test.cpp
    )
    target_link_libraries(echo-source-edit-plan-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-source-edit-plan-test)
    add_test(NAME echo-source-edit-plan-test COMMAND echo-source-edit-plan-test)

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

    add_executable(echo-modulation-vfx-processor-test tests/modulation_vfx_processor_test.cpp)
    target_link_libraries(echo-modulation-vfx-processor-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-modulation-vfx-processor-test)
    add_test(NAME echo-modulation-vfx-processor-test COMMAND echo-modulation-vfx-processor-test)

    add_executable(
        echo-offline-loudness-analyzer-test
        tests/offline_loudness_analyzer_test.cpp
    )

    add_executable(
        echo-offline-wav-renderer-test
        tests/offline_wav_renderer_test.cpp
    )
    target_link_libraries(echo-offline-wav-renderer-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-offline-wav-renderer-test)
    add_test(NAME echo-offline-wav-renderer-test COMMAND echo-offline-wav-renderer-test)
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

    add_executable(echo-scene-vfx-processor-test tests/scene_vfx_processor_test.cpp)
    target_link_libraries(echo-scene-vfx-processor-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-scene-vfx-processor-test)
    add_test(NAME echo-scene-vfx-processor-test COMMAND echo-scene-vfx-processor-test)

    add_executable(echo-transform-vfx-processor-test tests/transform_vfx_processor_test.cpp)
    target_link_libraries(echo-transform-vfx-processor-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-transform-vfx-processor-test)
    add_test(NAME echo-transform-vfx-processor-test COMMAND echo-transform-vfx-processor-test)
endif()
