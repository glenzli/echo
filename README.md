# Echo

[中文](#中文) · [English](#english)

---

<a id="中文"></a>

## 中文

> **开发预览。** Echo 仍在快速迭代，功能、交互和持久化格式都可能调整。建议使用独立测试声音库体验，并为重要录音保留备份。

Echo 是一款 local-first 的声音记忆与非破坏性编辑应用。记忆库保存想留下的原录音、处理版本和多轨作品；素材提供配乐、环境声与音效。每个作品保留原始来源和准确的处理版本。

Echo 与 [Shadow](../shadow) 属于同一系列：Shadow 面向照片与 RAW，Echo 面向声音与录音。长期方向是在用户控制下，把原始录音、可追溯的机器理解、聆听历史和用户校准组织成声音记忆。当前开发版提供记忆库、全局与项目素材、渐进式本地理解，以及非破坏性处理和多轨编排。

### 设计方向

- **先聆听，后编辑**：Audio Space 以声音墙、相册和重温为入口；时间轴不是应用的默认首页。
- **Original 保持不变**：调整以版本保存，缓存可以重建，导出记录所使用的来源与处理版本。
- **Analysis 不是事实**：文字、事件、情绪、地点和语义近邻都保留模型与执行来源；用户校准独立于模型证据。
- **本地智能**：后台理解通过本地 Infer Runtime 执行，不进入实时音频回调，也不阻断资料库浏览和播放。
- **创作边界明确**：Creative VFX 只处理用户显式选择的既有录音，默认可旁路且不覆盖 Original；语音生成、换声、改词和从零生成声音不属于当前 Echo。
- **编排保持可追溯**：每个片段标明记忆或素材用途，并固定 Original 与处理版本；项目内精细编辑只更新该片段。收进记忆库时固定一个已完成的混音版本，后续项目编辑不会自动替换它。

### 当前可体验

- **记忆库与重温**：导入本地文件夹，通过声音墙、声音带、单音详情、搜索、复合筛选、Like、评分和声音相册整理录音；磁带可切换记忆、素材或原录音，将当前结果虚拟首尾相接以连续试听，不生成新的拼接文件；继续聆听、往年今日、最近聆听和新内容由可追溯的本地状态生成。
- **素材**：编辑器内提供本项目、记忆库和素材入口，可搜索、独立试听并加入轨道。导入文件保存为 Catalog 旁的持久副本，可只归项目或保留为全局素材；独立素材页提供类别与已有 AI 声音事件筛选。
- **声音理解**：导入后在后台渐进提取文字、时间对齐、声音事件与情境信息，并允许用户校准展示结果；精确文字检索、基于证据的自然语言检索，以及面向短、无文字录音的有限 CLAP 检索彼此保留独立证据空间。
- **非破坏性调整**：独立的声音调整工作区提供裁剪、淡入淡出、增益、EQ、Dynamics、响度测量、录制缺陷修复、空间处理、效果顺序与局部作用范围，并支持 Original／Adjusted A/B、撤销重做和显式保存版本。
- **确定性 Creative VFX**：Scene、Delay、Modulation、Tape、Pitch、Freeze、Granular 等处理与恢复性调整分域，可独立启用、旁路和保存，不把生成内容伪装成原录音。
- **声音编排**：可从资料库多选按顺序或分层创建独立编排，在最多 8 条轨道、256 个片段和 4 小时范围内移动、跨轨、分割、裁剪、复制、淡化、叠加和混合；轨道提供增益、声像、Mute／Solo，Master 提供增益与 limiter，并支持撤销重做、保存不可变版本、试听和 PCM24 WAV 混音导出。
- **方案与交付**：可保存和应用处理方案，将单条录音导出为 WAV，或将有界批次导出为 WAV／FLAC；导出与频谱修复工作副本保留来源版本和处理记录。

### 当前边界

- Echo 目前是面向 macOS Apple Silicon 的开发预览；Catalog、调整图和交互格式仍可能变化。
- 本地智能能力需要已配置且具备相应能力的 Infer Runtime。Runtime 不可用时，导入、浏览、波形、播放和既有 DSP 调整仍可使用。
- 人物、地点、情绪和声音类型目前主要是可追溯的模型证据或提示，不代表完整的人物关系、地理关系或事实确认系统。
- CLAP 原声音检索当前只覆盖有界的短录音切片；长录音的完整分段语义检索仍在建设中。
- 神经降噪、源分离和生成式声音能力不是当前开发版已完成的产品功能；公共仓库也不分发模型文件。
- 已保存混音可从记忆库重新打开来源项目；当前不将一个混音项目嵌套为另一个项目的片段。项目内精细编辑使用原始频谱修复，渲染后工作副本仍属于单音处理入口。
- 声音编排不是通用 DAW：当前不提供录音、输入监听、MIDI、速度网格、时间拉伸、插件宿主、发送总线、任意路由、自动化或视频同步。

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

Echo is a local-first sound-memory and non-destructive editing application. The memory library holds original recordings, adjusted versions, and multitrack works the user wants to keep. Materials supply music, ambience, and effects. Each work retains its original sources and exact processing revisions.

Echo is the audio sibling of [Shadow](../shadow): Shadow works with photographs and RAW files, while Echo works with sound and recordings. The long-term direction is to organize immutable recordings, traceable machine understanding, listening history, and user calibration into a sound-memory system under the user's control. The current build provides a memory library, global and project materials, progressive local understanding, non-destructive processing, and multitrack arrangement.

### Design direction

- **Listen first, edit second**: Audio Space starts with the Sound Wall, albums, and Revisit. A timeline is not the application's home screen.
- **Keep the Original unchanged**: adjustments are saved as revisions, caches are rebuildable, and exports record the source and processing revision they used.
- **Analysis is not fact**: text, events, emotion, location, and semantic neighbors retain model and execution provenance. User calibration remains separate from model evidence.
- **Local intelligence**: background understanding runs through the local Infer Runtime, stays outside the real-time audio callback, and never blocks Library browsing or playback.
- **Explicit creative boundary**: Creative VFX process only recordings the user selected, remain bypassable, and never overwrite the Original. Speech generation, voice conversion, rewriting words, and sound generation from scratch are outside the current Echo product.
- **Traceable assembly**: each clip identifies its memory or material role and pins an Original and processing revision. Precision editing inside a project changes only that clip. Keeping a mix in memory pins a completed listening edition; subsequent project edits do not replace it.

### Available in the current build

- **Memory library and Revisit**: import local folders and organize recordings through the Sound Wall, Sound Tape, single-sound detail, search, compound filters, Likes, ratings, and sound albums. Sound Tape switches between memories, materials, and originals and virtually joins the results for continuous listening without creating a concatenated file. Continue Listening, On This Day, Recently Played, and new additions are derived from traceable local state.
- **Materials**: the editor contains Project, Memories, and Materials bins with search, independent audition, and track placement. Imported files are durable copies beside the Catalog and can stay project-only or enter global materials. A separate material page filters by user category and existing AI sound events.
- **Sound understanding**: progressively extract text, alignment, sound events, and contextual information in the background, with user calibration over the displayed result. Exact text retrieval, evidence-based natural-language retrieval, and limited CLAP retrieval for short recordings without text remain separate evidence spaces.
- **Non-destructive adjustment**: a dedicated workspace provides trim, fades, gain, EQ, dynamics, loudness measurement, recording repair, space processing, effect ordering, and bounded effect regions, with Original/Adjusted A/B, undo/redo, and explicit version saving.
- **Deterministic Creative VFX**: Scene, Delay, Modulation, Tape, Pitch, Freeze, Granular, and related processing remain separate from restoration. Each can be enabled, bypassed, and saved without presenting generated material as the original recording.
- **Sound Assembly**: create a sequence or layered assembly from a Library selection, then move, re-track, split, trim, duplicate, fade, overlap, and mix up to 256 clips across 8 tracks and 4 hours. Track gain, pan, mute/solo, master gain and limiting, undo/redo, immutable version saves, preview, and PCM24 WAV mixdown are included.
- **Recipes and delivery**: save and apply processing recipes, then export one recording as WAV or a bounded batch as WAV/FLAC. Exports and rendered spectral-repair working copies retain their source revision and processing record.

### Current boundaries

- Echo is currently a development preview for macOS on Apple Silicon. Catalog, adjustment-graph, and interaction formats may continue to change.
- Local intelligent features require a configured Infer Runtime with the relevant capabilities. Import, browsing, waveforms, playback, and existing DSP adjustments remain usable when the Runtime is unavailable.
- People, place, emotion, and sound-type projections are currently traceable model evidence or hints, not a complete identity, relationship, or fact-confirmation system.
- Raw-audio CLAP retrieval currently covers a bounded short-recording slice. Complete segment-level semantic retrieval for long recordings is still under development.
- Neural denoise, source separation, and generative audio are not completed product features in the current build. The public repository does not distribute model files.
- Saved mixes reopen their source project; nesting a mix project inside another project is not supported. Project clip editing supports original spectral repair; post-render working copies remain in the single-sound workspace.
- Sound Assembly is not a general-purpose DAW. Recording, input monitoring, MIDI, tempo grids, time stretching, plug-in hosting, sends, arbitrary routing, automation, and video sync are outside the current module.

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
