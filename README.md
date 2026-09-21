# Echo

[中文](#中文) · [English](#english)

---

<a id="中文"></a>

## 中文

> **开发预览。** Echo 仍在快速迭代，功能、交互和持久化格式都可能调整。建议使用独立测试声音库体验，并为重要录音保留备份。

Echo 是一款 local-first 的声音记忆与非破坏性编辑应用。记忆库保存想留下的原录音、处理版本和多轨作品；素材提供配乐、环境声与音效。每个作品保留原始来源和准确的处理版本。

Echo 与 [Shadow](../shadow) 属于同一系列：Shadow 面向照片与 RAW，Echo 面向声音与录音。长期方向是在用户控制下，把原始录音、可追溯的机器理解、聆听历史和用户校准组织成声音记忆。当前开发版提供记忆库、全局与项目素材、渐进式本地理解，以及非破坏性处理和多轨编排。

### 设计方向

- **先聆听，后编辑**：Audio Space 以声音墙、声音集和重温为入口；时间轴不是应用的默认首页。
- **Original 保持不变**：调整以版本保存，缓存可以重建，导出记录所使用的来源与处理版本。
- **Analysis 不是事实**：文字、事件、情绪、地点和语义近邻都保留模型与执行来源；用户校准独立于模型证据。
- **本地智能**：后台理解通过本地 Infer Runtime 执行，不进入实时音频回调，也不阻断资料库浏览和播放。
- **创作边界明确**：Creative VFX 只处理用户显式选择的既有录音，默认可旁路且不覆盖 Original；生成补充采用显式采纳、来源披露和重温筛选的方向，当前支持短旁白候选生成、分别试听和显式采纳，生成身份保留。
- **编排保持可追溯**：每个片段标明记忆或素材用途，并固定 Original 与处理版本；项目内精细编辑只更新该片段。收进记忆库时固定一个已完成的混音版本，后续项目编辑不会自动替换它。

### 当前可体验

- **独立编辑**：直接打开音频，使用单音处理或多轨编排，保存包含素材的 `.echo` 工程或导出音频；此入口使用隔离会话，不启动记忆库维护和自动分析。`./scripts/run_debug.sh --edit` 可启动。

- **记忆库与重温**：导入本地文件夹，通过声音墙、声音带、单音详情、搜索、复合筛选、Like、评分和声音集整理录音；磁带可切换记忆、素材或原录音，将当前结果虚拟首尾相接以连续试听，不生成新的拼接文件；继续聆听、往年今日、最近聆听和新内容由可追溯的本地状态生成。
- **来源标记**：可给原始时间选区或整份来源声明 AI 处理、生成补充或对白重建，并保留修改历史。标记随工程和可听的编排引用传递；磁带默认排除已标记的生成来源，也可显式包含。未标记不代表已验证实录。
- **素材**：编辑器内提供本项目、记忆库和素材入口，可搜索、独立试听并加入轨道。导入文件保存为 Catalog 旁的持久副本，可只归项目或保留为全局素材；独立素材页提供类别与已有 AI 声音事件筛选。
- **声音理解**：导入后在后台渐进提取文字、时间对齐、声音事件与情境信息，并允许用户校准展示结果；精确文字检索、基于证据的自然语言检索，以及面向短、无文字录音的有限 CLAP 检索彼此保留独立证据空间。
- **非破坏性调整**：独立的声音调整工作区提供裁剪、淡入淡出、增益、EQ、Dynamics、响度测量、录制缺陷修复、空间处理、效果顺序与局部作用范围，并支持 Original／Adjusted A/B、撤销重做和显式保存版本。
- **确定性 Creative VFX**：Scene、Delay、Modulation、Tape、Pitch、Freeze、Granular 等处理与恢复性调整分域，可独立启用、旁路和保存，不把生成内容伪装成原录音。
- **编辑中的 AI**：显式转写最多五分钟的原始音频选区，按句段或可用的对齐词段定位、试听、隐藏或仅保留音频，保留边界余量与撤销。可复制完整或勾选的转写，或导出 TXT、SRT、WebVTT；字幕使用原录音时间，没有时间信息时仍可导出文字。独立模式只把结果保存在当前工程。素材搜索先显示字面结果，再补充已有索引的语义候选。
- **声音编排**：可从资料库多选按顺序或分层创建独立编排，在最多 8 条轨道、256 个片段和 4 小时范围内移动、跨轨、分割、裁剪、复制、淡化、叠加和混合；轨道提供增益、声像、Mute／Solo，Master 提供增益与 limiter，并支持撤销重做、保存不可变版本、试听和多格式混音导出。片段音量包络可编辑和旁路，也可根据参考轨峰值活动生成配乐闪避曲线；来源处理可复用，并提供所选片段范围的循环试听。来源准备后，混音按块生成进入短缓冲，无需等待整段混音文件；增益、声像和 Mute／Solo 等混音控制可在试听中生效；音频结构或效果链变化需要重新准备。
- **方案与交付**：可保存和应用处理方案，以 WAV、FLAC、MP3 或 M4A 导出单条录音或有界批次；导出与频谱修复工作副本保留来源版本和处理记录。
- **精确定位**：单音时间轴可输入秒数或时分秒，建立毫秒级选区；多轨可直接跳到指定时间。选择范围只调整视图，不改音频。
- **批量编排**：Cmd／Ctrl／Shift 点选片段，整体移动、跨轨、复制、分割或删除；波纹删除可作用于所选轨道或全部轨道，保留跨越删除区间的声音首尾。Alt 拖动只滑移片段内部的来源，批量操作可以一步撤销。

### 当前边界

- Echo 目前是面向 macOS Apple Silicon 的开发预览；Catalog、调整图和交互格式仍可能变化。
- 本地智能能力需要已配置且具备相应能力的 Infer Runtime。Runtime 不可用时，导入、浏览、波形、播放和既有 DSP 调整仍可使用。
- 人物、地点、情绪和声音类型目前主要是可追溯的模型证据或提示，不代表完整的人物关系、地理关系或事实确认系统。
- CLAP 原声音检索当前只覆盖有界的短录音切片；长录音的完整分段语义检索仍在建设中。
- 神经降噪、源分离、环境声生成与音频延长尚未完成；短旁白生成已提供显式候选流程。公共仓库也不分发模型文件。来源标记来自用户声明或音频文件携带的声明，编排按整份来源保守披露。WAV／FLAC 导出嵌入精简来源类别，重新导入会恢复标记；声明未经认证，外部转换可能移除元数据。私人说明和路径仍留在 Echo。
- 已保存混音可从记忆库重新打开来源项目；当前不将一个混音项目嵌套为另一个项目的片段。项目内精细编辑使用原始频谱修复，渲染后工作副本仍属于单音处理入口。
- 声音编排不是通用 DAW：当前不提供录音、输入监听、MIDI、速度网格、时间拉伸、插件宿主、发送总线、任意路由、轨道参数自动化或视频同步。

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

- **Listen first, edit second**: Audio Space starts with the Sound Wall, collections, and Revisit. A timeline is not the application's home screen.
- **Keep the Original unchanged**: adjustments are saved as revisions, caches are rebuildable, and exports record the source and processing revision they used.
- **Analysis is not fact**: text, events, emotion, location, and semantic neighbors retain model and execution provenance. User calibration remains separate from model evidence.
- **Local intelligence**: background understanding runs through the local Infer Runtime, stays outside the real-time audio callback, and never blocks Library browsing or playback.
- **Explicit creative boundary**: Creative VFX process only recordings the user selected, remain bypassable, and never overwrite the Original. Generated additions require explicit acceptance, source disclosure, and revisit controls. Short narration candidates can be generated, auditioned separately, and explicitly accepted with their generation identity retained.
- **Traceable assembly**: each clip identifies its memory or material role and pins an Original and processing revision. Precision editing inside a project changes only that clip. Keeping a mix in memory pins a completed listening edition; subsequent project edits do not replace it.

### Available in the current build

- **Independent editing**: open audio directly, use the shared single-source and multitrack editors, save a portable `.echo` project with its sources, or export audio. This session starts no library maintenance or automatic analysis. Launch with `./scripts/run_debug.sh --edit`.

- **Memory library and Revisit**: import local folders and organize recordings through the Sound Wall, Sound Tape, single-sound detail, search, compound filters, Likes, ratings, and sound collections. Sound Tape switches between memories, materials, and originals and virtually joins the results for continuous listening without creating a concatenated file. Continue Listening, On This Day, Recently Played, and new additions are derived from traceable local state.
- **Source labels**: Declare AI processing, generated additions, or reconstructed speech for Original-time intervals or entire sources, with correction history. Labels travel with projects and audible arrangement references. Tape excludes declared generated sources by default and offers explicit inclusion. Unmarked sources are not verified recordings.
- **Materials**: the editor contains Project, Memories, and Materials bins with search, independent audition, and track placement. Imported files are durable copies beside the Catalog and can stay project-only or enter global materials. A separate material page filters by user category and existing AI sound events.
- **Sound understanding**: progressively extract text, alignment, sound events, and contextual information in the background, with user calibration over the displayed result. Exact text retrieval, evidence-based natural-language retrieval, and limited CLAP retrieval for short recordings without text remain separate evidence spaces.
- **Non-destructive adjustment**: a dedicated workspace provides trim, fades, gain, EQ, dynamics, loudness measurement, recording repair, space processing, effect ordering, and bounded effect regions, with Original/Adjusted A/B, undo/redo, and explicit version saving.
- **Deterministic Creative VFX**: Scene, Delay, Modulation, Tape, Pitch, Freeze, Granular, and related processing remain separate from restoration. Each can be enabled, bypassed, and saved without presenting generated material as the original recording.
- **Editor AI**: explicitly transcribe up to five minutes of original audio, then locate, audition, hide or keep a segment or available aligned unit with boundary padding and undo. Copy the complete or selected transcript, or export TXT, SRT, or WebVTT; subtitles use original-recording times, and untimed results remain available as text. Independent mode keeps evidence in the current project. Material search shows literal matches first, followed by semantic candidates from existing indexes.
- **Sound Assembly**: create a sequence or layered assembly from a Library selection, then move, re-track, split, trim, duplicate, fade, overlap, and mix up to 256 clips across 8 tracks and 4 hours. Track gain, pan, mute/solo, master gain and limiting, undo/redo, immutable version saves, preview, and multiformat mixdown are included. Once sources are prepared, preview mixes into a short streaming buffer without rendering a complete mix file; gain, pan, and mute/solo controls can update during audition; audio topology or effect-chain changes require preparation again.
- **Recipes and delivery**: save and apply processing recipes, then export one recording or a bounded batch as WAV, FLAC, MP3, or M4A. Exports and rendered spectral-repair working copies retain their source revision and processing record.
- **Exact navigation**: enter seconds or a timecode for a millisecond source selection, or jump to a project time in the arrangement. Selecting a range changes the view without editing the audio.
- **Batch arrangement**: Cmd/Ctrl/Shift-click clips to move, re-track, duplicate, split, or delete them together. Ripple deletion affects selected tracks or all tracks while preserving the tails of crossing clips. Alt-drag slips source audio inside fixed clip boundaries; each batch edit takes one undo step.

### Current boundaries

- Echo is currently a development preview for macOS on Apple Silicon. Catalog, adjustment-graph, and interaction formats may continue to change.
- Local intelligent features require a configured Infer Runtime with the relevant capabilities. Import, browsing, waveforms, playback, and existing DSP adjustments remain usable when the Runtime is unavailable.
- People, place, emotion, and sound-type projections are currently traceable model evidence or hints, not a complete identity, relationship, or fact-confirmation system.
- Raw-audio CLAP retrieval currently covers a bounded short-recording slice. Complete segment-level semantic retrieval for long recordings is still under development.
- Neural denoise, source separation, ambience generation, and audio extension remain unavailable; short narration generation has an explicit candidate workflow. The public repository does not distribute model files. Source labels come from user declarations or imported file declarations and propagate conservatively for whole referenced sources. Exports embed a compact source-kind declaration in WAV/FLAC metadata, and Echo restores it on import. These are unauthenticated declarations; external conversion may strip them. Private notes and paths remain in Echo.
- Saved mixes reopen their source project; nesting a mix project inside another project is not supported. Project clip editing supports original spectral repair; post-render working copies remain in the single-sound workspace.
- Sound Assembly is not a general-purpose DAW. Recording, input monitoring, MIDI, tempo grids, time stretching, plug-in hosting, sends, arbitrary routing, track-parameter automation, and video sync are outside the current module.

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
