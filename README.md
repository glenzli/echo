# Echo

[中文](#中文) · [English](#english)

---

<a id="中文"></a>

## 中文

> **开发预览。** Echo 仍在快速迭代，功能、交互和持久化格式都可能调整。建议使用独立测试声音库体验，并为重要录音保留备份。

Echo 是一款 local-first 的声音资料库与非破坏性声音调整应用。它保存、理解、整理、修复并帮助人重新聆听真实发生过的声音；Library 是一等公民，Editor 是 Library 的能力。

Echo 与 [Shadow](../shadow) 属于同一系列：Shadow 面向照片与 RAW，Echo 面向声音与录音。长期方向是在用户控制下，把原始录音、可追溯的机器理解、聆听历史和用户校准组织成声音记忆。当前开发版主要提供声音资料库、渐进式本地理解，以及确定性的非破坏性处理与交付。

### 设计方向

- **先聆听，后编辑**：Audio Space 以声音墙、相册和重温为入口；时间轴不是应用的默认首页。
- **Original 保持不变**：调整以版本保存，缓存可以重建，导出记录所使用的来源与处理版本。
- **Analysis 不是事实**：文字、事件、情绪、地点和语义近邻都保留模型与执行来源；用户校准独立于模型证据。
- **本地智能**：后台理解通过本地 Infer Runtime 执行，不进入实时音频回调，也不阻断资料库浏览和播放。
- **创作边界明确**：Creative VFX 只处理用户显式选择的既有录音，默认可旁路且不覆盖 Original；语音生成、换声、改词和从零生成声音不属于当前 Echo。

### 当前可体验

- **资料库与重温**：导入本地文件夹，通过声音墙、波形、胶片带、搜索、复合筛选、Like、评分和声音相册整理录音；继续聆听、往年今日、最近聆听和新内容由可追溯的本地状态生成。
- **声音理解**：导入后在后台渐进提取文字、时间对齐、声音事件与情境信息，并允许用户校准展示结果；精确文字检索、基于证据的自然语言检索，以及面向短、无文字录音的有限 CLAP 检索彼此保留独立证据空间。
- **非破坏性调整**：独立的声音调整工作区提供裁剪、淡入淡出、增益、EQ、Dynamics、响度测量、录制缺陷修复、空间处理、效果顺序与局部作用范围，并支持 Original／Adjusted A/B、撤销重做和显式保存版本。
- **确定性 Creative VFX**：Scene、Delay、Modulation、Tape、Pitch、Freeze、Granular 等处理与恢复性调整分域，可独立启用、旁路和保存，不把生成内容伪装成原录音。
- **方案与交付**：可保存和应用处理方案，将单条录音导出为 WAV，或将有界批次导出为 WAV／FLAC；导出与频谱修复工作副本保留来源版本和处理记录。

### 当前边界

- Echo 目前是面向 macOS Apple Silicon 的开发预览；Catalog、调整图和交互格式仍可能变化。
- 本地智能能力需要已配置且具备相应能力的 Infer Runtime。Runtime 不可用时，导入、浏览、波形、播放和既有 DSP 调整仍可使用。
- 人物、地点、情绪和声音类型目前主要是可追溯的模型证据或提示，不代表完整的人物关系、地理关系或事实确认系统。
- CLAP 原声音检索当前只覆盖有界的短录音切片；长录音的完整分段语义检索仍在建设中。
- 神经降噪、源分离和生成式声音能力不是当前开发版已完成的产品功能；公共仓库也不分发模型文件。

### 运行开发版

已有准备好的本地开发环境时，可启动当前 canonical debug：

```sh
./scripts/run_debug.sh
```

构建、验证与 Debug 提升流程见[桌面应用开发说明](apps/desktop/README.md)。

### 项目导航

- [产品设计与里程碑](ROADMAP.md)
- [桌面应用与交互边界](apps/desktop/README.md)
- [Rust／C++ 音频桥](crates/echo-bridge/README.md)
- [桌面服务边界](crates/echo-desktop-bridge/README.md)
- [第三方来源与许可](THIRD_PARTY_NOTICES.md)

---

<a id="english"></a>

## English

> **Development preview.** Echo is evolving quickly, and features, interactions, and persistent formats may change. Use a separate test Library and keep backups of important recordings.

Echo is a local-first sound Library and non-destructive sound-adjustment application. It preserves, understands, organizes, restores, and helps people revisit sounds that actually happened. The Library is the primary product; the editor is one of its capabilities.

Echo is the audio sibling of [Shadow](../shadow): Shadow works with photographs and RAW files, while Echo works with sound and recordings. The long-term direction is to organize immutable recordings, traceable machine understanding, listening history, and user calibration into a sound-memory system under the user's control. The current build focuses on the sound Library, progressive local understanding, and deterministic non-destructive processing and delivery.

### Design direction

- **Listen first, edit second**: Audio Space starts with the Sound Wall, albums, and Revisit. A timeline is not the application's home screen.
- **Keep the Original unchanged**: adjustments are saved as revisions, caches are rebuildable, and exports record the source and processing revision they used.
- **Analysis is not fact**: text, events, emotion, location, and semantic neighbors retain model and execution provenance. User calibration remains separate from model evidence.
- **Local intelligence**: background understanding runs through the local Infer Runtime, stays outside the real-time audio callback, and never blocks Library browsing or playback.
- **Explicit creative boundary**: Creative VFX process only recordings the user selected, remain bypassable, and never overwrite the Original. Speech generation, voice conversion, rewriting words, and sound generation from scratch are outside the current Echo product.

### Available in the current build

- **Library and Revisit**: import local folders and organize recordings through the Sound Wall, waveforms, filmstrip, search, compound filters, Likes, ratings, and sound albums. Continue Listening, On This Day, Recently Played, and new additions are derived from traceable local state.
- **Sound understanding**: progressively extract text, alignment, sound events, and contextual information in the background, with user calibration over the displayed result. Exact text retrieval, evidence-based natural-language retrieval, and limited CLAP retrieval for short recordings without text remain separate evidence spaces.
- **Non-destructive adjustment**: a dedicated workspace provides trim, fades, gain, EQ, dynamics, loudness measurement, recording repair, space processing, effect ordering, and bounded effect regions, with Original/Adjusted A/B, undo/redo, and explicit version saving.
- **Deterministic Creative VFX**: Scene, Delay, Modulation, Tape, Pitch, Freeze, Granular, and related processing remain separate from restoration. Each can be enabled, bypassed, and saved without presenting generated material as the original recording.
- **Recipes and delivery**: save and apply processing recipes, then export one recording as WAV or a bounded batch as WAV/FLAC. Exports and rendered spectral-repair working copies retain their source revision and processing record.

### Current boundaries

- Echo is currently a development preview for macOS on Apple Silicon. Catalog, adjustment-graph, and interaction formats may continue to change.
- Local intelligent features require a configured Infer Runtime with the relevant capabilities. Import, browsing, waveforms, playback, and existing DSP adjustments remain usable when the Runtime is unavailable.
- People, place, emotion, and sound-type projections are currently traceable model evidence or hints, not a complete identity, relationship, or fact-confirmation system.
- Raw-audio CLAP retrieval currently covers a bounded short-recording slice. Complete segment-level semantic retrieval for long recordings is still under development.
- Neural denoise, source separation, and generative audio are not completed product features in the current build. The public repository does not distribute model files.

### Run the development build

With an existing local development setup, launch the current canonical debug build:

```sh
./scripts/run_debug.sh
```

See the [desktop application guide](apps/desktop/README.md) for build, validation, and Debug promotion details.

### Project navigation

- [Product design and milestones](ROADMAP.md)
- [Desktop application and interaction boundaries](apps/desktop/README.md)
- [Rust/C++ audio bridge](crates/echo-bridge/README.md)
- [Desktop service boundary](crates/echo-desktop-bridge/README.md)
- [Third-party provenance and licenses](THIRD_PARTY_NOTICES.md)

## License

Echo is free software under the MIT License. See [LICENSE](LICENSE).
