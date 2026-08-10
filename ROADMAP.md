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

仓库内曾用于概念验证的 MLX/Ollama/脚本执行已退出生产链路；Echo 不再扩充模型选择 UI、
物理模型路由、下载器或驻留进程管理。正式接入采用 Infer Build 已有的
`audio.transcribe` / `audio.align` 任务接口，不预建没有消费方的第二套调度机制。

正式 consumer 固定使用 Infer Runtime `0.1.0-candidate.2` 合同，以独立、非资源管理员的 Echo
App 身份调用。endpoint 选择顺序固定为显式 `ECHO_INFER_ENDPOINT`／产品设置 override、
`infra.discovery.registration@20260810.1` 中精确匹配的 `infer-runtime.consumer` loopback offer、
迁移期 `http://127.0.0.1:8787` 兼容兜底；generation 变化、lease 到期或连接失败后重新发现。
所有 Consumer HTTP 请求禁用 proxy 与 automatic redirect，且只接受 canonical numeric loopback
origin。固定端口兜底仅在 Runtime publisher 新版本完成重启与 soak、所有登记 Consumer 均完成
适配后删除。Echo 只从自身的 owner-only 安全存储读取 bearer token；明文不跨 Rust/C++/QML
边界，也不写入设置、通用日志或命令行参数。Runtime consumer 独立拥有合同探测、严格
multipart、HTTP status／`error.code` 分类和 Job snapshot 解码；
Echo 的后台 worker 只提交产品 Intent、记录本地任务状态并把 Runtime 的 Job／Attempt／
模型构建证据写入 Catalog。Runtime 不可用不得阻断扫描、波形、播放或 Library 浏览。

文本理解沿用同一个受管 Consumer 身份，但由独立的 Responses 协议 owner 负责。首个产品动作
固定映射到 `text.summarize`：`audio.align` 成功后，Echo 的后台队列提交有界文字与产品指令，
要求模型返回 contextual JSON，再由 Echo 做结构、数量和长度校验。Runtime 当前未冻结 JSON
Schema structured-output 合同，因此格式不合格必须作为稳定的派生元数据失败记录，不得猜测、
修补正文或回退为裸 Ollama／MLX 调用。成功结果与音频结果一样，必须先核验 App-scoped Job、
local-first／local_only／background／no fallback 约束，再写入分析证据。

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
  中间以网格并行浏览，右侧是所选声音的详情检查器；双击声音或切换视图后，中间区域原位进入
  单声音聆听视图，底部胶片带继续消费同一份筛选结果，左侧资料库与右侧详情不退出。
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
- 桌面窗口只保留一条与 Shadow 同构的融合标题工具栏：品牌、工作区导航、当前工作区工具、
  设置与原生窗口拖动共享一个 chrome owner；内容区域不得重复绘制第二条伪标题栏。
- 当前集合、结果计数、内容搜索、网格／单声音视图与卡片密度属于内容呈现状态，沿用 Shadow 的
  系列结构放在中间内容区顶部工具栏，不占用融合标题栏；卡片尺寸滑块以接近目标宽度的列数排布，
  默认应形成紧凑并行浏览，而不是宽卡片。排序与筛选留在跨工作区底部工具栏；Like、评分属于所选
  声音的用户操作，放在画布右下浮动工具栏。浮动工具栏不重复提供“展开”，双击卡片或顶部视图切换
  是进入单声音聆听视图的入口。
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
  - 已完成（正式 Runtime 切片，2026-08-09；Discovery 迁移 2026-08-11）：Echo 以独立
    非管理员 App 身份消费 `0.1.0-candidate.2`，并通过 owner-only leased registration 发现本机
    Consumer endpoint；后台队列依次提交 `audio.transcribe` 与 `audio.align`，读取 App-scoped
    Job/Attempt，并把合同版本、provider/deployment、physical model/build 与稳定错误码写入
    Catalog。桌面读取层以最新对齐证据细化段落时间，无法可靠匹配时保留原转写时间。
  - 默认分析策略（2026-08-09 校准）：新录音完成注册后持久入队 waveform 与
    `audio.transcribe`；应用启动时为已有但缺少 transcript 证据的在线录音做幂等回填。
    结构性扫描、导入和 waveform 优先于 ASR；ASR 失败不得影响 Original、播放或浏览。
    手动分析只作为失败重试/调试入口，不是正常产品路径。非空文字完成 `audio.align` 后，
    继续以最低队列优先级提交 `text.summarize` contextual 元数据；该默认仍不扩张到
    SenseVoice、diarization、embedding 或 TTS。
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
  - 待办：长录音代理／切片、SenseVoice 能力 Intent、speaker/event 证据；音频合同能在
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
- **M4 Audio Space**：声音相册：时间、人物、地点、声音类型、Revisit。
- **M5 Memory Contract**：只读 memory/render API 向上层开放（echo://asset/{uuid} 契约族；Shadow/Video 同契约，各自实现）。

### 当前状态校准（2026-08-11）

Echo 处于 **M0 已收口、M1 Runtime 音频证据链完成并开始接入 contextual、M2 声音墙进入
可解释的 AI 聚合与相册候选阶段、M3 已形成可试听、可测量、可保存版本并可离线导出的基础
非破坏性处理闭环**。Audio Space 已经形成首个可用垂直界面，但人物、地点、声音类型
仍是模型提示或展示维度，不应被描述为已经具备完整识别和关系系统。M4 表示声音相册体验
成熟，而不是首次出现 Audio Space 页面。

不把直接模型调用的数量当作里程碑进度；在 embedding／语义索引落地前，不把 transcript
包含匹配或关键词 Facet 描述成语义搜索；在 M3 前，不在实时播放路径加入任何 AI effect。

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
- **Catalog schema**：唯一规范格式为 `YYYYMMDD.N`（日期.当天版本号，例如 `20260809.4`）；catalog 打开当前 revision，或将明确支持的紧邻前序 revision 原子迁移到当前版本；不把无点整数编码暴露为产品或持久化身份。
- **格式**：Rust 用仓库 `rustfmt.toml`；C++/ObjC++ 用 `.clang-format`。
- **i18n**：英文原文为 canonical 消息身份，简体中文必须是完整产品呈现（与 Shadow 相同契约），技术 token 不翻译。
- **构建产物**：Cargo target / CMake build 目录放在仓库外的 `.echo-local-*`，不污染 worktree。
- **提交**：按里程碑自主提交；每次提交应可构建。
