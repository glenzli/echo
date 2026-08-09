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
 waveform                inference intent
        │                    │
        └──────────┬─────────┘
                   │
       Infer Build local control plane
 admission / policy / scheduler / deployment
                   │
       MLX / ONNX / whisper.cpp / Cloud
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
- inference intent submission and result ingestion

### Echo 与 Infer Build 的边界

Echo 是产品和声音记忆的 owner；Infer Build 是共享的本地推理控制面。生产链路按以下边界
演进，Echo 不再自行发展一套模型调度器：

| Echo 负责 | Infer Build 负责 |
| --- | --- |
| `AudioAsset`、Original、provenance、分析证据与用户确认 | admission、App policy、router/scheduler、quota、resource reservation |
| 决定何时提交 `audio.transcribe`、`audio.align`、`text.summarize` 等 Intent Job | 把 Intent 路由到具体模型、worker 和硬件后端 |
| 保存可追溯结果；模型升级后决定是否重算 | 模型路径、Python/worker 生命周期、部署状态和失败重试 |
| 进度、取消和失败状态的产品呈现 | Job/Attempt 持久化、资源仲裁和运行审计 |

仓库内现有 MLX/Ollama/脚本调用只作为概念验证和兼容适配层：允许验证数据契约，但不再
扩充模型选择 UI、物理模型路由、下载器或驻留进程管理。正式接入从一个真实消费方开始，
优先采用 Infer Build 已有的 `audio.transcribe` / `audio.align` 任务接口；不预建没有消费方的
第二套调度机制。

## 4. 音频底层

- 不自己重写 codec，直接用 FFmpeg / libavfilter。
- 内部统一 canonical PCM：**float32、48 kHz、channel-preserving**。真正输出时再编码。原始文件永远不动。

## 5. 数据模型：AudioAsset

```text
AudioAsset
├── Original        content hash / path / codec / timestamp / metadata（immutable）
├── Analysis        transcript / speakers / emotions / audio events / embeddings / segments
├── UserState       liked / rating / album membership（用户事实）
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

首次 import 很快可浏览；ASR 作为默认的后台派生元数据持久入队，但绝不阻塞导入、浏览或
播放。其余理解层仍由能力、资源和用户需要渐进触发；正式接入后由 Infer Build 负责准入、
空闲调度和资源仲裁，而不是让 Echo 在导入事务里同步运行模型。

## 7. 推理能力与模型参考

产品层只依赖能力与证据契约，不依赖某个物理模型。下表是当前验证用候选，不构成 Echo
自己的模型注册或调度路线；模型获取、部署和硬件路由最终由 Infer Build 负责。

| 能力 | 模型 | 说明 |
| --- | --- | --- |
| 主 ASR（说了什么） | Qwen3-ASR 0.6B/1.7B | 52 种语言方言；`mlx-community/Qwen3-ASR-1.7B` |
| 时间对齐 | Qwen3-ForcedAligner 0.6B | word/sentence timestamp，11 种语言 |
| 发生了什么 | SenseVoiceSmall | ASR + language + emotion + audio events（中文/粤语/英文/日/韩，BGM/掌声/笑声/哭声/咳嗽） |
| 谁在讲话 | FunASR：VAD + CAM++ speaker model + 标点 | speaker diarization；用户确认 Speaker A=我、B=孩子 |
| 稳定 baseline | whisper.cpp | minimal install 的转录能力；Core ML encoder 可放 ANE；所有 Python/MLX 模型坏了 Echo 仍完整 |
| 声音检索 | CLAP | audio↔text 同一 embedding space；"找火车声"不依赖 transcript；Transcript embedding + CLAP audio embedding 双索引 |

Effect 策略：传统 DSP（gain/loudness/EQ/filter/compressor/limiter/fade/resample/normalize）CPU 即可，甚至 DeepFilterNet 级别降噪也是低复杂度实时可跑。GPU 只用于 heavy speech enhancement、source separation、de-reverb、neural restoration、大 ASR、audio embedding batch。

过渡期的 `InferenceBackend` 抽象：

```text
InferenceBackend
├── MLX / CoreML / CUDA / ONNX Runtime / whisper.cpp / Cloud
```

业务层不感知底层 GPU。effect graph CPU-first，AI Effect 提交推理 Intent。过渡适配器不得
越过该边界把模型路径、虚拟环境或 worker 生命周期渗入 catalog、core 或 UI。

## 8. UI 原则

- 首屏是 **声音墙**，不是 Timeline 或单条录音的大波形。左侧是资料库、系统集合和声音相册，
  中间用横向声景卡片并行浏览，右侧是所选声音的详情检查器；双击或“展开”才进入完整播放工作区。
- 声景卡片必须来自真实证据：缓存 waveform、文件元数据和可追溯 AI 分析。AI 可生成可修改的
  标题、文字预览和事件标签，但不得用虚构图片冒充录音内容。
- 用户层产品语言称 ASR 结果为“文字”；`transcript` 只保留在模型能力、数据类型、协议和证据
  来源等技术语境中。点击文字仍可定位声音。
- Like、评分与相册归用户事实，不是 AI Analysis；文件内嵌时间、地点和标签归 Original 的来源
  元数据，必须标注来源，和模型推断严格区分。
- 单击 asset：右侧快速检查 Waveform / 文字 / Events / People / Metadata；双击 asset：进入
  Waveform / 文字 / Events / People / Adjustment 完整工作区。
- 桌面窗口只保留一条与 Shadow 同构的融合标题工具栏：品牌、工作区导航、当前工作区工具、
  设置与原生窗口拖动共享一个 chrome owner；内容区域不得重复绘制第二条伪标题栏。
- 组件命名延续系列设计语言：`EchoButton`、`EchoIcon`……与 Shadow 的 `Shadow*` 组件对应，视觉 token 一致（Shadow/Echo 同系列）。

## 9. 里程碑

- **M0 Audio Foundation**：Qt 播放、FFmpeg decode、waveform、SQLite、immutable Original、后台 job。
  - 已完成：FFmpeg probe/流式解码、waveform pyramid、Qt 回调式播放
    （seek/pause/volume）、SQLite catalog、immutable Original、目录扫描、可恢复 job queue、
    Audio Space 列表与波形详情。
  - 已收口（2026-08-09）：waveform artifact 由 catalog 引用并复用；扫描任务身份包含文件指纹，
    同一路径变更后可重新导入；后台导入只做 Level 0，不再自动跑完整模型链；后台结果在
    已打开的 Audio Space 中可见；修复失败任务读取和桌面异步生命周期。
  - 退出标准：导入后快速可浏览、cache 可删可重建、重复扫描幂等、文件变化可重入、播放
    与 UI 不依赖推理服务在线。
- **M1 Understand**：Qwen3-ASR + forced alignment + SenseVoice，waveform ↔ transcript 双向同步。
  - 已验证（概念阶段）：本地 MLX ASR 子进程契约；`echo-cli transcribe` 的
    导入→转写→证据入库；分段时间戳；桌面端手动分析入口。
  - 已接入（过渡切片，2026-08-09）：Echo 以 Infer Build 的 `audio.transcribe` logical intent、
    标准请求字段和 `infer.*` constraints 形成请求；当前执行端是 JSON-lines 兼容的直接 MLX
    adapter，只负责把请求硬映射到本地 Qwen3-ASR，不承担路由、准入、重试或资源生命周期。
  - 默认分析策略（2026-08-09 校准）：新录音完成注册后持久入队 waveform 与
    `audio.transcribe`；应用启动时为已有但缺少 transcript 证据的在线录音做幂等回填。
    结构性扫描、导入和 waveform 优先于 ASR；ASR 失败不得影响 Original、播放或浏览。
    手动分析只作为失败重试/调试入口，不是正常产品路径。该默认不扩张到 SenseVoice、
    diarization、embedding、LLM contextual understanding 或 TTS。
  - 冻结项：不再增强 Echo 内的物理模型注册、Ollama worker 或裸 Python 路由。
  - 下一真实切片：用 Infer Build 的正式 HTTP/Job 执行替换直接 adapter，接收 Job/Attempt
    进度和结果，保存模型/版本/置信度/时间戳；随后接 `audio.align`，完成
    waveform↔transcript 双向定位。
  - 待办：SenseVoice 能力 Intent、speaker/event 证据、取消/重试产品状态。
- **M2 Library**：声音墙、声音相册、Like/评分、来源元数据筛选、自然语言搜索、人物/声音、
  时间、audio event、CLAP semantic search。
  - 首个浏览切片（2026-08-09）：声音墙成为默认首屏；资料库、横向声景卡片和详情检查器拆分
    为独立 UI owner；现有大波形页面降为展开层。用户 Like/评分持久化，卡片使用真实 waveform、
    文件时间/格式和已有 AI 文字证据；FFmpeg probe 开始保存容器、采样率、声道和嵌入标签；
    窗口 chrome 收敛为与 Shadow 同构、可拖动的单一融合标题工具栏。
- **M3 Restore**：非破坏性 effect graph、EQ、loudness、DeepFilterNet、A/B Original。
- **M4 Audio Space**：声音相册：时间、人物、地点、声音类型、Revisit。
- **M5 Memory Contract**：只读 memory/render API 向上层开放（echo://asset/{uuid} 契约族；Shadow/Video 同契约，各自实现）。

### 当前状态校准（2026-08-09）

Echo 处于 **M0 收口、M1 默认 Intent admission 已开始但正式 Infer Build 执行尚未接入**
的阶段。Audio Space 已经形成首个可用垂直界面，但人物、地点、声音类型仍是展示维度，
不应被描述为已经具备完整识别和关系系统。M4 表示声音相册体验成熟，而不是首次出现
Audio Space 页面。

在 M1 的 Infer Build 任务切片完成前，不把直接模型调用的数量当作里程碑进度；在 M2 前，
不把 transcript 包含匹配描述成语义搜索；在 M3 前，不在实时播放路径加入任何 AI effect。

## 10. 垂直切片（判断 Echo 是否成立的标准）

```text
导入一段真实录音
→ waveform
→ Qwen3-ASR 文字 + timestamp
→ SenseVoice emotion/event
→ 声音墙并行浏览、点击文字定位声音
→ 搜索一句自然语言
→ 找到并播放真实片段
```

## 11. 工程约定

- **平台**：先跑通 macOS（Apple Silicon）；架构上不为 Windows 设障碍，迁移成本应可控（Qt/C++/Rust 均可移植，MLX 只存在于 AI 层并被 InferenceBackend 隔离）。
- **License**：MIT。
- **测试拓扑**：私有不变量测试紧邻 owner（`<owner>/tests.rs`，owner 文件以 `#[cfg(test)] mod tests;` 收尾）；跨模块契约在 crate facade 的 `src/tests/<responsibility>_contract.rs`；crate 级 `tests/` 只放消费公开 API 的黑盒契约。
- **Catalog schema**：唯一规范格式为 `YYYYMMDD.N`（日期.当天版本号，例如 `20260809.3`）；catalog 打开当前 revision，或将明确支持的紧邻前序 revision 原子迁移到当前版本；不把无点整数编码暴露为产品或持久化身份。
- **格式**：Rust 用仓库 `rustfmt.toml`；C++/ObjC++ 用 `.clang-format`。
- **i18n**：英文原文为 canonical 消息身份，简体中文必须是完整产品呈现（与 Shadow 相同契约），技术 token 不翻译。
- **构建产物**：Cargo target / CMake build 目录放在仓库外的 `.echo-local-*`，不污染 worktree。
- **提交**：按里程碑自主提交；每次提交应可构建。
