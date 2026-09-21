include("${PROJECT_SOURCE_DIR}/cmake/EchoFFmpeg.cmake")

add_library(
    echo-audio STATIC
    src/adaptive_noise_reducer.cpp
    src/auto_wah_vfx_processor.cpp
    src/beat_repeat_vfx_processor.cpp
    src/analysis_proxy.cpp
    src/algorithmic_reverb.cpp
    src/adjustment.cpp
    src/channel_repair_processor.cpp
    src/convolution/signalsmith_audiofft_adapter.cpp
    src/convolution_space_processor.cpp
    src/ffmpeg_input.cpp
    src/ffmpeg_timing.cpp
    src/decode.cpp
    src/de_click_processor.cpp
    src/click_analysis.cpp
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
    src/freeze_vfx_processor.cpp
    src/granular_vfx_processor.cpp
    src/impulse_response_preparer.cpp
    src/k_weighting_filter.cpp
    src/low_cut_filter.cpp
    src/loudness_gain_advisor.cpp
    src/loudness_meter.cpp
    src/modulated_delay_vfx.cpp
    src/modulation_vfx_processor.cpp
    src/offline_loudness_analyzer.cpp
    src/assembly_mixer.cpp
    src/assembly_playback.cpp
    src/prepared_pcm_reader.cpp
    src/offline_assembly_wav_renderer.cpp
    src/offline_flac_renderer.cpp
    src/export_metadata.cpp
    src/audio_streams.cpp
    src/audio_export.cpp
    src/offline_render.cpp
    src/offline_wav_renderer.cpp
    src/output_guard.cpp
    src/output_limiter.cpp
    src/playback.cpp
    src/parametric_equalizer.cpp
    src/pitch_vfx_processor.cpp
    src/phaser_vfx.cpp
    src/prepared_impulse_response.cpp
    src/rotary_vfx_processor.cpp
    src/scene_vfx_processor.cpp
    src/source_edit_plan.cpp
    src/spectrogram.cpp
    src/spectrogram_detail.cpp
    src/profiled_noise_reduction.cpp
    src/spectral_repair_processor.cpp
    src/spring_space_reverb.cpp
    src/stereo_vfx_processor.cpp
    src/space_processor.cpp
    src/transform_vfx_processor.cpp
    src/tremolo_vfx.cpp
    src/tape_vfx_processor.cpp
    src/waveform.cpp
    third_party/fftconvolver/FFTConvolver.cpp
    third_party/fftconvolver/Utilities.cpp
)
add_library(Echo::Audio ALIAS echo-audio)

target_include_directories(echo-audio PUBLIC include PRIVATE src)
target_include_directories(echo-audio SYSTEM PRIVATE third_party)

target_compile_features(echo-audio PUBLIC cxx_std_20)

# Keep the runnable Debug editor responsive while retaining symbols and checks.
# The DSP runs behind the same boundaries in Cargo and CMake; do not use fast-math.
# MSVC keeps its existing /RTC debug policy; /O2 would conflict with it.
if(NOT MSVC)
    target_compile_options(echo-audio PRIVATE "$<$<CONFIG:Debug>:-O2>")
endif()

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
    add_executable(echo-audio-export-test tests/audio_export_test.cpp)
    target_link_libraries(echo-audio-export-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-audio-export-test)
    add_test(NAME echo-audio-export-test COMMAND echo-audio-export-test)
    add_executable(echo-click-analysis-test tests/click_analysis_test.cpp)
    target_link_libraries(echo-click-analysis-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-click-analysis-test)
    add_test(NAME echo-click-analysis-test COMMAND echo-click-analysis-test)
    set_tests_properties(echo-click-analysis-test PROPERTIES TIMEOUT 30)
    add_executable(echo-assembly-playback-test tests/assembly_playback_test.cpp)
    target_link_libraries(echo-assembly-playback-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-assembly-playback-test)
    add_test(NAME echo-assembly-playback-test COMMAND echo-assembly-playback-test)
    set_tests_properties(echo-assembly-playback-test PROPERTIES TIMEOUT 30)
    add_executable(echo-audio-workflow-contract-test tests/audio_workflow_contract_test.cpp)
    target_link_libraries(echo-audio-workflow-contract-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-audio-workflow-contract-test)
    add_test(NAME echo-audio-workflow-contract-test COMMAND echo-audio-workflow-contract-test)

    add_executable(echo-profiled-noise-reduction-test tests/profiled_noise_reduction_test.cpp)
    target_link_libraries(echo-profiled-noise-reduction-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-profiled-noise-reduction-test)
    add_test(NAME echo-profiled-noise-reduction-test COMMAND echo-profiled-noise-reduction-test)

    add_executable(echo-auto-wah-vfx-processor-test tests/auto_wah_vfx_processor_test.cpp)
    target_link_libraries(echo-auto-wah-vfx-processor-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-auto-wah-vfx-processor-test)
    add_test(NAME echo-auto-wah-vfx-processor-test COMMAND echo-auto-wah-vfx-processor-test)

    add_executable(echo-beat-repeat-vfx-processor-test tests/beat_repeat_vfx_processor_test.cpp)
    target_link_libraries(echo-beat-repeat-vfx-processor-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-beat-repeat-vfx-processor-test)
    add_test(NAME echo-beat-repeat-vfx-processor-test COMMAND echo-beat-repeat-vfx-processor-test)

    add_executable(echo-stereo-vfx-processor-test tests/stereo_vfx_processor_test.cpp)
    target_link_libraries(echo-stereo-vfx-processor-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-stereo-vfx-processor-test)
    add_test(NAME echo-stereo-vfx-processor-test COMMAND echo-stereo-vfx-processor-test)

add_executable(echo-spectrogram-detail-test tests/spectrogram_detail_test.cpp)
target_link_libraries(echo-spectrogram-detail-test PRIVATE Echo::Audio)
echo_audio_enable_warnings(echo-spectrogram-detail-test)
add_test(NAME echo-spectrogram-detail-test COMMAND echo-spectrogram-detail-test)

add_executable(echo-spectrogram-test tests/spectrogram_test.cpp)
    target_link_libraries(echo-spectrogram-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-spectrogram-test)
add_test(NAME echo-spectrogram-test COMMAND echo-spectrogram-test)

add_executable(echo-spectral-repair-processor-test tests/spectral_repair_processor_test.cpp)
target_link_libraries(echo-spectral-repair-processor-test PRIVATE echo-audio)
add_test(NAME echo-spectral-repair-processor-test COMMAND echo-spectral-repair-processor-test)

    add_executable(echo-pitch-vfx-processor-test tests/pitch_vfx_processor_test.cpp)
    target_link_libraries(echo-pitch-vfx-processor-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-pitch-vfx-processor-test)
    add_test(NAME echo-pitch-vfx-processor-test COMMAND echo-pitch-vfx-processor-test)

    add_executable(echo-tape-vfx-processor-test tests/tape_vfx_processor_test.cpp)
    target_link_libraries(echo-tape-vfx-processor-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-tape-vfx-processor-test)
    add_test(NAME echo-tape-vfx-processor-test COMMAND echo-tape-vfx-processor-test)

    add_executable(echo-freeze-vfx-processor-test tests/freeze_vfx_processor_test.cpp)
    target_link_libraries(echo-freeze-vfx-processor-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-freeze-vfx-processor-test)
    add_test(NAME echo-freeze-vfx-processor-test COMMAND echo-freeze-vfx-processor-test)

    add_executable(echo-granular-vfx-processor-test tests/granular_vfx_processor_test.cpp)
    target_link_libraries(echo-granular-vfx-processor-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-granular-vfx-processor-test)
    add_test(NAME echo-granular-vfx-processor-test COMMAND echo-granular-vfx-processor-test)

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
        echo-convolution-space-processor-test
        tests/convolution_space_processor_test.cpp
    )
    target_link_libraries(echo-convolution-space-processor-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-convolution-space-processor-test)
    add_test(
        NAME echo-convolution-space-processor-test
        COMMAND echo-convolution-space-processor-test
    )

    add_executable(echo-space-processor-test tests/space_processor_test.cpp)
    target_link_libraries(echo-space-processor-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-space-processor-test)
    add_test(NAME echo-space-processor-test COMMAND echo-space-processor-test)

    add_executable(echo-rotary-vfx-processor-test tests/rotary_vfx_processor_test.cpp)
    target_link_libraries(echo-rotary-vfx-processor-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-rotary-vfx-processor-test)
    add_test(NAME echo-rotary-vfx-processor-test COMMAND echo-rotary-vfx-processor-test)

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
        echo-impulse-response-preparer-test
        tests/impulse_response_preparer_test.cpp
    )
    target_link_libraries(echo-impulse-response-preparer-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-impulse-response-preparer-test)
    add_test(
        NAME echo-impulse-response-preparer-test
        COMMAND echo-impulse-response-preparer-test
    )

    add_executable(
        echo-prepared-impulse-response-test
        tests/prepared_impulse_response_test.cpp
    )
    target_link_libraries(echo-prepared-impulse-response-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-prepared-impulse-response-test)
    add_test(
        NAME echo-prepared-impulse-response-test
        COMMAND echo-prepared-impulse-response-test
    )

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
        echo-offline-assembly-wav-renderer-test
        tests/offline_assembly_wav_renderer_test.cpp
    )
    target_link_libraries(echo-offline-assembly-wav-renderer-test PRIVATE Echo::Audio)
    echo_audio_enable_warnings(echo-offline-assembly-wav-renderer-test)
    add_test(
        NAME echo-offline-assembly-wav-renderer-test
        COMMAND echo-offline-assembly-wav-renderer-test
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
