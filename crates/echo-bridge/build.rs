//! Builds the C++ audio engine sources together with the generated CXX glue.
//!
//! The Cargo source manifest must stay identical to the `CMake` target
//! sources (`cpp/echo-audio/CMakeLists.txt`); the build fails closed on
//! divergence.

use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

const ENGINE_SOURCES: &[&str] = &[
    "src/bridge/cxx_bridge.cpp",
    "src/adaptive_noise_reducer.cpp",
    "src/auto_wah_vfx_processor.cpp",
    "src/beat_repeat_vfx_processor.cpp",
    "src/analysis_proxy.cpp",
    "src/algorithmic_reverb.cpp",
    "src/adjustment.cpp",
    "src/channel_repair_processor.cpp",
    "src/convolution/signalsmith_audiofft_adapter.cpp",
    "src/convolution_space_processor.cpp",
    "src/decode.cpp",
    "src/de_click_processor.cpp",
    "src/de_esser.cpp",
    "src/de_hum_filter.cpp",
    "src/de_plosive_processor.cpp",
    "src/delay_vfx_processor.cpp",
    "src/digital_degrade_vfx_processor.cpp",
    "src/drive_vfx_processor.cpp",
    "src/diffuse_space_reverb.cpp",
    "src/dynamics_processor.cpp",
    "src/effect_processing_chain.cpp",
    "src/effect_mask_plan.cpp",
    "src/freeze_vfx_processor.cpp",
    "src/granular_vfx_processor.cpp",
    "src/impulse_response_preparer.cpp",
    "src/k_weighting_filter.cpp",
    "src/low_cut_filter.cpp",
    "src/loudness_gain_advisor.cpp",
    "src/loudness_meter.cpp",
    "src/modulated_delay_vfx.cpp",
    "src/modulation_vfx_processor.cpp",
    "src/offline_assembly_wav_renderer.cpp",
    "src/offline_loudness_analyzer.cpp",
    "src/offline_flac_renderer.cpp",
    "src/offline_render.cpp",
    "src/offline_wav_renderer.cpp",
    "src/output_guard.cpp",
    "src/output_limiter.cpp",
    "src/playback.cpp",
    "src/parametric_equalizer.cpp",
    "src/pitch_vfx_processor.cpp",
    "src/phaser_vfx.cpp",
    "src/prepared_impulse_response.cpp",
    "src/rotary_vfx_processor.cpp",
    "src/scene_vfx_processor.cpp",
    "src/source_edit_plan.cpp",
    "src/spectrogram.cpp",
    "src/spectral_repair_processor.cpp",
    "src/spring_space_reverb.cpp",
    "src/stereo_vfx_processor.cpp",
    "src/space_processor.cpp",
    "src/transform_vfx_processor.cpp",
    "src/tremolo_vfx.cpp",
    "src/tape_vfx_processor.cpp",
    "src/waveform.cpp",
    "third_party/fftconvolver/FFTConvolver.cpp",
    "third_party/fftconvolver/Utilities.cpp",
];

const ENGINE_ADDITIONAL_INPUTS: &[&str] = &[
    "include/echo/audio/adaptive_noise_reducer.hpp",
    "include/echo/audio/auto_wah_vfx_processor.hpp",
    "include/echo/audio/beat_repeat_vfx_processor.hpp",
    "include/echo/audio/analysis_proxy.hpp",
    "include/echo/audio/algorithmic_reverb.hpp",
    "include/echo/audio/assembly.hpp",
    "include/echo/audio/adjustment.hpp",
    "include/echo/audio/channel_repair_processor.hpp",
    "include/echo/audio/convolution_space_processor.hpp",
    "include/echo/audio/decode.hpp",
    "include/echo/audio/de_click_processor.hpp",
    "include/echo/audio/de_esser.hpp",
    "include/echo/audio/de_hum_filter.hpp",
    "include/echo/audio/de_plosive_processor.hpp",
    "include/echo/audio/delay_vfx_processor.hpp",
    "include/echo/audio/digital_degrade_vfx_processor.hpp",
    "include/echo/audio/drive_vfx_processor.hpp",
    "include/echo/audio/diffuse_space_reverb.hpp",
    "include/echo/audio/dynamics_processor.hpp",
    "include/echo/audio/creative_vfx.hpp",
    "include/echo/audio/effect_processing_chain.hpp",
    "include/echo/audio/effect_mask_plan.hpp",
    "include/echo/audio/freeze_vfx_processor.hpp",
    "include/echo/audio/granular_vfx_processor.hpp",
    "include/echo/audio/impulse_response_preparer.hpp",
    "include/echo/audio/k_weighting_filter.hpp",
    "include/echo/audio/low_cut_filter.hpp",
    "include/echo/audio/loudness_gain_advisor.hpp",
    "include/echo/audio/loudness_meter.hpp",
    "include/echo/audio/modulation_vfx_processor.hpp",
    "include/echo/audio/offline_loudness_analyzer.hpp",
    "include/echo/audio/offline_assembly_wav_renderer.hpp",
    "include/echo/audio/offline_flac_renderer.hpp",
    "include/echo/audio/offline_render.hpp",
    "include/echo/audio/offline_wav_renderer.hpp",
    "include/echo/audio/output_guard.hpp",
    "include/echo/audio/output_limiter.hpp",
    "include/echo/audio/playback.hpp",
    "include/echo/audio/parametric_equalizer.hpp",
    "include/echo/audio/pitch_vfx_processor.hpp",
    "include/echo/audio/prepared_impulse_response.hpp",
    "include/echo/audio/rotary_vfx_processor.hpp",
    "include/echo/audio/scene_vfx_processor.hpp",
    "include/echo/audio/source_edit_plan.hpp",
    "include/echo/audio/spectrogram.hpp",
    "include/echo/audio/spectral_repair_processor.hpp",
    "include/echo/audio/spring_space_reverb.hpp",
    "include/echo/audio/space_processor.hpp",
    "include/echo/audio/stereo_vfx_processor.hpp",
    "include/echo/audio/transform_vfx_processor.hpp",
    "include/echo/audio/tape_vfx_processor.hpp",
    "include/echo/audio/waveform.hpp",
    "src/modulated_delay_vfx.hpp",
    "src/phaser_vfx.hpp",
    "src/tremolo_vfx.hpp",
    "src/bridge/cxx_bridge.hpp",
    "src/convolution/signalsmith_audiofft_adapter.hpp",
    "third_party/fftconvolver/FFTConvolver.h",
    "third_party/fftconvolver/Utilities.h",
    "third_party/signalsmith-dsp/common.h",
    "third_party/signalsmith-dsp/fft.h",
    "third_party/signalsmith-dsp/perf.h",
];

fn track_inputs(audio_root: &Path, inputs: &[&str]) {
    for relative_path in inputs {
        println!(
            "cargo:rerun-if-changed={}",
            audio_root.join(relative_path).display()
        );
    }
}

fn verify_source_manifest(audio_root: &Path) {
    let cmake_path = audio_root.join("CMakeLists.txt");
    let cmake = fs::read_to_string(&cmake_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", cmake_path.display()));
    let cmake_sources = cmake
        .split_whitespace()
        .map(|token| token.trim_matches(|character| matches!(character, '"' | '(' | ')')))
        .filter(|token| {
            (token.starts_with("src/") || token.starts_with("third_party/"))
                && !token.starts_with("src/bridge/")
                && Path::new(token)
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("cpp"))
        })
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>();
    let cargo_sources = ENGINE_SOURCES
        .iter()
        .filter(|path| !path.starts_with("src/bridge/"))
        .map(|path| (*path).to_owned())
        .collect::<BTreeSet<_>>();
    let missing = cmake_sources.difference(&cargo_sources).collect::<Vec<_>>();
    let extra = cargo_sources.difference(&cmake_sources).collect::<Vec<_>>();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "Cargo echo-audio source manifest diverged from CMake; missing={missing:?}; extra={extra:?}"
    );
}

fn main() {
    let crate_root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("crate root"));
    let repository_root = crate_root.join("../..");
    let audio_root = repository_root.join("cpp/echo-audio");
    let audio_include = audio_root.join("include");
    verify_source_manifest(&audio_root);

    let ffmpeg_libraries = ["libavformat", "libavcodec", "libavutil", "libswresample"];
    let mut include_paths = Vec::new();
    let mut link_paths = Vec::new();
    let mut link_libraries = Vec::new();
    for library in ffmpeg_libraries {
        let metadata = pkg_config::Config::new()
            .cargo_metadata(false)
            .probe(library)
            .unwrap_or_else(|_| panic!("pkg-config {library} must be discoverable"));
        include_paths.extend(metadata.include_paths.iter().cloned());
        for link_path in &metadata.link_paths {
            link_paths.push(link_path.clone());
        }
        for link_library in &metadata.libs {
            if link_library == "stdc++" {
                continue;
            }
            link_libraries.push(link_library.clone());
        }
    }
    for include_path in &include_paths {
        println!("cargo:rustc-link-search=native={}", include_path.display());
    }
    for link_path in &link_paths {
        println!("cargo:rustc-link-search=native={}", link_path.display());
    }
    for link_library in &link_libraries {
        println!("cargo:rustc-link-lib={link_library}");
    }

    let mut build = cxx_build::bridge("src/lib.rs");
    for relative_path in ENGINE_SOURCES {
        build.file(audio_root.join(relative_path));
    }
    build
        .include(&audio_include)
        .include(audio_root.join("src"))
        .include(&audio_root);
    let third_party_include = audio_root.join("third_party");
    if env::var("CARGO_CFG_TARGET_FAMILY").as_deref() == Ok("unix") {
        build
            .flag("-isystem")
            .flag(third_party_include.to_string_lossy().as_ref());
    } else {
        build.include(&third_party_include);
    }
    for include_path in &include_paths {
        if env::var("CARGO_CFG_TARGET_FAMILY").as_deref() == Ok("unix") {
            build
                .flag("-isystem")
                .flag(include_path.to_string_lossy().as_ref());
        } else {
            build.include(include_path);
        }
    }
    build.std("c++20");

    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        build.flag("/W4").flag("/permissive-");
    } else {
        build
            .flag("-Wall")
            .flag("-Wextra")
            .flag("-Wpedantic")
            .flag("-Wconversion")
            .flag("-Wsign-conversion");
    }

    build.compile("echo-bridge-cxx");

    println!("cargo:rerun-if-changed=src/lib.rs");
    track_inputs(&audio_root, ENGINE_SOURCES);
    track_inputs(&audio_root, ENGINE_ADDITIONAL_INPUTS);
}
