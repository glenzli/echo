# Echo ROADMAP

本文档是 Echo 的产品设计与工程决策记录。改动架构、数据模型或里程碑边界时，先更新这里。

## 1. 产品定义

> Echo 是一个 local-first、非破坏性的声音记忆与编辑系统。记忆库保存用户希望留下的原录音、调整版本与多轨作品；编辑器可以引用这些记忆，并从全局或项目素材中加入配乐、环境声和音效。每个结果保留清楚、版本固定的来源关系。

记忆库是日常收藏、搜索和重温的入口；完整多轨编辑与精细声音处理服务于这些内容。素材是否进入记忆库由用户决定，外部声音不再被强制当作值得长期重温的记忆。当前仍不新增录音、MIDI、插件宿主或生成式声音执行能力。

能力划分为六个域：

```text
Preserve   原始记录、provenance、非破坏性版本
Understand ASR、人物、情绪、声音事件、语义索引
Restore    降噪、响度、EQ、去混响、修复录制缺陷
Revisit    记忆库 / 声音相册 / 记忆、素材与原始磁带
Creative   对所选真实录音做显式、可旁路的确定性场景与角色化处理
Assemble   引用记忆和素材，形成非破坏性、可追溯的声音编排与交付
```

关键定位：**记忆表达保留意图，素材表达制作用途；同一来源可以同时具有两种身份。**

系列定位：Shadow = Photo Library + RAW Developer + AI Understanding；Echo = Audio Library + Audio Restoration + AI Understanding。

## 2. 核心原则（不可动摇）

1. **Original immutable**：原始文件永不修改，永远保留 original_ref。
2. **Analysis 不是事实**：所有 AI 结果必须存 `value + model + model_version + confidence + timestamp`，模型升级后可重新分析。
3. **Cache 可全部删除**：waveform、embedding、transcript cache、render proxy 全部可重建。
4. **实时音频路径保持极度简单**：audio callback 里绝不出现 Rust→AI→Python→allocator→async 链。AI 只在后台工作，绝不侵入播放链路。
5. **记忆库是首屏**：Listen first, Edit second。第一屏不是 Timeline Editor。
6. **调整克制**：恢复性调整（Loudness/EQ/降噪/去混响/Dynamics/Trim/Fade/Channel）与
   Creative VFX 必须在产品语言、持久化身份和执行 owner 上分域。Creative 只处理用户显式选择的
   既有录音，默认关闭、可旁路、静音不生声、不改词、不克隆具体人物、不覆盖 Original，也不冒充
   Restoration。语音生成、目标人物换声、改词、音乐生成、提示词 SFX 与其他生成式能力仍属于未来
   Audio Studio；AI 降噪、源分离和神经修复继续等待独立 InferenceBackend 里程碑。
7. **编排独立**：单资产 `AdjustmentGraph` 始终只拥有 Original／源时间调整；跨资产轨道、
   Clip 放置、混合时间、编排版本和 Mixdown provenance 由独立 `SoundAssembly` owner 负责。
   Clip 引用已登记声音来源及一个明确调整 revision；播放与导出先把该 revision 确定性准备为
   线性声音来源，再在编排时间混合。外部文件可以作为素材登记；编排结果可以作为记忆保存，
   但始终保留 assembly revision 与各个原始来源，不伪装成现场 Original。

### 2026-09-20：记忆、素材与现有编辑工作区

- `sound_items` 负责记忆／全局素材的用户收集身份，引用真实 `AudioAsset` 或独立 `SoundAssembly`。
  原资产 ID 保持不变；已保存编排使用其自己的稳定身份，保存聆听版本不制造假的 Original。
  收藏、评分、相册和聆听状态使用统一声音条目身份；旧原录音、调整与 Analysis 字节不改写。
- 素材可以在全局复用，也可以只归属于项目。文件导入后受管保存，已经被项目引用的素材不会被
  当作可删除的渲染缓存。取消收集只改变浏览身份，不删除素材字节或破坏已有项目引用。
- 编排记忆的每个聆听版本固定一个已完成的混音及其精确 provenance；混音存放在 Catalog
  旁的持久媒体目录，和可重建 cache 分开。编辑草稿／保存工程不会悄悄替换记忆库聆听版本。
- 沿用现有声音空间、单音处理和编排工作区。声音空间承接记忆库；编辑器内提供本项目、记忆库、
  素材浏览与独立试听；全局素材页复用同一来源与分类数据。已有 AI 事件、关键词和用户校准作为
  可解释查找依据，分类不是现场事实，也不引入第二套推理调度器。
- 编排片段检查器展示来源名称、身份、区间与固定版本。精细处理必须明确是修改来源记忆还是
  当前项目的处理版本，返回编排保留当前文档、选中片段和时间位置。

## 3. 技术架构

### 多轨编辑交互的细化

- 在现有 `SoundAssembly` 模型内补全直接操作：片段移动、双边裁切、淡入淡出手柄、
  片段端点／播放头／时间网格吸附、分割、相邻复制、同轨闭合空隙与显式交叉淡化。
  一次手势只产生一次撤销记录；来源区间和四小时编排上限始终有效。
- 轨道控制区固定，标尺、网格、播放头与横向滚动共享坐标；素材栏、检查器按需展开。
  异步原始波形按固定来源版本的编辑段映射到片段，包括裁切、隐藏、静音和间隔。
  该波形是来源概览，不声称是实时效果输出；混音仍使用已固定版本的原生离线渲染。
- 编辑器保持用户命名、内嵌标题或文件名；后台分析不能将片段改名。模型事件分类明确标作
  AI 识别。真实 MP3 验证同时覆盖流起始时间戳归零和不足一毫秒的已知时长取整尾差；
  更大的解码缺帧仍由严格的导出帧数检查拒绝。
- 预览绑定作者文档内容；修改、切换项目或退出工作区时终止旧试听。保存工程不能使旧预览
  自动变成当前版本。来源波形队列只有一个活动读取，结果只投影到当前项目。
- 开发调试可以获取有来源和许可的音频，并用 Infer Runtime 独立的本地操作身份生成明确标记
  的合成素材，记录请求、模型／作业身份与文件哈希。这不扩展 Echo 的生成权限或引入产品生成入口。

### 2026-09-20：频谱精修与传统编辑能力

产品呈现保持记忆导向，编辑能力以传统音频编辑器的可操作性、试听一致性和交付可靠性为目标。
本次将原有固定衰减框升级为精确频谱修复：独立的时频选区、逐区选择／移动／删除、时间与 Hz
输入、可调衰减和双轴羽化、显式谐波扩展、原始频段监听和修复区间试听，均复用现有非破坏性
区域合同与草稿历史，不新增像素坐标或不可重建的缓存事实。

频谱细节从当前源时间窗口重新解码，按实际 FFT bin 映射为线性或对数频率轴；支持频率聚焦、
瞬态／音调分辨率与显示动态范围。屏幕图像只描述来源，选择与音频处理始终使用源时间和 Hz。
后台任务有界、可取消，过期结果不能投影到新来源或新视窗。原始频段监听是暂态诊断，不写入版本。
草稿频谱参数必须同时进入编辑试听、响度测量和已保存版本的离线交付，任何参数变化使旧结果失效。
修复资料库 SQL 投影遗漏频谱参数的问题，保存重开必须恢复完整调整图；后台元数据更新不得重置同一
来源版本的未保存草稿。

后续独立里程碑仍包括噪声样本学习／频谱降噪、上下文修补与克隆、音符级音高校准与保调时间伸缩、
轨道效果链和自动化。现有移调、底噪扩展器及矩形衰减不能冒充这些能力；每项以实际编辑、试听、
保存重开和交付验证收口。

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

仓库内曾用于概念验证的 MLX/Ollama/脚本执行已退出生产链路；Echo 不再扩充模型选择 UI、
物理模型路由、下载器或驻留进程管理。正式接入采用 Infer Build 已有的
`audio.transcribe` / `audio.align` 任务接口，不预建没有消费方的第二套调度机制。

正式 consumer 固定使用官方 `infer-runtime-client`，Git revision 为
`5b2895f9f2ad4ae10ce6eaae8eacec77077d23a6`；Core 身份固定为
`infer-runtime.consumer-core@20260813.1`，能力目录固定为
`infer-runtime.capability-catalog@20260813.1`。Echo 仍以独立、非资源管理员的 `app_id=echo`
调用，既有 managed credential 路径与 ACL 不变。endpoint 只允许显式
`ECHO_INFER_ENDPOINT`／产品设置 development override，或由 SDK 从
`infra.discovery.registration@20260812.1` 发现；没有合法 Discovery 时失败关闭，不猜测固定端口。

SDK 是 Discovery、generation、canonical numeric loopback 校验、proxy／redirect 禁止、Core／
Capability header、credential file loading、HTTP／`error.code` 分类和 Job snapshot 解码的唯一 owner。
Echo 不再持有这些协议基础实现，也不把 bearer token 物化到 Rust/C++/QML 产品边界、设置、日志或
命令行参数。SDK 发现的 manifest 只是连接候选，连接才建立存活事实；Echo 不修改或删除 Provider
manifest。Echo 的后台 worker 只提交稳定 Intent、核验产品约束、记录本地任务状态，并把已净化的
Runtime Job／Attempt／模型构建证据写入 Catalog。四条已迁移路径分别要求精确 capability：
`audio.transcribe` → `infer.audio.transcription@20260814.1`、`audio.align` →
`infer.audio.alignment@20260811.1`、`text.summarize` → `infer.responses@20260812.1`、
`semantic.embed_text` → `infer.vision.text-embedding@20260811.1`。具名 Deployment／Model Profile
路由不自动启用，协议升级也不扩大 App ACL、fallback 或 Provider 可见性。Runtime 不可用不得
阻断扫描、波形、播放或 Library 浏览。

`infer.audio.transcription@20260814.1` 把 `language` 固定为仅在无歧义时存在的 document-level
标量；混合语言的 provider 证据由 SDK 类型化为无区间的 input set，或仅在 provider 提供时使用的
有区间 segments。Echo 不把空标量误判为无语音、不私有解析数组，也不虚构语言时间范围。

2026-08-13 hard migration 的合同测试只使用官方 SDK fixture／fake transport，不把仍运行旧预发布
草案的 daemon 当作新合同成败依据；真实 E2E 必须等 Runtime 切换到上述 Core／Catalog 后执行。
声音事件现由官方 SDK revision `5b2895f9f2ad4ae10ce6eaae8eacec77077d23a6` 的
`Client::detect_audio_events_file` 消费，精确能力为
`infer.audio.event-detection@20260813.2`。Echo 只映射经 SDK 严格验证的 AudioSet
事件、coverage、speech presence、ontology／policy 与 provenance，再核验 Echo App-scoped
Job 和本地约束；不得复制 HTTP／YAMNet 解析或回退到私有协议。

文本理解沿用同一个受管 Consumer 身份，但由独立的 Responses 协议 owner 负责。首个产品动作
固定映射到 `text.summarize`：`audio.align` 成功后，Echo 的后台队列提交有界文字与产品指令，
要求模型返回 contextual JSON，再由 Echo 做结构、数量和长度校验。Runtime 当前未冻结 JSON
Schema structured-output 合同，因此格式不合格必须作为稳定的派生元数据失败记录，不得猜测、
修补正文或回退为裸 Ollama／MLX 调用。成功结果与音频结果一样，必须先核验 App-scoped Job、
local-first／local_only／background／no fallback 约束，再写入分析证据。

自然语言检索继续沿用同一 Echo Consumer 身份，但只增加 `semantic.embed_text` 最小 Intent 权限，
不获得图片、人脸或资源管理能力。Echo 通过 typed
`POST /infer/v1/vision/text-embeddings` 获取 768 维、L2 normalized 的文本向量，严格核验
精确 Core／Capability 身份、App-scoped Job、local-first／local_only／background／offline／
no fallback／零成本约束，以及 provider/deployment/model build 和精确 embedding-space 身份。
当前能力来自 SigLIP2 的文本塔：它适合在同一跨模态空间内提供 provisional 的文本证据近邻，
但不是专为通用 text-text 检索优化的 encoder，也不能冒充尚未接入的 CLAP 音频向量。
用户查询只在内存中短暂存在，不进入 Catalog、普通 metadata 或日志；Runtime 不可用时，已有
literal FTS、Library 浏览、扫描、波形与播放保持可用。

Contextual 产品合同从 2026-08-10 起显式区分“声音速写”和“理解摘要”：声音速写是供声音墙、
胶片带与检查器标题消费的一行短标题，必须沿用文字的主要语言、描述可听见的场景或事件，且不得
复制正文或使用“这段录音／某人描述”等模型元话语。声音速写最多 14 个中日韩字符或 7 个词，
Consumer 对其他方面合格的模型速写执行最终硬压缩；理解摘要是可选的压缩层，不是换写正文：
短文字直接留空，较长文字的摘要也必须不超过 40 个中日韩字符或 16 个词，且不超过来源文字
约三分之一，才用于详情与搜索。没有压缩价值的可选摘要降为空，不连带丢弃其他上下文证据。
两者都属于可重建模型证据，不替代完整文字。合同以内容 schema version 和后台 job revision
共同版本化；旧版本或缺少合格声音速写的 contextual 证据在启动时幂等回填，以新的 append-only
Analysis 覆盖读取投影，不原地篡改历史证据。长录音仍必须先经过后续独立的代理／分段与聚合切片，
当前有界输入不得静默截断，也不提前把尚无消费方的章节系统塞进 contextual owner。

用户校准与 Analysis 必须是两条独立事实链：Catalog 以 append-only revision 保存用户对声音标题、
摘要、事件、情绪、关键词、文字和语言的稀疏修订，模型原始输出及其 provenance 永不回写。展示、
文字检索、语义文档与动态筛选读取“用户修订优先、未修订字段回落到最新模型证据”的有效投影；
再次分析只能更新没有被用户校准的字段。清空字段也是显式修订，恢复模型值则追加一个新的空校准
revision。时间戳片段仍属于模型对齐证据；用户改过完整文字后，UI 不得把旧片段时间假装成精确
对应关系。Catalog `20260813.3` 是这一校准链与详情检查器原位编辑的首个有界切片。

## 4. 音频底层

- 不自己重写 codec，直接用 FFmpeg / libavfilter。
- 内部统一 canonical PCM：**float32、48 kHz、channel-preserving**。真正输出时再编码。原始文件永远不动。

## 5. 数据模型：AudioAsset

```text
AudioAsset
├── Original        content hash / path / codec / timestamp / metadata（immutable）
├── Analysis        transcript / speakers / emotions / audio events / embeddings / segments
├── UserState       liked / rating / album membership / metadata calibration（用户事实）
├── AdjustmentGraph gain / eq / denoise / normalize / trim（非破坏性）
├── DerivedRenders  可重建
├── Publications    用户导出的文件与来源版本证据（不由缓存维护删除）
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
播放。首个 Level 5 contextual 切片只在非空文字完成对齐后持久入队，作为摘要、关键词和
情境提示的可重建派生元数据；历史录音在启动时做幂等回填。其余理解层仍由能力、资源和
用户需要渐进触发；正式接入后由 Infer Build 负责准入、空闲调度和资源仲裁，而不是让 Echo
在导入事务里同步运行模型。

后台分析的产品状态以 Catalog 中的 durable Job、Inference Run 和已接受证据共同投影，不能
用“卡片上有没有文字”猜测。声音墙与检查器必须区分排队、运行、已完成、等待 Runtime、需要
人工处理和 Original 缺失；已完成的阶段与长录音分段结果始终保留，续跑只从第一个缺失阶段
继续。Runtime Discovery／连接、Provider 容量、队列满和 deadline 属于自动恢复错误：Echo 在
Runtime 合同探测重新健康后以有界退避重新入队，不要求用户重导入，也不阻塞扫描、波形和播放。
后台 Worker 的领取与终态转换以进程内递增 revision 通知桌面刷新权威 Catalog 投影；桌面只轮询
这个常量开销的 revision，不周期扫描整份声音库，也不得让已失败任务停留在“正在分析”的旧快照。
认证、权限、协议／合同不一致、模型输出结构校验和源文件错误不得后台无限循环；它们保留稳定
错误码并提供单声音与批量人工重试，其中 Original 缺失必须先完成重连。取消也只允许用户显式
恢复。所有恢复都复用同一个本地 Job 身份与 App-scoped provenance，不复制分析证据，也不把
正文、token 或 Provider 诊断写入 UI 日志。

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
  中间以网格并行浏览，右侧是所选声音的详情检查器；“声音带”将当前筛选结果虚拟首尾相接，
  用统一带头连续试听并跨声音定位，但不生成拼接文件、不保存顺序，也不承担编排编辑。双击声音
  仍在中间区域原位进入单声音聆听视图，左侧资料库与右侧详情不退出。
- 声景卡片必须来自真实证据：缓存 waveform、文件元数据和可追溯 AI 分析。AI 可生成可修改的
  标题、文字预览和事件标签，但不得用虚构图片冒充录音内容。
- 声音墙中的波形只承担声音密度、时长与播放位置提示，不承担识别语义：默认卡片约三成半高度
  给波形、约六成半给声音速写、情绪、关键词、文字片段、事件、时间地点与来源信息。文字片段
  与声音速写保持明确的标题／正文层级，不得重新充当标题。卡片放大时新增空间优先披露更多信息；
  只有单声音聆听视图才把波形提升为定位、分段与拖动的主要交互面。
- 声景卡片标题固定使用合格的“声音速写”，其次才是文件内嵌标题与文件名；完整文字永不作为标题
  兜底。声音速写在模型合同内保持一行和短长度，卡片尺寸只决定视觉截断，不把长正文压成伪标题。
  详情中的“理解”只在摘要具有真实压缩价值时展示，短文字允许没有摘要；文字区继续展示逐字内容，
  三种信息不得混用，也不得用“未分析”占位替代一个经过分析但有意留空的摘要。
- 用户层产品语言称 ASR 结果为“文字”；`transcript` 只保留在模型能力、数据类型、协议和证据
  来源等技术语境中。点击文字仍可定位声音。
- Like、评分与相册归用户事实，不是 AI Analysis；文件内嵌时间、地点和标签归 Original 的来源
  元数据，必须标注来源，和模型推断严格区分。
- 单击 asset：右侧快速检查 Waveform / 文字 / Events / People / Metadata；双击 asset：在同一
  Audio Space 中进入 Waveform / 播放 / 文字的单声音视图。Audio Space 的网格与单声音视图都只
  负责浏览、证据检查和试听；恢复性 Adjustment 位于标题栏并列的独立“声音调整”工作区，继承
  当前选中声音，并以显式、可收敛的版本面板区分草稿预览与版本保存，任何浏览行为都不隐式改参。
- “声音编排”与 Audio Space、“声音调整”并列为第三个顶层工作区。它从 Library 选择创建或打开
  一个持久编排文档；左侧管理编排与轨道，中间时间轴直接操作 Clip，右侧检查选中轨道或 Clip，
  顶部继续复用统一的 Undo／Redo／保存版本语法。编排工作区不得接管单资产恢复参数，也不得让
  临时播放准备或导出任务成为 QML 隐藏生命周期。
- 桌面窗口只保留一条与 Shadow 同构的融合标题工具栏：品牌、工作区导航、当前工作区工具、
  设置与原生窗口拖动共享一个 chrome owner；内容区域不得重复绘制第二条伪标题栏。
- 当前集合、结果计数、内容搜索、网格／声音带与卡片密度属于内容呈现状态，沿用 Shadow 的
  系列结构放在中间内容区顶部工具栏，不占用融合标题栏；卡片尺寸滑块以接近目标宽度的列数排布，
  默认应形成紧凑并行浏览，而不是宽卡片。排序与筛选留在跨工作区底部工具栏；Like、评分属于所选
  声音的用户操作，放在画布右下浮动工具栏。浮动工具栏不重复提供“展开”；双击声音进入单声音
  聆听视图，顶部声音带按钮只切换集合级连续浏览，两者不再共用一个胶片图标或交互身份。
- 复合筛选只展示当前资料库真实存在的关键词、情绪、录制年份、地点和事件值；同一类别内任一值
  可匹配，不同类别以及 Like／评分／人声条件必须同时满足。底部筛选以图标为主，避免用派生结果
  名称挤占持续浏览空间。当前“人声”集合与筛选只接纳已有非空 ASR 文字证据的声音，是保守的
  已识别人声集合；尚未完成分析或没有文字证据时，不反向断言录音中不存在人声。
- 左侧栏保持为稳定的资料库、系统集合与相册导航，不持续展开高基数关键词。AI 关键词是所选声音的
  模型证据：完整列表进入右侧详情，卡片只展示少量代表词；模型尚未提供显著性时不得把顺序前几项
  描述成“核心关键词”。情绪同样是声音属性，在卡片文字区以彩色标签呈现，并进入详情，但不作为
  声音相册的成员规则。关键词、情绪、地点、事件和人物提示按各自维度使用最新非空模型证据；一次
  重分析的空值表示没有新证据，不得静默抹除旧的正向证据，真正撤销需后续显式 tombstone 合同。
  卡片不重复标注“AI 文字”或用装饰星号声明产品的基础能力；派生内容为空时保持安静，不用格式、
  模型状态等占位文字填充。
- 卡片缩放表达信息密度而非百分比：整段声音始终完整映射到波形；卡片变宽时使用更细的
  waveform pyramid 展示分辨率，并按“概览／浏览／丰富”渐进呈现文字、时间与来源证据。
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
  - 已完成（正式 Runtime 切片，2026-08-09；Discovery 与声音事件迁移 2026-08-11；官方 SDK
    hard migration 2026-08-13）：Echo 以独立非管理员 App 身份消费
    `infer-runtime.consumer-core@20260813.1`／`infer-runtime.capability-catalog@20260813.1`，并由
    revision `5b2895f9f2ad4ae10ce6eaae8eacec77077d23a6` 的官方 SDK 通过 owner-only 稳定
    registration 发现本机 Consumer endpoint；后台队列提交 `audio.transcribe`，非空文字继续
    `audio.align`。四条受支持路径都读取 App-scoped Job/Attempt，并把 Core／Capability 身份、
    provider/deployment、physical model/build 与稳定错误码写入 Catalog。桌面读取层以最新对齐
    证据细化段落时间，无法可靠匹配时保留原转写时间。固定 `8787` fallback、candidate 合同分支和
    Echo 私有 Discovery／wire／header／error 实现已删除；当前旧 daemon 不参与新合同验收。
  - 默认分析策略（2026-08-09 校准）：新录音完成注册后持久入队 waveform 与
    `audio.transcribe`；应用启动时为已有但缺少 transcript 证据的在线录音做幂等回填。
    结构性扫描、导入和 waveform 优先于 ASR；ASR 失败不得影响 Original、播放或浏览。
    手动分析只作为失败重试/调试入口，不是正常产品路径。非空文字完成 `audio.align` 后，
    继续以最低队列优先级提交 `text.summarize` contextual 元数据；有效空文字会以本地-only
    `audio.detect_events` 取得事件证据。ASR 失败、未分析或空文字本身都不得反向断言“无人声”。Runtime 的
    `speech_presence` 只有完整覆盖且低于版本化阈值时才可为 `absent`。该默认仍不扩张到
    SenseVoice、diarization 或 TTS。M2 已在合格 contextual 证据之后追加独立、可重建的
    text-evidence embedding，不改变 M1 的音频分析 admission。
  - 声音事件合同（2026-08-14；官方 Catalog／SDK 已发布）：`audio.detect_events` 以
    YAMNet／AudioSet 形成有界、多标签、
    带时间区间的 `AudioEvents` 证据。Echo 以 AudioSet MID `class_id` 作为稳定身份，完整保留
    coverage、ontology、阈值／平滑 policy、模型与解码 provenance；英文 label 仅为展示文字。
    Echo 从每个稳定 class id 的最高分观察确定性派生短声音速写、关键词和事件 facet，复用现有
    声音墙、复合筛选与 text-evidence 搜索；派生展示不替代原始事件证据，也不冒充 CLAP 音频向量。
  - Contextual 展示合同（2026-08-10）：`text.summarize` 输出升级为带版本的严格 JSON，新增
    独立声音速写并保留理解摘要、关键词、情绪、地点、事件和人物提示。Echo 校验字段全集、版本、
    长度、主要书写系统和元话语；卡片不再使用正文兜底。旧 contextual job 与证据通过版本化身份
    自动进入 append-only 回填，失败只影响派生展示，不影响扫描、波形、播放和已有文字。
    v3 进一步将声音速写硬压缩到 14 个中日韩字符／7 个词，并把理解摘要改为可选压缩证据：
    短文字留空，长文字摘要必须同时满足绝对长度和相对压缩率，避免摘要与正文等长；摘要不合格
    时只降为空，不连带丢弃关键词、情绪等其他有效分析。
  - 冻结项：不再增强 Echo 内的物理模型注册、Ollama worker 或裸 Python 路由。
  - 正式 Runtime 接入边界（2026-08-09）：`audio.transcribe` 成功后读取 App-scoped
    Job snapshot，再以 `audio.align` 细化文字时间；Echo 持久化 Runtime job id、合同版本、
    provider/deployment、physical model/build、Attempt 和稳定错误码。25 MiB 上传上限在
    consumer admission 明确失败；长录音代理／切片属于后续独立 payload 切片，不允许静默
    截断原始声音。Echo 不消费 speech/TTS/voice-clone Intent。
  - 长录音切片合同（2026-08-11）：当原始文件超过 Runtime 25 MiB 上限，或声音时长
    超过 10 分钟时，Echo 以原始内容身份和稳定规划版本生成 8 分钟叶子切片；该时长边界同时
    满足 `audio.detect_events` 的 600 秒直接输入上限。每次只流式
    解码一个时间窗，生成 16 kHz、单声道、16-bit PCM WAV 分析代理，按 BLAKE3 内容地址
    原子发布到可删除 cache；Original 不改写，后台不读入整个源文件或代理。切片计划、
    代理引用、每段 transcribe/align/contextual 阶段和 Runtime provenance 逐段持久化，父任务
    在任一阶段成功后都可断点续跑；整段有效空文字时，声音事件复用同一组有界代理并把每段 Runtime
    结果连同 Original 时间轴偏移聚合为一个 append-only `AudioEvents` 证据；不为同一声音并发
    占用多个本地模型候选。
  - 分层文字合同（2026-08-11）：叶子文字和对齐时间统一投影回 Original 时间轴，
    并索引为资产级文字证据；叶子声音速写再以有界子节点递归压缩，直到得到一个根速写。
    每个节点保留子区间、内容 schema 与 Runtime provenance，没有靠静默截断制造“整段摘要”。
    声音墙与轻量详情不再急切加载整份 transcript JSON，只返回有界预览和叶子计数；
    完整文字、章节与点击定位只由显式打开的单声音工作区及专用读取 owner 按需提供；
    完整文字实体的分页／虚拟化是后续大实例压测的性能门槛，不由声音墙偷偷代读。
  - 待办：SenseVoice 的窄域 emotion 能力 Intent、speaker 证据；音频合同能在
    执行中暴露 Job id 后，再补真正可中断的 Runtime 取消（当前同步 endpoint 仅在终态返回 id）。
- **M2 Library**：声音墙、声音相册、Like/评分、来源元数据筛选、自然语言搜索、人物/声音、
  时间、audio event、CLAP semantic search。
  - 首个浏览切片（2026-08-09）：声音墙成为默认首屏；资料库、横向声景卡片和详情检查器拆分
    为独立 UI owner；现有大波形页面降为展开层。用户 Like/评分持久化，卡片使用真实 waveform、
    文件时间/格式和已有 AI 文字证据；FFmpeg probe 开始保存容器、采样率、声道和嵌入标签；
    窗口 chrome 收敛为与 Shadow 同构、可拖动的单一融合标题工具栏；视图与卡片尺寸进入内容区
    顶部呈现工具栏，排序与筛选留在底栏；所选声音的 Like／评分进入右下浮动工具栏，卡片密度开始
    驱动波形分辨率与渐进信息呈现。
    网格与带胶片带的单声音视图共享筛选结果和选择身份；关键词、情绪、年份、地点与事件形成
    existing-value 复合筛选，网格末尾保留浮动工具栏安全区。
  - 首个 AI 聚合切片（2026-08-09）：contextual 证据中的关键词进入可重建的 Catalog Facet
    索引；原始模型输出仍保留在 append-only Analysis 中，Facet 只保存规范化检索键与展示值。
    关键词在卡片上提供少量代表词，并在所选声音的右侧详情中完整呈现；可重建 Facet 保留为后续
    有边界的搜索／筛选能力，不在左侧资料库中展开。不做未经证据支持的同义词或重要性推断，也不把
    关键词提升为用户确认事实。浏览投影按关键词、情绪、地点、事件和人物各维度选择最新非空证据；
    新的非空集合替换同维度旧集合，空集合不再把仍可追溯的旧正向证据从卡片和筛选中抹掉。
  - 智能相册候选切片（2026-08-09）：Catalog 将 contextual 浏览索引扩展到情绪、地点、事件和
    人物提示；相册投影只使用地点、事件、人物提示，以及 Original 的录制日期和嵌入地点，情绪明确
    保留为单段声音属性，不生成相册。候选至少需要
    两个精确成员，携带成员身份、证据维度与来源，重分析后旧成员关系自动退出；桌面只按 Catalog
    返回的成员集合浏览，不在 QML 中重新解释模型字符串。同维度新的非空证据会令旧成员关系退出；
    空值不充当否定证据。候选不是用户相册事实，不做同义词、人物
    身份或相近地点合并，用户确认／保存相册留给后续独立切片。
  - 自然语言检索首个切片（2026-08-11）：Catalog `20260811.6` 为每段声音构建有界、可重建的
    semantic document，来源由声音速写、压缩摘要、关键词、情绪、地点、事件、人物提示和有限
    文字预览组成；当前 contextual／transcript Analysis id 共同形成 source revision，任何证据
    更新都会令旧向量失效并持久入队回填。向量按精确 space 保存为 8-bit 量化投影，Runtime
    provenance 与文档一起保存；不同 space 永不混排。literal 层同时为整份有界证据建立 FTS，
    避免只有逐字文字能被精确找到。
    桌面搜索先同步返回文件名、卡片证据与 FTS 结果，320 ms 停顿后才在独立线程提交一次短暂
    query embedding；旧 generation 返回会被丢弃，Qt 主线程不等待 Runtime。混合结果把直接
    包含、FTS 与 semantic 近邻按来源排序；semantic 只披露前 40 候选中相对靠前的四分之一，
    最少 3、最多 12 条，分数只作相对排序而不显示为“置信度”。Echo CLI 提供只输出资产身份和
    相似度的诊断入口。当前只完成“AI／文字证据 ↔ 自然语言”的 provisional 检索；不声称已经
    能在没有任何文字证据时从原始波形找到火车、雨声等声音，CLAP audio↔text 双索引仍是后续
    独立能力与质量门槛。
- **M3 Restore**：非破坏性 effect graph、EQ、loudness、DeepFilterNet、A/B Original。
  - 首个基础处理切片（2026-08-10）：Catalog 以 append-only revision 保存明确单位的
    `AdjustmentGraph`，当前只包含 Trim、Fade in/out 与输出 Gain；重复保存同一图保持幂等，
    Original 与缓存身份均不改变。独立“声音调整”工作区提供草稿、预览、重置和保存版本，编辑
    波形明确显示有效范围；Audio Space 的单声音视图只播放当前已保存版本，不再承载编辑控件。
    C++ audio engine 在打开播放源时把调整图预编译为帧范围与包络，在解码生产线程应用，
    Qt 实时回调继续只读取已经处理好的 SPSC buffer。当前不含导出、EQ、响度分析或降噪，
    这些必须沿同一版本图与可重建 render 边界逐项进入，不能回写源文件。
  - 首个轨道编辑交互（2026-08-10）：编辑工作区将长声音投影为可缩放、可横向滚动的时间轴，
    提供动态时间标尺、全局／选区适配、播放跟随和底部导航。裁剪边界、线性淡入淡出端点与整段
    Clip Gain dB 线都在波形轨道上直接拖动，手势期间只更新短生命周期草稿，试听时才重新准备
    播放图；支持在同一裁剪选区内切换 Adjusted／Original 做 A/B 试听。参数栏只承担精确读数、
    撤销当前草稿、清空调整与显式保存版本。此切片不伪装已实现任意包络关键帧、曲线类型、EQ、
    响度、Dynamics、降噪或导出；这些仍需先扩展 AdjustmentGraph 与执行器合同。
  - 编辑器第二个基础切片（2026-08-10）：`AdjustmentGraph` 与 Catalog `20260810.2` 将淡入、
    淡出分别扩展为 Linear、Smooth、Equal Power 三种稳定曲线；旧版本记录迁移后明确保留为
    Linear。C++ 播放生产线程按曲线计算真实振幅，时间轴绘制与试听执行共享同一枚举合同，不再把
    曲线当作纯 UI 外观。编辑工作区增加短生命周期时间选区、选区循环、按选区裁切，以及按完整
    手势合并的 Undo／Redo；`SoundAdjustmentDraft` 独立承担校验、历史和显式发布，时间轴与右侧
    精调检查器只负责直接操作和呈现。此切片仍不包含关键帧包络、响度测量、EQ、Dynamics、降噪
    或导出；这些应各自以真实执行器和可验证用户结果进入后续版本图，而不是先放置无效控件。
  - 编辑器第三个基础切片（2026-08-10）：`AdjustmentGraph` 与 Catalog `20260810.3` 增加可旁路的
    20–240 Hz Low Cut 参数；二阶 Butterworth 高通在 C++ 解码生产线程按声道维护状态并作用于
    真实试听，Seek 会重置滤波器状态，Qt 实时回调仍只消费已处理 PCM。编辑器底部精调台提供开关、
    频率滑块、完整 Undo／Redo、显式保存版本与 Original A/B；Audio Space 播放当前已保存参数。
    此切片不把 Low Cut 扩张为尚未实现的参数均衡器，也不包含响度、Dynamics、降噪或导出。
  - 首个高级恢复切片（2026-08-10）：`AdjustmentGraph` 与 Catalog `20260810.4` 增加固定频段的
    三段均衡增益（120 Hz 低频搁架、1 kHz 中频峰值、8 kHz 高频搁架，均为 ±12 dB）；Catalog
    继续 append-only 保存 authored centibel intent，滤波系数与逐声道状态只由 C++ 播放计划准备。
    三段 Biquad 在解码生产线程执行，Seek 会重置状态，实时回调仍只读取已处理 PCM。调整工作台
    将窄纵向基础面板与独立三段均衡面板并排，恢复正常可读字号并降低默认波形高度，为后续真实的
    响度、Dynamics 与降噪 panel owner 留出空间；未实现能力不显示空控件。
  - 三段均衡试听连续性修正（2026-08-10）：播放中的 EQ 手势不再销毁并重建解码会话；QML 以
    16 ms 合并窗口发布最新目标，C++ 播放会话通过原子参数邮箱交给解码生产线程，并在两个完整
    Biquad 状态组之间做 30 ms 线性交叉过渡。预处理 ring 收敛为约 85 ms，保证回调仍只读取已
    处理 PCM，同时避免两秒旧声音掩盖交互结果。设备输出前增加不属于 authored graph 的安全
    保护 owner：短攻击／释放包络配合可微软上限，替代 `[-1, 1]` 硬截幅；Seek 同时重置滤波和
    保护状态。此修正不改变 Catalog schema，也不把安全保护冒充为可编辑 Dynamics／Limiter。
  - 首个 Dynamics 切片（2026-08-10）：`AdjustmentGraph` 与 Catalog `20260810.5` 增加可旁路的
    立体声联动软拐点压缩器，稳定保存 Threshold、Ratio、Attack、Release 与 Makeup authored
    intent。C++ 独立 `DynamicsProcessor` 在三段均衡和 Clip Gain 之后、淡入淡出包络与设备安全
    保护之前执行；跨解码块保留检测包络，Seek 重置状态，实时参数变更只更新最新目标而不重启
    播放。调整工作台新增紧凑 Dynamics 面板与真实传递曲线，所有控制进入同一草稿历史、Original
    A/B、显式保存和 Audio Space 已保存版本播放合同。此切片是峰值压缩，不宣称已经具备 LUFS
    响度测量、自动增益、Limiter authored effect、多段压缩或降噪。
  - 播放响度反馈切片（2026-08-10）：独立 `LoudnessMeter` 在设备安全保护之后读取实际 prepared
    PCM，以 BS.1770 K-weighting 和精确 400 ms 滚动窗口计算 Momentary LUFS，同时发布带衰减保持的
    Sample Peak 与压缩器实时 Gain Reduction。测量在解码生产线程执行，Qt 回调仍只复制 ring 中
    PCM；播放会话经原子快照把三个只读指标交给桌面控制器，Dynamics 面板以紧凑电平条和数值反馈
    当前听到的处理结果。Seek 会重置测量窗口。此切片不改变 `AdjustmentGraph` 或 Catalog schema，
    也不把 Momentary LUFS／Sample Peak 描述成整段 Integrated LUFS、True Peak 或响度归一化结果。
  - 总输出与整段分析切片（2026-08-10）：`AdjustmentGraph` 与 Catalog `20260810.6` 增加可旁路的
    最终输出限制器，稳定保存 Ceiling 与 Release；独立 `OutputLimiter` 在淡入淡出之后、设备安全
    保护之前，以 64 帧子块预读和四相重建峰值估计驱动立体声联动增益，支持播放中 latest-wins
    参数更新、Seek 重置与实时衰减反馈。实时表与整段分析共同消费抽出的 `KWeightingFilter`，避免
    出现两套响度定义；独立后台控制器快速遍历当前调整后的完整预览，以 400 ms／100 ms 分块及
    BS.1770 绝对／相对门限计算 Integrated LUFS，并给出 4× 重建 True Peak 估计，过程中不占用
    音频设备、不阻塞 Qt 主线程。总输出面板把限制器控制、实时衰减和显式“分析”结果收在同一
    区域；结果以完整调整身份防止旧结果冒充当前草稿。本切片尚不提供响度自动归一化、标准合规
    报告、导出渲染或持久化分析缓存，True Peak 明确是预览估计而非交付认证。
  - 目标响度建议切片（2026-08-11）：独立 `LoudnessGainAdvisor` 把当前完整预览的 Integrated
    LUFS、真峰值估计、用户选择的目标与总输出 Ceiling 投影为 0.1 dB 步进的透明 Clip Gain
    建议；正增益同时受实测真峰值余量与 authored Clip Gain 范围约束，不依靠下游 Limiter
    隐式制造响度。总输出面板提供 -23／-18／-16／-14 LUFS 快速目标，明确展示建议增益、预计
    响度以及峰值或参数范围限制；用户显式“应用”才把结果写入当前草稿，并形成一条可撤销历史，
    调整身份变化后旧分析立即失效并要求重算。目标选择只是工作区暂态，不冒充平台标准、持久化
    调整或已经完成的归一化；本切片仍不包含自动批处理、导出渲染与交付合规认证。
  - 参数均衡切片（2026-08-11）：Catalog `20260811.1` 将原固定三段增益无损提升为六段参数
    均衡 authored intent；每段明确保存启用状态、Bell／Low Shelf／High Shelf／Notch 类型、
    20 Hz–20 kHz 频率、0.10–20.00 Q 与 ±12 dB 增益。旧 120 Hz／1 kHz／8 kHz 调整迁移到
    等价节点，原 revision 与 Original 均不改写。C++ `ParametricEqualizer` 成为唯一执行器，六段
    Biquad 与快速连续编辑继续在完整状态组之间做 30 ms 交叉过渡；播放中通过生产线程控制邮箱
    更新，不进入 Qt 实时回调。编辑面板以同一 C++ 系数计算真实响应曲线，提供六个可直接拖动的
    节点及所选频段的类型、频率、Q、增益精调，所有操作进入同一草稿历史、A/B、保存与整段响度
    分析合同。本切片不伪造实时频谱，也不包含动态 EQ、线性相位、卷积混响或第三方插件宿主。
  - 首个离线导出切片（2026-08-11）：独立 `OfflineWavRenderer` 复用试听的完整 authored
    adjustment 执行顺序，但明确排除只用于设备试听安全的 `OutputGuard`；渲染以固定内存流式
    输出 48 kHz、双声道、24-bit PCM WAV，并对最终量化样本重新计算 Integrated LUFS 与 True
    Peak 估计。Qt `RenderExportController` 在后台执行、节流发布进度、支持协作取消，并通过
    `QSaveFile` 原子提交，禁止目标覆盖 immutable Original。Catalog `20260811.3` 新增用户交付
    文件的 publication provenance：源 Asset、最新已保存 adjustment revision、规范化输出路径、
    格式、帧数、大小、BLAKE3 内容身份与响度证据；它与可清理／可重建的 cache artifact 分离。
    编辑器顶部提供专用导出入口与本地文件选择，草稿未保存时拒绝导出，保证结果可追溯；旧
    `20260811.1`／`.2` Catalog 均可连续原子迁移。本切片尚不包含 RF64、其他编码格式、批量
    导出、响度合规报告、交付队列恢复或文件历史管理界面。
  - 批量交付与格式管理切片（2026-08-11）：声音墙可把当前筛选结果作为一个有界批次交付，
    不要求逐条进入编辑器。批次逐条复用每段声音已保存的 adjustment revision，支持 WAV
    16-bit PCM、WAV 24-bit PCM 与 FLAC 24-bit lossless 三个明确 profile，统一输出为 48 kHz
    stereo；文件名以声音速写／嵌入标题／原文件名依次取首个可用 stem，并以 `-2`、`-3` 保留
    同名结果，永不静默覆盖 Original 或已有交付文件。批次状态与每条输入快照原子保存在 Echo
    应用状态目录；异常退出后运行中条目退回待处理并可继续，取消只停止未完成条目，单条失败不
    终止其余任务。每个成功文件仍逐条进入 Catalog publication provenance，格式、位深、内容
    身份、大小和响度证据与单条导出一致；恢复清单不是用户交付历史的第二事实来源。此切片不
    声称具备 RF64/BWF、采样率转换选项、响度合规报告、多声道交付或格式预设编辑器。
  - 首个修复链切片（2026-08-11）：`AdjustmentGraph` 与 Catalog `20260811.7` 增加可旁路的
    自适应宽带降噪与去齿音 authored intent。宽带降噪以立体声联动的电平与噪声底跟踪，只在
    低电平背景区间做软拐点向下扩展；参数与增益均以有界时间常数连续过渡，避免拖动、旁路和
    跨解码块产生点击或“滋滋”破音。去齿音以可调高频侧链检测驱动高频分量衰减，保留低频与
    主体语音；两者都位于 Low Cut 之后、参数均衡与 Dynamics 之前，并进入同一草稿历史、
    Original A/B、保存版本、整段响度分析及离线导出合同。独立 Restoration 面板只显示真实
    可试听控件，并明确首版宽带降噪擅长稳定底噪与停顿区间，不冒充能消除语音下噪声的神经
    修复；DeepFilterNet、去混响、源分离和 AI restoration 仍通过后续 InferenceBackend Intent
    进入，不塞入实时回调或 Catalog。
  - 可编排效果链切片（2026-08-11）：`AdjustmentGraph` 与 Catalog `20260811.9` 将修复、参数
    均衡、Dynamics、算法空间和总输出提升为一条有界的 authored effect chain。首版每种节点只
    允许一个实例，以稳定类型承担身份；修复、均衡、Dynamics 与空间可重排并可整节点独立旁路，
    总输出固定为链尾，Low Cut、片段增益与淡化仍是片段层处理，不伪装成可任意移动的效果节点。
    旧 revision 迁移为“修复 → 均衡 → Dynamics → 空间 → 总输出”，因此默认试听和离线导出保持
    既有顺序。C++ 播放计划按同一持久链执行，编辑器以纵向节点链选择面板、显示旁路状态并约束
    重排；所有节点参数继续进入统一草稿历史、Original A/B、响度分析、保存版本和离线导出合同。
    首版不引入没有真实消费者的多实例 UUID、分支／并行图、发送总线、第三方插件宿主或自动化
    包络；Delay 等可重复效果要等其真实执行器与清晰交互一同进入再扩展身份模型。
  - 长链编辑布局切片（2026-08-11）：声音调整继续使用“时间轨道在上、调整工作台在下”的可拖动
    纵向分割；下层收敛为“信号链在左、当前节点参数在右”的可拖动横向分割，不再让基础调整、链
    导航和参数页作为三个固定宽度栏争夺空间。片段／前级固定在信号链入口，总输出固定在链尾，只有
    中间 authored insert 节点独立滚动，因此长链仍保持入口、顺序、旁路和母线可见。裁切、淡化、
    Low Cut 与 Clip Gain 仍属于不可重排的片段层处理；此布局不引入分支图、并行路由或新的 DSP
    语义，后续只有出现真实发送／总线消费者时才增加可选节点图视图。
  - 可编辑效果目录切片（2026-08-11）：Catalog `20260811.10` 允许用户从当前有界恢复链中删除
    或重新加入修复、参数均衡、Dynamics 与算法空间单实例节点；节点仍可排序和整节点旁路，
    总输出始终存在并固定在末端。删除只改变 authored topology，不销毁该节点保留的参数；重新加入
    时沿用参数并显式启用，所有增删、排序与旁路继续进入同一草稿历史、保存版本、试听、响度分析
    与离线导出合同。效果目录只列出已有真实执行器的恢复性节点，不借“插件”外观承诺多实例、
    分支、发送总线、第三方宿主或创作型效果。C++ 执行计划以固定容量和 active count 表达可选拓扑，
    实时回调不分配；片段增益改为明确的 pre-chain 固定阶段，不再错误附着在可删除的 EQ 节点上。
    同期独立验证 DeHum 与 DeClick 两个恢复型 DSP owner；DeHum 尚待 authored 参数接入，DeClick
    还需要播放与离线渲染共享的固定延迟补偿，因此二者本切片只进入构建和算法测试，不提前显示
    无法形成试听／导出闭环的控件。
  - 延迟感知修复组件切片（2026-08-11）：Catalog `20260811.11` 在保留既有节点 wire identity
    的前提下追加 DeHum 与 DeClick 两个有界单实例修复节点。旧声音和新建默认图都不自动加入它们，
    因此迁移不改变既有试听；用户从效果目录显式加入后，DeHum 保存 50／60 Hz、谐波数、Q 与有限
    抑制度，DeClick 保存灵敏度、最大短脉冲宽度和修复混合，并各自支持删除、重排和整节点旁路。
    C++ 将链顺序、处理器生命周期、参数更新、重置和固有延迟收拢为独立 prepared effect-bus
    owner；DeClick 在 48 kHz 下固定 97 帧前视延迟，节点存在期间即使旁路也保持该延迟，避免开关
    引起时间跳变。播放、Seek、整段分析和离线导出统一丢弃链首延迟并以零输入补足链尾，最终帧数、
    时间位置、Trim 与 Fade 均继续锚定 Original 时间轴。延迟只在解码生产线程／离线渲染路径中
    补偿，Qt 实时回调仍只复制已经补偿的 PCM；本切片不引入多实例、分支、发送、插件宿主或允许
    改变延迟的实时拓扑编辑。
  - 可复用处理方案切片（2026-08-11）：Echo 将一组可复制的恢复性参数从某段声音的完整
    `AdjustmentGraph` 中明确抽为具名“处理方案”。处理方案拥有稳定 identity，内容以 immutable
    revision append-only 保存；应用时把所选 Low Cut、修复、DeHum、DeClick、参数均衡、Dynamics、
    算法空间与总输出意图确定性物化为目标声音自己的新 adjustment revision，目标声音原有 Trim、
    Fade 与片段 Gain 永远保留。默认“合并”只覆盖方案声明的处理组件；显式“替换处理链”也只替换
    处理域，不越过片段边界。单段与批量应用共同保存方案 revision、合并方式以及逐目标
    updated／unchanged／failed 收据，使一次应用可审计且不会把部分失败冒充整体成功。
    共享方案只存在于 Rust Catalog／应用层；C++ 实时与离线执行器继续只编译每段声音已经物化的
    本地 `AdjustmentGraph`，播放回调绝不查询方案库。首版没有 live-link、自动传播、任意节点图、
    多实例 UUID、参数智能适配或跨声音共享分析测量；复制后每段声音独立演进，后续更新必须由用户
    再次显式应用。
  - 处理方案管理与安全回退切片（2026-08-11）：具名处理方案继续使用稳定 identity，用户可修改
    名称、从一段声音的已保存调整显式追加新的 immutable revision，并把不再使用的方案归档；归档
    只令方案退出选择与应用入口，不删除历史 revision 或既有批次收据。声音墙增加独立于当前检查器
    主选项的多选集合，普通点击建立单选，Command 点击切换成员，Shift 点击按当前结果顺序连续选择；
    顶部方案入口仍明确作用于“当前结果”，多选浮动操作只作用于所选声音，避免一个按钮在不同选择
    数量下静默改变语义。
    每次方案应用完成后允许一次显式批次回退：只有目标声音当前 adjustment revision 仍等于该批次
    物化出的 revision 时，才把批次前的图（或从未调整时的 identity graph）追加为新的本地 revision；
    后续又被编辑、再次应用或删除的目标以 conflict／failed 逐项记录，绝不覆盖更新的用户工作。
    回退本身也持久保存批次 identity、时间和逐目标 restored／unchanged／conflict／failed 收据，数据库
    错误仍回滚整个事务。此切片不提供 redo、跨设备同步、live-link 自动传播、批量参数自适应或把
    方案 revision 直接作为播放图；所有试听、分析与导出继续只读取每段声音自己的最新调整版本。
  - 持久处理历史切片（2026-08-11）：桌面端把既有方案应用批次与一次性回退收据投影为独立的
    “处理历史”，按批次创建时间倒序显示方案当前名称、实际使用的 immutable revision、合并方式、
    目标数量以及 updated／unchanged／failed 结果；方案后来改名或归档都不删除历史，审计 identity
    仍以 recipe／revision／batch 为准。未产生回退收据的批次可从历史中再次发起同一安全回退；一旦
    回退完成，历史永久显示 restored／unchanged／conflict／failed 汇总，不提供 redo，也不把冲突
    伪装成成功。此投影直接聚合已有持久表，不复制声音参数、不修改实时链，并以有界最近记录避免
    桌面一次加载无界审计数据；逐声音诊断与跨设备历史同步留待后续切片。
  - 源锚定片段编排与效果遮罩切片（2026-08-11）：Echo 在 immutable Original 与既有恢复链之间
    增加一层有界、可逆的片段编排。分割只建立原音频时间边界；“隐藏”保留来源区间和恢复入口，
    但不进入结果时间；“静音”保留原时长并输出零信号；“插入空隙”只生成无来源静音，不把外部素材
    拼接伪装成轻量修整。每个来源片段可保存自己的增益与淡化，非连续区间连接由执行计划添加短暂
    平滑边缘，Original 本身永不改写。旧 adjustment revision 确定性投影为覆盖既有 Trim 的单一可听
    来源片段，因此迁移不改变试听或导出。
    效果遮罩始终锚定 Original 时间而不是会收拢的结果时间；一个遮罩可选择多个已有 insert 节点，
    多个遮罩重叠时仍按 authored effect chain 的串行顺序组合。某节点首次进入遮罩后只在其全部遮罩
    并集内生效，未进入遮罩的节点继续整段生效；边缘以有界 feather 做 dry／wet 平滑，处理器状态仍
    连续推进，避免遮罩开关产生点击。首版继续复用每种效果的一套参数与单实例 identity，不引入局部
    参数副本、分支图、发送总线或外部素材组合；Master 与带固有前视延迟的 DeClick 暂不接受局部
    遮罩。试听、整段分析与离线导出必须消费同一个 prepared source-edit／mask plan，并严格保持
    隐藏、静音、空隙、片段增益和局部效果的帧数及边界一致。
  - 单轨专业操作补齐切片（2026-08-30）：Echo 借用成熟波形编辑器的区域操作语法，但仍保持
    Library-first 的单资产非破坏性边界。时间选区可用 `S` 建立区域边界，`Delete` 以可恢复的隐藏
    语义闭合所选时间，`M` 保留时长静音，`R` 恢复来源；播放头无选区时也可用 `S` 精确分割。
    一个完整来源区域可打开独立检查器，原位精调区域增益、淡入／淡出时长和 Linear／Smooth／
    Equal Power 曲线；区域带直接绘制同一 authored envelope，草稿 Undo／Redo、试听、整段分析与
    离线导出继续消费既有 `EditSegment`／`SourceEditPlan` 合同，不复制另一套波形处理实现。
    Scene、Delay、Modulation、Digital Degrade、Tape、Auto-Wah 与 Stereo 等输入驱动 Creative
    节点可从当前时间选区直接建立 Original-time effect mask；Master、DeClick、Transform、Drive、
    Rotary、Freeze、Granular、Pitch 与 Beat Repeat 因终端或完整历史／固定延迟要求，在 Domain、
    C++ 执行计划和桌面投影中统一拒绝局部遮罩。此切片不把 Echo 扩张为多轨 DAW：外部素材拼接、
    region 移动／复制粘贴、交叉淡化、自动化关键帧、录音编排、发送总线和第三方插件宿主仍不在
    当前单轨修复工作区的产品合同内。
  - 爆破音修复切片（2026-08-11）：`AdjustmentGraph` 与 Catalog `20260811.15` 在既有
    Restoration 节点内增加默认旁路的 De-plosive authored intent，稳定保存低频边界、灵敏度、
    最大抑制度与释放时间；它在宽带降噪和 De-esser 之前处理话筒近讲产生的短促低频爆发，继续随
    Restoration 节点参与重排和 Original 时间遮罩，不增加新的链拓扑 identity。独立 C++ owner
    以立体声联动的快／慢低频包络和宽带占比识别瞬态，只衰减分离出的低频分量；参数、启停和旁路
    都逐样本平滑，所有工作区在构造时分配，算法零前视、零固有延迟。试听、实时参数更新、整段
    分析与离线导出继续复用同一 effect-chain 执行器；离线 24-bit 结果与关闭设备保护后的试听 PCM
    有量化误差内的一致性回归。同期移除 De-esser 的块内临时分配，Qt 音频回调仍只复制已处理
    ring buffer。本切片不冒充去风噪、去混响、神经修复或任意多实例插件，也不改变声音空间、卡片、
    全局颜色和图标体系。
  - 声道修复切片（2026-08-11）：`AdjustmentGraph` 与 Catalog `20260811.16` 追加默认不进入
    authored chain 的单实例 Channel Repair 节点，保存左右极性反转、声道交换、单声道折叠与
    `-100..100` 平衡意图；旧 revision、默认声音和既有处理方案迁移后保持原试听。独立 C++ owner
    将所有参数合成为 `2×2` 声道矩阵，按“极性 → 交换 → 无增益平衡 → 双声道居中单声道”顺序
    执行，并以 20 ms 逐样本过渡处理参数、启停与旁路，实时线程不分配。节点进入统一草稿历史、
    效果目录、遮罩、处理方案、实时播放、整段分析和离线导出；首版不把立体声宽度、空间化、
    相位旋转或创作型声像自动化混入恢复工具。
  - 算法空间角色数据面切片（2026-08-12）：Catalog `20260812.1` 在既有单实例 Space 节点中
    追加稳定的 Room／Hall／Plate authored character；旧 revision 缺字段时确定性恢复为 Room，
    迁移只前移 schema identity，不重写历史 JSON 或 Original。Room 冻结既有四延迟算法与数值
    行为，Hall／Plate 由独立 `DiffuseSpaceReverb` owner 以双声道输入扩散、八路 Householder FDN、
    RT60 反馈和湿声带通形成两套真实拓扑调音；实现为 Echo clean-room 代码，不复制或链接
    JUCE／Dragonfly／DaisySP ReverbSc／Soundpipe RevSC。构造阶段一次性分配，逐帧路径零分配，
    character 或其他参数变化继续在完整引擎间做 50 ms 过渡；三者报告零处理延迟，Pre-delay 与
    湿声首达只属于效果内容，不进入 DeClick 时间线补偿。Catalog、桌面只读投影、轻量播放、
    实时试听、整段分析及单次／批量离线导出共用同一 character；桌面保存入口现已显式携带并验证
    character，草稿变更可沿统一发布合同写入 Catalog。Space 面板以 Room／Hall／Plate 三段选择器
    调用同一 draft setter，选择进入 Undo／Redo、dirty、保存和重载；面板与信号链标题共享统一几何。
    Spring
    必须由色散／模态独立 owner 实现，卷积与 IR 资产许可、哈希和延迟合同也另立里程碑，二者都
    不伪装成 FDN preset 或 Restoration。
  - 首个 Creative VFX 数据与执行切片（2026-08-12）：Catalog `20260812.2` 将场景滤镜、延迟、
    调制与角色化变声保存为四个稳定、可独立旁路的 authored singleton 节点；旧 revision 迁移后
    四者全部关闭，既有链顺序、听感与 Original 均不改变。场景覆盖 Telephone／Radio／Intercom／
    Behind Wall／Underwater，延迟覆盖 Slapback／Echo，调制覆盖 Chorus／Flanger／Phaser／Tremolo，
    角色化覆盖 Robot／Monster／Tiny／Giant／Ghost；最后一族明确是夸张听感而非身份克隆或自然
    换声。每个家族都有独立 DSP owner、类型化参数与无点击更新；构造时完成有界缓冲分配，实时
    process/update 不分配，静音不生声。Scene、Delay、Modulation 报告零基础设施延迟；Transform
    固定 50 ms 并与 DeClick 一同由共享 effect-chain 做顺序无关的时间线补偿，固定延迟节点不接受
    Original 时间遮罩。试听、整段分析、单次／批量离线导出和处理方案都消费同一执行合同；桌面
    投影提供完整 Creative map 与 live update 入口，通用工作台以四个独立 Creative 节点接入效果
    目录、排序、旁路、草稿历史、处理方案和专属参数页，并保持一次家族调参只生成一个 Undo 手势；
    Original 试听只旁路 Creative，不丢弃已选角色和参数。本切片为 Echo clean-room 实现，不引入 JUCE／Dragonfly／
    DaisySP ReverbSc／Soundpipe RevSC／Signalsmith Stretch，也不进入 Restoration、AI 或 Runtime。
  - 确定性空间与数字劣化切片（2026-08-12）：Catalog `20260812.4` 在 Listening continuity
    `20260812.3` 之后为既有单实例 Space
    character 追加稳定 wire 值 `3` 的 Spring；它不复用 Hall／Plate FDN，而由独立
    `SpringSpaceReverb` 以三路色散波导、稳定全通
    节、回程高频损耗与正交双声道编解码形成真实弹簧听感。它复用 Space 的 Mix／Pre-delay／Decay／
    Size／Damping／wet cuts 意图，报告零基础设施延迟，传播时间只属于湿声内容；构造时准备固定
    工作区，实时 process／update 不分配，结构变化以完整状态交叉过渡。Creative JSON 同时追加
    默认关闭的独立 Digital Degrade singleton：Bitcrusher、Sample-rate reduction 与显式同时消费
    两组 typed settings 的 Lo-Fi；处理仅由输入驱动，不加 dither、hiss 或其他独立声源，报告零
    基础设施延迟并支持 Original 时间遮罩。旧 Catalog revision 缺少新 Creative 字段时确定性恢复
    为 disabled，既有 Room／Hall／Plate wire、旧 Creative 听感、链 active count 与 Original 均不变；
    试听实时更新、共享 effect-chain、整段分析、离线导出与处理方案使用同一合同，专属控件由 UI
    owner 接入。卷积由下一独立确定性里程碑先交付 owned source store、BLAKE3 source／prepared
    identity、append-only IR provenance／license、离线 48 kHz preparation、可验证的无分配 runtime
    bank 与明确失败语义；不得只增加一个 IR 路径控件，也不得把 2ch dual-mono 宣称为 true-stereo。
    Freeze／Granular 因静音后持续生成内容和尾音边界不同，不纳入本输入驱动切片。
  - 卷积空间底层资产与执行切片（2026-08-13）：新增独立 `echo-ir` 生命周期 owner，将 exact source
    WAV 以 BLAKE3 内容地址保存到不可逐出的 durable store；同一 source 只存一份，每次导入仍追加
    独立的标题、作者、出处与用户声明 license 记录。发布顺序固定为 source object → canonical
    preparation cache → provenance event，失败最多留下可延后 GC 的孤儿对象，不留下指向缺失 source
    的记录。离线 preparer 仅接受 classic RIFF/WAVE 的 PCM16／24／32 或 float32、mono／stereo、
    8--192 kHz，以及声道掩码明确的常见 WAVE_FORMAT_EXTENSIBLE；RF64、压缩 WAV、歧义 layout、
    non-finite、数字静音、超过五秒或超过 32 MiB 的输入 fail closed。结果保留幅度、前导静音和
    accepted tail，以带 preparation／avcodec／swresample identity 的 portable header 与 planar
    float32@48 kHz 写入可重建 cache；cache 删除后可从 owned source 重建为相同 prepared hash。
    runtime 使用固定 pin 的 MIT `FFTConvolver` core 与 MIT Signalsmith FFT adapter，未复制其
    Ooura-derived AudioFFT；构造阶段完成分区 FFT 与全部分配，process／update／reset 零分配、
    零锁、零基础设施延迟。v1 支持 mono IR 驱动 L／R 或 2ch stereo-parallel L→L／R→R，明确不称
    true-stereo；稳定旁路停止 FFT 并清空 history，重新启用从空 bank 淡入。Catalog `20260813.2`
    保存 immutable import identity、source／prepared hash、append-only rights/provenance 与 authored
    Algorithmic／Convolution Space mode；桌面以异步本地 WAV 导入完成 source → preparation → provenance
    闭环，用户必须显式声明 SPDX 或“本人拥有且不再分发”，失败只展示有界错误而不发布半成品。
    worker 构造并验证完整 ready bank，音频块边界发布；IR identity 切换显式采用旧 wet fade-out →
    history reset → 新 wet fade-in，不假称保留切换前卷积历史。实时播放、整段响度分析、单次与批量
    离线导出消费同一 Space 参数和 validated artifact，导出仍沿用 authored frame count 并截断尾音。
    true-stereo、云端 IR 浏览、bundled 第三方 IR 与 include-effect-tail 均不在本切片。
  - 驱动与旋转扬声器切片（2026-08-13）：Catalog `20260813.1` 在 Creative map 与 authored
    chain 尾部追加默认关闭的 Drive 和 Rotary 两个独立 singleton。Drive 提供 Soft Clip／
    Overdrive／Fuzz、干湿比、驱动量、音色与输出增益；独立处理器以二倍过采样和固定 FIR 抑制
    非线性混叠，并始终报告 32 帧基础设施延迟，使启停、旁路和链内重排不会跳时。Rotary 提供
    Slow／Fast／Brake、干湿比、运动量与立体声宽度；独立处理器以低／高频分路、两组受约束转速、
    幅度调制和有界分数延迟形成通用旋转运动，不冒充任何品牌箱体，基础设施延迟为零。两者都在
    构造期准备工作区，实时 update／process 不分配，静音不生声；首版因 Drive 固有延迟和 Rotary
    的连续运动状态均不接受 Original 时间遮罩。试听、整段分析、离线导出、Catalog revision、
    处理方案与专属 Creative 参数页共享同一类型化设置；旧 `20260812.4` revision、Listening 状态、
    Creative JSON 和 effect-chain bytes 在迁移时不改写，缺失字段只在读取时恢复为 disabled 默认值。
  - 冻结／颗粒与真立体声空间切片（2026-08-14）：Catalog `20260813.4` 为 Creative chain
    追加默认关闭的 Freeze 与 Granular。Freeze 保存源时间锚定的频谱捕获点和干湿比，并由执行层
    预读固定历史；Granular 保存颗粒大小、密度、回看、散布、音高、声像与确定性种子，使试听与
    导出可重复。两者均是明确创意处理，不生成新的来源材料；由于各自依赖完整源历史，首版不接受
    局部效果遮罩。Catalog `20260813.5` 同时把 convolution IR preparation 明确分为 mono、
    stereo-parallel 与 `LL/LR/RL/RR` true-stereo：用户导入四声道 WAV 时必须显式选择后者，
    Echo 从不依通道数猜测布局。v2 true-stereo 以四条卷积路径实现 `wetL=LL(L)+RL(R)`、
    `wetR=LR(L)+RR(R)`；既有 mono/stereo IR 仍保持原路由与字节身份。Creative 目录、草稿
    历史、处理方案、实时试听与离线导出已消费 Freeze／Granular；Space 的导入界面会显示 IR
    实际布局与来源／许可证据。此切片不包含云端 IR 浏览、捆绑第三方 IR、生成式环境声，或效果尾音
    超出 authored 声音时长的导出。
  - 输入驱动 Creative VFX 扩展（2026-08-15）：Creative chain 追加默认关闭的 Tape、Pitch 与
    Auto-Wah 三个独立 singleton。Tape 只以既有输入产生饱和、wow/flutter 与衰减式 dropout，不添加
    独立噪声源；Pitch 提供固定因果延迟的移调、可选和声与明确标为 spectral colour 的 formant colour，
    不作特定人声身份保持或模仿声明；Auto-Wah 是包络跟随共振滤波。三者都拥有类型化 JSON、处理方案、
    Runtime／离线执行、桌面目录、草稿历史和中文面板；旧 revision 缺失字段恢复为 disabled defaults。
    Pitch 因固定延迟不能使用局部 effect mask，Tape 与 Auto-Wah 可遵循普通可遮罩 insert 语义。
  - 立体声、节拍重复与本地 Creative 快照（2026-08-15）：Creative chain 追加默认关闭的 Stereo
    与 Beat Repeat 两个独立 singleton。Stereo 是零延迟的 mid/side 宽度和等功率声像处理，不把单声道
    信号伪装成立体声；Beat Repeat 使用预分配的源历史、30–500 ms 切片、1–4 次重复与可选反向读出，
    固定报告最多两秒的因果延迟，因此不能使用局部 effect mask。两者的类型化 JSON、处理方案、实时
    试听、整段分析、离线导出与桌面面板使用同一范围约束，旧 revision 缺失字段恢复为 disabled defaults。
    Creative 页还可将一个已验证的 Creative 参数快照与 authored effect chain 保存为最多 32 个本地命名
    预设；它们只保存在应用设置中，应用时仍是当前声音的一次可撤销调整，不创建通用预设系统、不写回
    Original，也不保存生成式来源。
  - Creative 场景与 Delay Ducking（2026-08-15）：Creative 页提供 Voice memo、Night drive 与
    Dream voice 三个可撤销的参数宏；它们只应用既有确定性 VFX 参数与节点，不保存 `preset_id`，
    不创建生成式声源。Delay VFX 增加默认关闭的输入包络 Ducking，使用被处理源自身的响度按
    Amount／Attack／Release 降低湿声，既不接收外部 sidechain，也不改变 Echo feedback；类型化
    JSON、实时试听、离线导出和桌面面板共享同一范围约束。Reverb 仍通过旧固定参数保存通道，
    因此没有在本切片暴露半持久化的 Ducking 控件，留待专门的契约迁移。
  - Reverb Ducking 契约迁移（2026-08-15）：算法 Space 的 Reverb 现以同一输入包络语义提供
    默认关闭的 Ducking；Amount／Attack／Release 只缩放输出中的湿声贡献，不改变 Room、Hall、
    Plate 或 Spring 的内部反馈。领域设置、Catalog JSON、桌面 ABI、播放／导出投影与 Space 面板
    都保留同一组有界字段；旧 revision 缺失字段恢复为 disabled 默认值。桌面继续使用已有的原子
    Space 参数 map 承载该算法空间专属控件，而不是延长历史固定参数信号。
  - 频谱修复工作区 P0（2026-08-15）：Echo 从 immutable Original 解码为 canonical 48 kHz mono，
    用 `2048` 帧 Hann STFT／`512` 帧 hop 生成最多 `1024×128` 的有界频谱概览；概览以版本化 JSON
    payload 写入 content-addressed cache，Catalog 仅保存可重建的 derived-artifact reference，损坏或
    schema 不兼容时隔离并重建。桌面通过单一 Rust/CXX/Qt 投影消费该概览，在编辑工作区显示与时间轴
    同一 viewport、选区和播放头的 Original 只读频谱图。领域层定义 source-time + Hz + attenuation
    + feather 的至多 64 个参数区域；桌面草稿、Undo/Redo、Catalog JSON 与原生投影完整传递这些语义，
    频谱图上的拖拽新增一个可清除的默认柔和衰减区域。P1 的 `2048/512` Hann STFT 核既提供独立离线
    处理，也提供跨解码块保持重叠历史、seek 时重置、结束时只补齐真实源时长的 producer-thread stream；
    它在 `SourceEditPlan` 之前进入共享 `PlaybackSession`，因此实时试听、离线 WAV/FLAC 导出和响度分析
    使用同一 source-anchored 语义。合成正弦覆盖区域内衰减、区域外保持、任意分块和尾部时长；离线导出
    契约覆盖真实消费链。画笔、修补、细粒度选择编辑和更丰富的区域参数面板仍须作为后续单独契约完成，
    不得把显示 cache 或像素坐标伪装成用户修复事实。
  - 频谱工作层 P2（组合合同，待实现）：需要“擦除／克隆／修补”等破坏性频谱工具时，用户必须先显式
    创建一个资产唯一的 `SpectralWorkingLayer`；它不是每笔操作新建的 Adjustment，也不是可写 Original。
    它只可锚定 immutable Original 的内容 identity，故可听结果的首版顺序固定为
    `Original → SpectralWorkingLayer → 既有非破坏性调整／SourceEditPlan → Output`。这使均衡、修复、
    动态、空间、Creative VFX、母带及时间编辑全都位于工作层之后：它们仍可独立修改、旁路或重排，不会因
    已有破坏性频谱笔刷而要求用户重置、冻结或拆分前置调整。工作层内部可连续进行破坏性笔刷与修补，并在
    局部 copy-on-write 时频 tile 上合并；产品默认只把它呈现为一个可旁路、可整体移除、可保存版本的层，
    不以每一笔污染信号链。它保存 Original content identity、tile manifest、工具／算法版本和必要的笔画
    证据；预览／完整 render 是可删除的 derived cache，不能反过来成为编辑事实。AI／非确定性修补始终先
    产出 Candidate，只有显式 Accept 才能合并进该工作层。首版不支持任意层插入、多个活动工作层、自动
    rebase 或跨资产复制；试听 A/B 至少提供 Original、工作层结果与当前完整结果，导出 provenance 同时
    记录 Original identity、working-layer manifest 和下游 adjustment revision。
  - 渲染后频谱工作副本（独立合同，2026-08-15）：绝大多数修复应在 VFX 之前完成；若用户确实要
    擦除某个 VFX 或混音渲染所产生的伪影，不能把该操作塞回 Original-first `SpectralWorkingLayer`。显式的
    `RenderedSpectralWorkingCopy` 以内部 render 的内容 identity 和精确上游 adjustment revision 为不可变
    parent；创建仅接受仍为当前 revision 的 render，改动该 parent 后旧副本保留并明确标为不可用，绝不静默
    rebase。桌面在后台把当前已保存的完整调整渲染到 content-addressed private cache；副本有不可变 parent
    cache identity 和随确定性擦除提交原子替换的 current cache identity。用户在显式 Erase mode 中框选时，会
    将至多 512 个带时频范围、96 dB 衰减及 feather 参数的操作追加到同一 manifest，而不是每笔创建 Adjustment；
    当前 cache 和 manifest 一起提交。频谱预览可切换至该 cache，且提供明确的 rendered audition；整体旁路、
    整体移除、上游失效与重新冻结均保持显式。交付时可明确选择渲染修复副本；publication provenance 快照
    Original identity、working-copy parent/current render、manifest 与工具版本，因而后续擦除不会重写已交付
    文件的来源。上游失效后界面明确提供“冻结新的工作副本”。首版采用完整缓存重渲染而非虚假的像素/时频
    tile COW，因而不能声称局部编辑的性能优势；这个分支不属于首版修复工作层，也不应以“任意位置插层”
    弱化可追溯性。
  - 受约束参数工作台切片（2026-08-11）：编辑页保持“时间轨道在上、信号链在左、选中节点参数
    在右”的结构，但不再要求每个面板横向铺满窗口。信号链使用窄而稳定的轨道，普通恢复页、
    Dynamics、Space 与 Master 各自采用与内容匹配的可读宽度；只有 EQ 响应图获得更宽的可视区域，
    其滑块仍保持短而可精调。参数卡紧邻所选链节点，剩余空间作为安静背景；时间轨道与下方工作台
    的比例继续可拖动，底部 A/B 试听收敛为紧凑控制条。此切片只调整 presentation，不改变草稿、
    Undo/Redo、遮罩手势或 DSP 所有权，也不以更多全宽容器制造专业感。
- **M4 Audio Space**：声音相册：时间、人物、地点、声音类型、Revisit。
  - 声音带首个切片（2026-08-31）：Audio Space 把当前集合、搜索、排序和复合筛选所得的可播放
    Asset 投影为一盘暂态“声音带”，按结果顺序虚拟首尾相接；统一带头显示全局／单段时间，支持
    点击段落、跨段拖动和自然播完后连续进入下一段，实际试听仍逐段消费各 Asset 当前已保存的
    adjustment revision。切换集合或筛选会重建投影，声音带本身不持久化、不渲染长文件、不允许
    重排、叠加或混音；用户可把当前声音显式送入独立声音编排，双击段落仍进入单声音详情。
  - Revisit 首页首个切片（2026-08-13）：Audio Space 默认进入独立的“重温”入口，以 Catalog
    生成有界、来源锚定的导航快照，而不是在 QML 中重新推断用户历史。首页分开展示继续聆听、
    用户确认的声音相册、往年今日、最近聆听和资料库新内容；继续位置仍使用 Original／源时间，
    往年今日只比较原始录制日期的月日，用户相册封面只使用真实成员声音的波形，不制造照片或
    AI 封面。缺失原始文件不进入这些可播放集合，继续聆听与最近聆听互斥，各区均有固定上限；
    桌面层只将稳定 Asset ID 解析为已加载的声音墙投影。用户从重温页开始搜索时显式返回整个
    资料库，避免把首页推荐误解为搜索边界。本切片不依赖 Infer Runtime，不新增用户画像或隐藏
    的模型排序，也不宣称解决超大资料库的完整分页；后续应让相同 Catalog owner 提供游标化墙面
    投影，而不是扩大 Revisit 快照。
  - 聆听连续性首个切片（2026-08-12）：Catalog `20260812.3` 在用户状态中持久保存最近聆听时间
    与 Original／源时间锚定的恢复位置，和 Like、评分、Analysis、Adjustment revision 分离；声音墙
    增加“最近聆听”集合，卡片波形以克制的已听进度提示长期录音，检查器和单声音视图从同一位置
    继续。只有真实播放满 3 秒才产生用户事实，播放中每约 5 秒及暂停／停止时做有界 checkpoint；
    录音至少 60 秒、已听至少 10 秒且仍剩至少 10 秒并未超过 95% 时才保留恢复点，接近结尾自动
    清空，但最近聆听事实继续存在。该状态不依赖 Infer Runtime，不把试听写入 AI 证据，也不改变
    Original、处理方案或非破坏性编辑图。
  - 用户声音相册首个切片（2026-08-11）：Catalog 将用户相册与成员关系保存为独立 UserState
    事实，名称、封面声音、成员身份和修改时间不依赖可重建 Analysis。Audio Space 左侧把用户相册
    与 AI 建议相册分区呈现；用户可新建、重命名、删除相册，并从所选声音的浮动操作栏显式增删
    成员。AI 建议仍由时间、地点、事件和人物证据动态投影，情绪不参与相册规则；“保存建议”在
    单一事务中创建用户相册并快照当时的成员，后续模型重分析不会静默增删已确认成员。本切片不做
    人物身份合并、地理邻近聚类、规则持续同步、共享相册或跨设备同步。
- **M5 Sound Assembly**：从 Library 资产创建独立的多轨声音编排，保存不可变 revision，试听和
  离线导出消费同一准备／混合合同，并把每个 Clip 的 Original 与调整版本写入交付 provenance。
  - 首个完整编排模块（2026-08-31）：`SoundAssembly` 是独立于 `AdjustmentGraph` 的用户文档；
    一个文档包含最多 8 条有序音轨和 256 个 Clip，Clip 引用 Library `AssetId`、明确的 adjustment
    revision、该 revision 线性结果中的来源区间，以及编排时间位置、增益、声像、淡入／淡出和静音
    状态。轨道保存名称、增益、声像、Mute／Solo；Master 保存增益与可旁路 limiter。每次保存追加
    immutable assembly revision，旧 revision 与已经发布的 Mixdown 含义不随后续编辑改变。
    编排总长限制为 4 小时，使合法文档的 48 kHz stereo PCM24 输出始终落在 classic RIFF/WAVE
    的 32-bit data chunk 上限内；支持 RF64 前不接受只能保存、不能交付的超长文档。
  - 资料库多选可按“顺序”或“分层”创建编排；编排工作区提供文档列表、轨道增删／重命名、Clip
    移动、跨轨、复制、分割、裁剪、删除、增益／声像／淡化、轨道 Mute／Solo、时间轴缩放／滚动、
    播放头定位、完整 Undo／Redo、显式保存版本和 WAV Mixdown。重叠 Clip 作为普通可叠加声音参与
    混合；用户用两侧淡化形成 Linear／Smooth／Equal Power crossfade，不引入隐式顶层覆盖规则。
  - 每个唯一 `(AssetId, adjustment revision)` 在非实时准备阶段确定性渲染为私有 canonical 48 kHz
    stereo 来源；编排播放回调只从预混 SPSC ring 读取，离线 Mixdown 使用同一 `AssemblyPlan`、Clip
    包络、轨道／Master 增益、声像和 limiter。准备、播放与导出均可取消，失败不会发布半成品；输出
    经原子提交后才记录 assembly revision、所有 Original content hash、调整 revision 与最终文件
    hash。缺失 Original 或失效 revision 必须显式阻止准备，不静默改用最新版本。
  - 本里程碑不提供输入监听／录音、MIDI、节拍与速度网格、拉伸、插件宿主、发送总线、任意路由、
    Track effect rack、自动化关键帧或视频同步；这些不能通过空控件或被动 schema 预建。外部声音先
    作为普通资产进入 Library，编排结果作为派生交付存在，不改写任何输入 Original。
- **M6 Memory Contract**：只读 memory/render API 向上层开放（echo://asset/{uuid} 契约族；Shadow/Video 同契约，各自实现）。

### 当前状态校准（2026-08-13）

Echo 处于 **M0 已收口、M1 Runtime 音频证据链与长录音分层理解完成、M2 声音墙已具备
用户相册、可解释 AI 聚合和首个 provisional 自然语言检索、M3 已形成可试听、可测量、可保存版本并可离线导出的基础非破坏性处理
闭环并建立独立 Creative VFX 数据与执行域、M4 开始把 AI 相册候选提升为由用户确认的声音相册体验**。人物、地点、声音类型仍是模型
提示或展示维度，不应被描述为已经具备完整识别和关系系统；用户保存建议只确认当时的成员快照，
不反向确认人物身份或地点关系。M4 的成熟仍需要音频信号语义索引、人物关系和更完整的 Revisit
体验。

M5 的完成标准不是出现多条空轨道，而是一个编排可以从 Library 创建、保存、退出后重新打开、
试听、修改并原子导出；相同 revision 的试听与 Mixdown 必须使用相同 Clip／轨道／Master 数学，
Catalog 能完整解释输出引用的每个 Original 与 adjustment revision。

不把直接模型调用的数量当作里程碑进度；当前 text-evidence embedding 只能描述为 provisional
自然语言检索，不把它夸大为原始声音语义搜索；在 M3 前，不在实时播放路径加入任何 AI effect。

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

```text
从 Library 选择多段真实录音
→ 创建顺序或分层声音编排
→ 移动、裁剪、叠放并用淡化形成 crossfade
→ 保存 immutable assembly revision
→ 退出并重新打开同一版本
→ 试听与 WAV Mixdown 使用同一混合计划
→ Catalog 可追溯每个 Clip 的 Original 与 adjustment revision
```

## 11. 工程约定

- **平台**：先跑通 macOS（Apple Silicon）；架构上不为 Windows 设障碍，迁移成本应可控（Qt/C++/Rust 均可移植，MLX 只存在于 AI 层并被 InferenceBackend 隔离）。
- **License**：MIT。
- **测试拓扑**：私有不变量测试紧邻 owner（`<owner>/tests.rs`，owner 文件以 `#[cfg(test)] mod tests;` 收尾）；跨模块契约在 crate facade 的 `src/tests/<responsibility>_contract.rs`；crate 级 `tests/` 只放消费公开 API 的黑盒契约。
- **Catalog schema**：唯一规范格式为 `YYYYMMDD.N`（日期.当天版本号，例如 `20260809.4`）；catalog 打开当前 revision，或将明确支持的紧邻前序 revision 原子迁移到当前版本；不把无点整数编码暴露为产品或持久化身份。
- **格式**：Rust 用仓库 `rustfmt.toml`；C++/ObjC++ 用 `.clang-format`。
- **i18n**：英文原文为 canonical 消息身份，简体中文必须是完整产品呈现（与 Shadow 相同契约），技术 token 不翻译。
- **构建产物**：Cargo target / CMake build 目录放在仓库外的 `.echo-local-*`，不污染 worktree。
- **提交**：按里程碑自主提交；每次提交应可构建。
