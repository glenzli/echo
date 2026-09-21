# Echo desktop application assembly: the Qt Quick shell over the Rust
# memory engine and C++ audio engine.

cmake_minimum_required(VERSION 3.28)

find_package(Qt6 6.5 REQUIRED COMPONENTS Concurrent LinguistTools Quick Multimedia)
find_package(PkgConfig REQUIRED)
find_package(Iconv REQUIRED)
qt_standard_project_setup(I18N_TRANSLATED_LANGUAGES zh_CN)

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
    "${PROJECT_SOURCE_DIR}/cpp/echo-audio/src/*.hpp"
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
include("${PROJECT_SOURCE_DIR}/cmake/EchoFFmpeg.cmake")

set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)

qt_add_executable(
    echo-desktop
    MACOSX_BUNDLE
    src/main.cpp
    src/independent_editor_controller.cpp
    src/independent_editor_controller.hpp
    src/application_paths.cpp
    src/application_paths.hpp
    src/desktop_selection_transcription.cpp
    src/selection_transcription_controller.cpp
    src/selection_transcription_controller.hpp
    src/generated_narration_controller.cpp
    src/generated_narration_controller.hpp
    src/desktop_sound_library.cpp
        src/desktop_source_disclosure.cpp
    src/desktop_backend.cpp
    src/desktop_backend.hpp
    src/impulse_response_controller.cpp
    src/impulse_response_controller.hpp
    src/batch_export_controller.cpp
    src/batch_export_controller.hpp
    src/playback_controller.cpp
    src/playback_controller.hpp
    src/playback_session_handoff.hpp
    src/creative_vfx_projection.cpp
    src/creative_vfx_projection.hpp
    src/creative_vfx_presets.cpp
    src/creative_vfx_presets.hpp
    src/parametric_equalizer_projection.cpp
    src/parametric_equalizer_projection.hpp
    src/playback_adjustment_projection.cpp
    src/space_projection.cpp
    src/playback_adjustment_projection.hpp
    src/render_export_controller.cpp
    src/render_export_controller.hpp
    src/rendered_spectral_working_copy_controller.cpp
    src/rendered_spectral_working_copy_controller.hpp
    src/qt_render_byte_sink.hpp
    src/semantic_search_controller.cpp
    src/semantic_search_controller.hpp
    src/assembly_waveform_controller.cpp
    src/assembly_waveform_controller.hpp
    src/prepared_assembly_source_cache.cpp
    src/prepared_assembly_source_cache.hpp
    src/sound_assembly_controller.cpp
    src/sound_assembly_controller.hpp
    src/click_analysis_controller.cpp
    src/click_analysis_controller.hpp
    src/noise_profile_controller.cpp
    src/noise_profile_controller.hpp
    src/noise_profile_projection.cpp
    src/noise_profile_projection.hpp
    src/spectrogram_preview_controller.cpp
    src/spectrogram_preview_controller.hpp
    src/reverb_projection.cpp
    src/reverb_projection.hpp
    src/restoration_projection.cpp
    src/restoration_projection.hpp
    src/ui_preferences.cpp
    src/ui_preferences.hpp
    src/inference_preferences.cpp
    src/inference_preferences.hpp
    src/loudness_analysis_controller.cpp
    src/loudness_analysis_controller.hpp
    src/mac_titlebar.hpp
)
if(APPLE)
    target_sources(echo-desktop PRIVATE src/mac_titlebar.mm)

    set(ECHO_APP_ICON_FILE "${CMAKE_CURRENT_SOURCE_DIR}/assets/app-icon.icns")
    set_source_files_properties(
        "${ECHO_APP_ICON_FILE}"
        PROPERTIES MACOSX_PACKAGE_LOCATION Resources
    )
    target_sources(echo-desktop PRIVATE "${ECHO_APP_ICON_FILE}")
endif()
set_source_files_properties(qml/Theme.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)
set_source_files_properties(qml/SoundSemantics.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)
qt_add_qml_module(
    echo-desktop
    URI EchoDesktop
    VERSION 0.1
    QML_FILES
        qml/SourceDisclosure.js
        qml/SourceDisclosureBadge.qml
        qml/SourceDisclosureDialog.qml
        qml/SourceDisclosureSmoke.qml
        qml/EditorOpenError.qml
        qml/IndependentEditor.qml
        qml/IndependentAiSmoke.qml
        qml/IndependentEditorSmoke.qml
        qml/Main.qml
        qml/AudioMetadataBar.qml
        qml/AudioLibraryWorkspace.qml
        qml/AudioLibrarySidebar.qml
        qml/AudioPlaybackWorkspace.qml
        qml/AdvancedEffectsRack.qml
        qml/AdjustmentPanelHeader.qml
        qml/AudioSpaceWorkspace.qml
        qml/BasicAdjustmentPanel.qml
        qml/ChannelRepairPanel.qml
        qml/CreativeVfxPanel.qml
        qml/SceneVfxPanel.qml
        qml/DelayVfxPanel.qml
        qml/DigitalDegradeVfxPanel.qml
        qml/DriveVfxPanel.qml
        qml/FreezeVfxPanel.qml
        qml/GranularVfxPanel.qml
        qml/TapeVfxPanel.qml
        qml/PitchVfxPanel.qml
        qml/AutoWahVfxPanel.qml
        qml/BeatRepeatVfxPanel.qml
        qml/ModulationVfxPanel.qml
        qml/TransformVfxPanel.qml
        qml/RotaryVfxPanel.qml
        qml/DeClickPanel.qml
        qml/DePlosivePanel.qml
        qml/DeHumPanel.qml
        qml/DynamicsPanel.qml
        qml/SoundSignalChain.qml
        qml/EffectCatalog.qml
        qml/EchoButton.qml
        qml/EchoIcon.qml
        qml/EchoIconButton.qml
        qml/EchoParameterSlider.qml
        qml/EchoStereoMeter.qml
        qml/EchoSegmentedControl.qml
        qml/EchoSectionLabel.qml
        qml/EchoSettingsDialog.qml
        qml/EchoSwitch.qml
        qml/EchoComboBox.qml
        qml/EchoTextField.qml
        qml/InspectorSection.qml
        qml/LoudnessTargetControl.qml
        qml/ListeningProgressTracker.qml
        qml/MainTitleBar.qml
        qml/UnsavedEditDialog.qml
        qml/EchoWindowChrome.qml
        qml/EchoWorkspaceTab.qml
        qml/IndependentTitleBar.qml
        qml/MasterOutputPanel.qml
        qml/MetadataCalibrationEditor.qml
        qml/ParametricEqGraph.qml
        qml/ProcessingHistoryDialog.qml
        qml/ProcessingRecipeApplyDialog.qml
        qml/ProcessingRecipeManagerDialog.qml
        qml/ProcessingRecipePicker.qml
        qml/ProcessingRecipeSaveDialog.qml
        qml/RevisitAlbumCard.qml
        qml/RevisitDashboard.qml
        qml/RevisitSoundStrip.qml
        qml/RevisitState.qml
        qml/RestorationPanel.qml
        qml/BatchExportDialog.qml
        qml/SoundAdvancedFilterPopup.qml
        qml/SoundAlbumNameDialog.qml
        qml/SoundAlbumRow.qml
        qml/SoundAlbumState.qml
        qml/SoundAdjustmentDraft.qml
        qml/SoundAdjustmentEditor.qml
        qml/EchoValueSpinBox.qml
        qml/EchoTimeSpinBox.qml
        qml/SoundAssemblyEditing.js
        qml/SoundAssemblySelection.js
        qml/SoundAutomation.js
        qml/SoundDuckingPanel.qml
        qml/SoundGainEnvelope.qml
        qml/SoundAssemblyClip.qml
        qml/SoundAssemblyTrack.qml
        qml/SoundAssemblyMarkers.qml
        qml/SoundAssemblyMarkerLane.qml
        qml/MemoryWorkflowSmoke.qml
        qml/MultitrackWorkflowSmoke.qml
        qml/SpectralWorkflowSmoke.qml
        qml/NoiseWorkflowSmoke.qml
        qml/MemorySourceReferences.qml
        qml/SoundPlaybackSource.qml
        qml/SoundSourceSearch.js
        qml/SoundSourceBrowser.qml
        qml/GeneratedNarrationDialog.qml
        qml/SoundAssemblyInspector.qml
        qml/SoundAssemblyWorkspace.qml
        qml/SpaceReverbPanel.qml
        qml/SpectrogramView.qml
        qml/SpectralRepairSurface.qml
        qml/NoiseProfileEditing.js
        qml/NoiseReductionPanel.qml
        qml/SpectralEditing.js
        qml/StereoVfxPanel.qml
        qml/ImpulseResponseImportDialog.qml
        qml/SoundEditorTimeline.qml
        qml/SourceEditTimeline.qml
        qml/SourceRegionInspector.qml
        qml/EffectMaskEditor.qml
        qml/SoundTranscriptEditing.js
        qml/SourceEditRanges.js
        qml/SoundClickRepairPanel.qml
        qml/ClickRepairEditing.js
        qml/ClickRepairSmoke.qml
        qml/SoundTranscriptPanel.qml
        qml/EchoCheckBox.qml
        qml/SoundEditingWorkspace.qml
        qml/SoundExportDialog.qml
        qml/SoundCard.qml
        qml/SoundFilmstripItem.qml
        qml/SoundFilterState.qml
        qml/SoundFocusView.qml
        qml/SoundInspector.qml
        qml/SoundLibraryRow.qml
        qml/SoundMultiSelectionState.qml
        qml/SoundMultiSelectionToolbar.qml
        qml/SoundPresentationToolbar.qml
        qml/SoundSelectionToolbar.qml
        qml/SoundSemanticTag.qml
        qml/SoundSemantics.qml
        qml/SoundTapeSegment.qml
        qml/SoundTapeView.qml
        qml/SoundWall.qml
        qml/SoundWallBottomBar.qml
        qml/Theme.qml
        qml/ToneEqualizerPanel.qml
        qml/WaveformView.qml
    RESOURCES
        icons/play.svg
        icons/marker.svg
        icons/pause.svg
        icons/stop.svg
        icons/refresh.svg
        icons/tune.svg
        icons/waveform.svg
        icons/mic.svg
        icons/folder.svg
        icons/review-grid.svg
        icons/filmstrip.svg
        icons/tape.svg
        icons/assembly.svg
        icons/filter.svg
        icons/filter-off.svg
        icons/edit.svg
        icons/undo.svg
        icons/redo.svg
        icons/crop.svg
        icons/dynamics.svg
        icons/reset-all.svg
        icons/high-pass.svg
        icons/gain.svg
        icons/fade-linear.svg
        icons/fade-smooth.svg
        icons/fade-equal-power.svg
        icons/equalizer.svg
        icons/export.svg
        icons/history.svg
        icons/heart.svg
        icons/heart-filled.svg
        icons/star.svg
        icons/star-filled.svg
        icons/clock.svg
        icons/source-missing.svg
        icons/album.svg
        icons/album-add.svg
        icons/sparkles.svg
        icons/plus.svg
        icons/more-horizontal.svg
        icons/chevron-down.svg
        icons/follow-playhead.svg
        icons/loop.svg
        icons/fit-selection.svg
        icons/fit-all.svg
        icons/clear-selection.svg
        icons/recipe-save.svg
        icons/recipe-apply.svg
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
        Qt6::Concurrent
        echo-desktop-rust
        PkgConfig::FFmpeg
        ${CMAKE_DL_LIBS}
        Iconv::Iconv
)

add_dependencies(echo-desktop echo-desktop-rust-build)

set_target_properties(
    echo-desktop
    PROPERTIES
        MACOSX_BUNDLE_INFO_PLIST "${CMAKE_CURRENT_SOURCE_DIR}/Info.plist.in"
        MACOSX_BUNDLE_BUNDLE_NAME "Echo"
        MACOSX_BUNDLE_GUI_IDENTIFIER "dev.echo.audio"
        MACOSX_BUNDLE_ICON_FILE "app-icon.icns"
        MACOSX_BUNDLE_SHORT_VERSION_STRING "${PROJECT_VERSION}"
        MACOSX_BUNDLE_BUNDLE_VERSION "1"
        OUTPUT_NAME "Echo"
)

# Translation contract: echo_zh_CN.ts is compiled into the /i18n resource.
qt_add_translations(
    echo-desktop
    TARGETS echo-desktop
    SOURCE_TARGETS echo-desktop
    TS_FILE_BASE echo
    TS_FILE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/i18n"
    RESOURCE_PREFIX "/i18n"
    LUPDATE_TARGET echo-desktop-update-translations
    LUPDATE_OPTIONS -no-obsolete
    LRELEASE_TARGET echo-desktop-release-translations
    LRELEASE_OPTIONS -nounfinished
    NO_GENERATE_PLURALS_TS_FILE
)

if(BUILD_TESTING)
    add_executable(echo-playback-session-handoff-test tests/playback_session_handoff_test.cpp)
    target_include_directories(echo-playback-session-handoff-test PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src")
    target_compile_features(echo-playback-session-handoff-test PRIVATE cxx_std_20)
    target_link_libraries(echo-playback-session-handoff-test PRIVATE Echo::Audio Qt6::Core)
    echo_audio_enable_warnings(echo-playback-session-handoff-test)
    add_test(NAME echo-playback-session-handoff-test COMMAND echo-playback-session-handoff-test)
    set_tests_properties(echo-playback-session-handoff-test PROPERTIES TIMEOUT 30)
    add_executable(echo-click-analysis-controller-test tests/click_analysis_controller_test.cpp src/click_analysis_controller.cpp src/click_analysis_controller.hpp)
    target_include_directories(echo-click-analysis-controller-test PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src")
    target_compile_features(echo-click-analysis-controller-test PRIVATE cxx_std_20)
    target_link_libraries(echo-click-analysis-controller-test PRIVATE Qt6::Core Qt6::Concurrent Echo::Audio)
    echo_audio_enable_warnings(echo-click-analysis-controller-test)
    add_test(NAME echo-click-analysis-controller-test COMMAND echo-click-analysis-controller-test)
    add_executable(echo-selection-transcription-controller-test tests/selection_transcription_controller_test.cpp src/selection_transcription_controller.cpp src/selection_transcription_controller.hpp)
    target_include_directories(echo-selection-transcription-controller-test PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src" "${ECHO_CXXBRIDGE_INCLUDE_DIRECTORY}")
    target_compile_features(echo-selection-transcription-controller-test PRIVATE cxx_std_20)
    target_link_libraries(echo-selection-transcription-controller-test PRIVATE Qt6::Core Qt6::Concurrent echo-desktop-rust PkgConfig::FFmpeg ${CMAKE_DL_LIBS} Iconv::Iconv)
    add_dependencies(echo-selection-transcription-controller-test echo-desktop-rust-build)
    echo_audio_enable_warnings(echo-selection-transcription-controller-test)
    add_test(NAME echo-selection-transcription-controller-test COMMAND echo-selection-transcription-controller-test)
    add_executable(echo-generated-narration-controller-test tests/generated_narration_controller_test.cpp src/generated_narration_controller.cpp src/generated_narration_controller.hpp)
    target_include_directories(echo-generated-narration-controller-test PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src" "${ECHO_CXXBRIDGE_INCLUDE_DIRECTORY}")
    target_compile_features(echo-generated-narration-controller-test PRIVATE cxx_std_20)
    target_link_libraries(echo-generated-narration-controller-test PRIVATE Qt6::Core Qt6::Concurrent echo-desktop-rust PkgConfig::FFmpeg ${CMAKE_DL_LIBS} Iconv::Iconv)
    add_dependencies(echo-generated-narration-controller-test echo-desktop-rust-build)
    echo_audio_enable_warnings(echo-generated-narration-controller-test)
    add_test(NAME echo-generated-narration-controller-test COMMAND echo-generated-narration-controller-test)
    add_executable(echo-prepared-assembly-source-cache-test tests/prepared_assembly_source_cache_test.cpp src/prepared_assembly_source_cache.cpp)
    target_include_directories(echo-prepared-assembly-source-cache-test PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src")
    target_compile_features(echo-prepared-assembly-source-cache-test PRIVATE cxx_std_20)
    target_link_libraries(echo-prepared-assembly-source-cache-test PRIVATE Echo::Audio Qt6::Core)
    echo_audio_enable_warnings(echo-prepared-assembly-source-cache-test)
    add_test(NAME echo-prepared-assembly-source-cache-test COMMAND echo-prepared-assembly-source-cache-test)
    add_executable(echo-semantic-search-controller-test tests/semantic_search_controller_test.cpp src/semantic_search_controller.cpp src/semantic_search_controller.hpp)
    target_include_directories(echo-semantic-search-controller-test PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src" "${ECHO_CXXBRIDGE_INCLUDE_DIRECTORY}")
    target_compile_features(echo-semantic-search-controller-test PRIVATE cxx_std_20)
    target_link_libraries(echo-semantic-search-controller-test PRIVATE Qt6::Core Qt6::Concurrent echo-desktop-rust PkgConfig::FFmpeg ${CMAKE_DL_LIBS} Iconv::Iconv)
    add_dependencies(echo-semantic-search-controller-test echo-desktop-rust-build)
    echo_audio_enable_warnings(echo-semantic-search-controller-test)
    add_test(NAME echo-semantic-search-controller-test COMMAND echo-semantic-search-controller-test)
    add_executable(echo-spectrogram-preview-test tests/spectrogram_preview_controller_test.cpp src/spectrogram_preview_controller.cpp src/spectrogram_preview_controller.hpp)
    target_include_directories(echo-spectrogram-preview-test PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src")
    target_compile_features(echo-spectrogram-preview-test PRIVATE cxx_std_20)
    target_link_libraries(echo-spectrogram-preview-test PRIVATE Echo::Audio Qt6::Core Qt6::Gui Qt6::Concurrent)
    echo_audio_enable_warnings(echo-spectrogram-preview-test)
    add_test(NAME echo-spectrogram-preview-test COMMAND echo-spectrogram-preview-test)
    add_executable(
        echo-application-paths-test
        src/application_paths.cpp
        tests/application_paths_test.cpp
    )
    target_compile_features(echo-application-paths-test PRIVATE cxx_std_20)
    target_include_directories(
        echo-application-paths-test
        PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src"
    )
    target_link_libraries(echo-application-paths-test PRIVATE Qt6::Core)
    echo_audio_enable_warnings(echo-application-paths-test)
    add_test(NAME echo-application-paths-test COMMAND echo-application-paths-test)

    add_executable(
        echo-creative-vfx-projection-test
        src/creative_vfx_projection.cpp
        tests/creative_vfx_projection_test.cpp
    )
    target_compile_features(echo-creative-vfx-projection-test PRIVATE cxx_std_20)
    target_include_directories(
        echo-creative-vfx-projection-test
        PRIVATE
            "${CMAKE_CURRENT_SOURCE_DIR}/src"
            "${PROJECT_SOURCE_DIR}/cpp/echo-audio/include"
    )
    target_link_libraries(echo-creative-vfx-projection-test PRIVATE Qt6::Core)
    echo_audio_enable_warnings(echo-creative-vfx-projection-test)
    add_test(
        NAME echo-creative-vfx-projection-test
        COMMAND echo-creative-vfx-projection-test
    )
endif()

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
