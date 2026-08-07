# Echo ROADMAP

本文档是 Echo 的产品设计与工程决策记录。改动架构、数据模型或里程碑边界时，先更新这里。

## 1. 产品定义

> Echo 是一个 local-first、source-anchored 的声音记忆系统。它负责保存、理解、索引、修复和重新聆听真实发生过的声音，而不是创造新的声音作品。

Echo 不是 DAW，不是 Audacity + AI，不是 TTS Playground。

能力划分为四个域：

```text
Preserve   原始记录、provenance、非破坏性版本
Understand ASR、人物、情绪、声音事件、语义索引
Restore    降噪、响度、EQ、去混响、修复录制缺陷
Revisit    声音空间 / 声音相册（最高层）
```

关键定位：**Library 是一等公民，Editor 是 Library 的能力，而不是反过来。**

系列定位：Shadow = Photo Library + RAW Developer + AI Understanding；Echo = Audio Library + Audio Restoration + AI Understanding。

## 2. 核心原则（不可动摇）

1. **Original immutable**：原始文件永不修改，永远保留 original_ref。
2. **Analysis 不是事实**：所有 AI 结果必须存 `value + model + model_version + confidence + timestamp`，模型升级后可重新分析。
3. **Cache 可全部删除**：waveform、embedding、transcript cache、render proxy 全部可重建。
4. **实时音频路径保持极度简单**：audio callback 里绝不出现 Rust→AI→Python→allocator→async 链。AI 只在后台工作，绝不侵入播放链路。
5. **Audio Space 是首屏**：Listen first, Edit second。第一屏不是 Timeline Editor。
6. **调整克制**：只做恢复性调整（Loudness/EQ/降噪/去混响/Dynamics/Trim/Fade/Channel）。不做语音生成/改词/换声/音乐生成/SFX——那些属于未来的 Audio Studio，Echo 的 identity 不能漂。

## 3. 技术架构

```text
┌──────────────────────────────────────┐
│              Qt / QML                │
│ UI / Audio Space / Waveform / Info   │
└─────────────────┬────────────────────┘
                  │
            C++ Application
                  │
        ┌─────────┴──────────┐
        │                    │
   Audio Engine          Rust Core
   C++ / DSP             Memory Engine
        │                    │
 decoder/playback        catalog/index
 render graph            provenance
 effects                 jobs/search
 waveform                AI routing
        │                    │
        └──────────┬─────────┘
                   │
               AI Workers
      ┌────────────┼────────────┐
      │            │            │
 whisper.cpp   MLX/ONNX     Cloud API
```

### Qt（UI / Library / Waveform / Inspector）

- Qt Multimedia 只负责设备 I/O（`QAudioSink` 回调式 API），audio graph、waveform、effect pipeline、seek/cache 自己管理。
- **Qt 6.11 实测约束（2026-08 验证）**：`QAudioSink::start(QIODevice*)` 拉取模式在 CoreAudio 后端失效（Active 后直接 Idle，从不调用 readData）；`QAudioSink::stop()` 后重复 dispose 会崩溃。正确用法是**回调式 API** `sink.start([](QSpan<float>){...})`，且**单个 sink 全程复用**（suspend/resume 代替销毁）。
- 因此播放引擎对设备输出**统一立体声 48kHz float**（mono 由 resampler 上混、stereo 直通），设备格式保持稳定；canonical PCM 表示仍是 channel-preserving。

### C++（Audio Engine）

- FFmpeg 集成（decode/encode/filter）
- PCM buffer、ring buffer、waveform pyramid
- sample-accurate seek、resampling
- DSP graph、实时 effect 处理、播放引擎

### Rust（Memory Engine）

- library scanner、asset identity
- SQLite catalog、metadata、provenance
- 后台 job 系统、analysis 结果
- semantic search、speaker/event 关系
- AI capability routing

## 4. 音频底层

- 不自己重写 codec，直接用 FFmpeg / libavfilter。
- 内部统一 canonical PCM：**float32、48 kHz、channel-preserving**。真正输出时再编码。原始文件永远不动。

## 5. 数据模型：AudioAsset

```text
AudioAsset
├── Original        content hash / path / codec / timestamp / metadata（immutable）
├── Analysis        transcript / speakers / emotions / audio events / embeddings / segments
├── AdjustmentGraph gain / eq / denoise / normalize / trim（非破坏性）
├── DerivedRenders  可重建
└── Relations       people / place / event（用户逐步确认，append-only）
```

## 6. 分级分析（20TB 级世界的前提）

绝不"导入→所有模型全跑一遍"。progressively enriched：

```text
Level 0  文件 metadata / duration / waveform peaks
Level 1  VAD / basic audio classification
Level 2  ASR
Level 3  speaker / emotion / events
Level 4  CLAP embedding / semantic indexing
Level 5  LLM contextual understanding / memory association
```

首次 import 很快可浏览；机器空闲时慢慢理解过去。

## 7. AI 模型选型与路由

**模型获取策略（2026-08 定）**：Echo 不负责下载模型。用户用本机 `hf` 工具维护共享的
`HuggingFace` 缓存（默认根 `~/.cache/huggingface/hub`，可用 `HF_HOME`/`HF_HUB_CACHE`
覆盖）；`echo-ai` 的模型注册表把逻辑模型解析到缓存快照路径，缺失时给出精确的
`hf download <repo>` 命令。设置面板提供模型目录配置与各模型状态。运行时经
`mlx_audio` 的 `generate.py`（`--model --audio --format json`）子进程调用，使用
TTS 实验的 venv（Python 3.11 + mlx）。

| 能力 | 模型 | 说明 |
| --- | --- | --- |
| 主 ASR（说了什么） | Qwen3-ASR 0.6B/1.7B | 52 种语言方言；`mlx-community/Qwen3-ASR-1.7B` |
| 时间对齐 | Qwen3-ForcedAligner 0.6B | word/sentence timestamp，11 种语言 |
| 发生了什么 | SenseVoiceSmall | ASR + language + emotion + audio events（中文/粤语/英文/日/韩，BGM/掌声/笑声/哭声/咳嗽） |
| 谁在讲话 | FunASR：VAD + CAM++ speaker model + 标点 | speaker diarization；用户确认 Speaker A=我、B=孩子 |
| 稳定 baseline | whisper.cpp | minimal install 的转录能力；Core ML encoder 可放 ANE；所有 Python/MLX 模型坏了 Echo 仍完整 |
| 声音检索 | CLAP | audio↔text 同一 embedding space；"找火车声"不依赖 transcript；Transcript embedding + CLAP audio embedding 双索引 |

Effect 策略：传统 DSP（gain/loudness/EQ/filter/compressor/limiter/fade/resample/normalize）CPU 即可，甚至 DeepFilterNet 级别降噪也是低复杂度实时可跑。GPU 只用于 heavy speech enhancement、source separation、de-reverb、neural restoration、大 ASR、audio embedding batch。

InferenceBackend 抽象：

```text
InferenceBackend
├── MLX / CoreML / CUDA / ONNX Runtime / whisper.cpp / Cloud
```

业务层不感知底层 GPU。effect graph CPU-first，AI Effect 单独进 inference backend。

## 8. UI 原则

- 首屏是 Audio Space（时间 / 人物 / 声音类型 / Revisit），不是 Timeline。
- 点击 asset 后进入：Waveform / Transcript / Events / People / Adjustment。
- 组件命名延续系列设计语言：`EchoButton`、`EchoIcon`……与 Shadow 的 `Shadow*` 组件对应，视觉 token 一致（Shadow/Echo 同系列）。

## 9. 里程碑

- **M0 Audio Foundation**：Qt 播放、FFmpeg decode、waveform、SQLite、immutable Original、后台 job。
  - 已完成：FFmpeg probe/流式解码、waveform pyramid（缓存化）、Qt 回调式播放（seek/pause/volume）、Audio Space 列表 + 波形详情。
  - 待办：后台 job 调度（等 M1 ASR 出现第一个真实消费方再建）、导入目录扫描、播放进度的波形联动优化。
- **M1 Understand**：Qwen3-ASR + forced alignment + SenseVoice，waveform ↔ transcript 双向同步。
  - 已完成（2026-08）：模型注册表（HF 缓存解析、缺失提示）；ASR worker
    （`tools/asr/transcribe.py`，子进程契约）；`echo-cli transcribe` 导入→转写→
    transcript 证据入库（含分段时间戳，`say` 语音实测文本完全正确）。
  - 待办：SenseVoice（需 `convert.py` 一次性转换后接入）、ForcedAligner 词级对齐、
    waveform↔transcript 双向同步、桌面端"分析"入口。
- **M2 Library**：自然语言搜索、人物/声音、时间、audio event、CLAP semantic search。
- **M3 Restore**：非破坏性 effect graph、EQ、loudness、DeepFilterNet、A/B Original。
- **M4 Audio Space**：声音相册：时间、人物、地点、声音类型、Revisit。
- **M5 Memory Contract**：只读 memory/render API 向上层开放（echo://asset/{uuid} 契约族；Shadow/Video 同契约，各自实现）。

## 10. 垂直切片（判断 Echo 是否成立的标准）

```text
导入一段真实录音
→ waveform
→ Qwen3-ASR transcript + timestamp
→ SenseVoice emotion/event
→ 点击文字定位声音
→ 搜索一句自然语言
→ 找到并播放真实片段
```

## 11. 工程约定

- **平台**：先跑通 macOS（Apple Silicon）；架构上不为 Windows 设障碍，迁移成本应可控（Qt/C++/Rust 均可移植，MLX 只存在于 AI 层并被 InferenceBackend 隔离）。
- **License**：MIT。
- **测试拓扑**：私有不变量测试紧邻 owner（`<owner>/tests.rs`，owner 文件以 `#[cfg(test)] mod tests;` 收尾）；跨模块契约在 crate facade 的 `src/tests/<responsibility>_contract.rs`；crate 级 `tests/` 只放消费公开 API 的黑盒契约。
- **格式**：Rust 用仓库 `rustfmt.toml`；C++/ObjC++ 用 `.clang-format`。
- **i18n**：英文原文为 canonical 消息身份，简体中文必须是完整产品呈现（与 Shadow 相同契约），技术 token 不翻译。
- **构建产物**：Cargo target / CMake build 目录放在仓库外的 `.echo-local-*`，不污染 worktree。
- **提交**：按里程碑自主提交；每次提交应可构建。
