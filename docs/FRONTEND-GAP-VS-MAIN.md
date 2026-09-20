# 前端功能差距盘点：`rewrite/tauri-svelte` vs `origin/main`

> 基线 = `origin/main`（Python / NiceGUI + FastAPI 版）。
> 对比对象 = `rewrite/tauri-svelte`（Rust + Svelte 5 + Tauri 2 重写版）。
> 本文只做盘点，不改 `main`、不动后端与接口契约。
> 所有结论均来自实际读取仓库文件，行号以当前 checkout 为准。

---

## 0. 基线事实确认（技术栈 / 目录 / 命令）

| 项目 | `origin/main` | `rewrite/tauri-svelte` |
| --- | --- | --- |
| 语言/框架 | Python 3 + NiceGUI + FastAPI | Rust + Svelte 5 + Tauri 2 |
| 前端载体 | `app.py`（UI 与 REST 同文件，515 行） | `src/App.svelte`（单文件 672 行） |
| 业务/数据层 | `webapp/service.py`、`cxapi/*` | `src-tauri/src/api/*` |
| 包管理器 | `uv`（`uv.lock`、`pyproject.toml`） | `npm`（`package-lock.json`，CI 用 `npm ci`） |
| 前端构建 | 无（服务端渲染 HTML） | Vite 8 + Tailwind 4 |
| 启动命令 | `python app.py`（uvicorn :2333） | `npm run dev`（Vite :1420）、`npm run tauri dev` |
| 构建命令 | 无 | `npm run build` |
| 类型检查/lint | 无 | **无脚本**（`jsconfig.json` `checkJs: false`，仓库内无 eslint/prettier/svelte-check） |
| CI | 无 | `.github/workflows/ci.yml`：`npm ci` + `npm run build`、`cargo check`、Windows `npm run tauri build -- --bundles nsis,msi` |

**页面/路由（重要）**：`main` 只有一个页面 `@ui.page("/")`（`app.py:118`），没有多路由、没有前端路由库、没有 `href`/`ui.navigate`（已 grep 确认 0 处命中）。
→ 因此"已有 URL 路径保持不变"对本仓库的约束是：**页面入口 `/` 不变**；Tauri 侧对应的是 `index.html` 单入口（`src/main.js` 挂载 `App.svelte`），`tauri.conf.json` `devUrl=http://localhost:1420`、`frontendDist=../dist` 均不改。

**`main` 的 REST 表面**（`app.py:57-115`，8 个端点）：

| 方法 | 路径 | 行号 |
| --- | --- | --- |
| GET | `/api/sessions` | `app.py:57` |
| POST | `/api/login/session` | `app.py:62` |
| POST | `/api/login/password` | `app.py:70` |
| POST | `/api/login/qr/start` | `app.py:78` |
| GET | `/api/login/qr/poll` | `app.py:86` |
| GET | `/api/classes` | `app.py:94` |
| POST | `/api/tasks/start` | `app.py:102` |
| GET | `/api/tasks/{task_id}` | `app.py:110` |

**当前分支的 Tauri 命令表面**（`src-tauri/src/commands.rs`，`lib.rs:12-22` 注册 9 个）：
`login`(54)、`list_courses`(69)、`list_homework`(75)、`list_exams`(90)、`list_chapters`(105)、`list_task_points`(121)、`start_brush`(134)、`stop_brush`(189)、`brush_running`(204)。

> 说明：`main` 是 Web（HTTP/JSON），当前分支是桌面（Tauri IPC）。"请求层"的形态天然不同，因此差距清单按**功能与交互**对齐，而不是逐条对齐 HTTP 端点。

---

## 差距清单 A：`main` 有、当前分支没有

| # | 功能 | `main` 位置 | 当前分支状态 | 落地方式（不改后端） |
| --- | --- | --- | --- | --- |
| A1 | 本地会话列表（脱敏 手机号/姓名/puid），首项自动选中 | `app.py:286-293`（`load_sessions`），渲染 `app.py:426` | ✅ 已实现 | `sessions.rs` + `list_saved_sessions`（原为后端阻塞，已从 main 移植，见下节 B1）|
| A2 | "载入会话"按钮（用已存 cookie 免密登录） | `app.py:303-310`，按钮 `app.py:428` | ✅ 已实现 | `use_saved_session` 免密载入 + 密码自动重登（见 B1）|
| A3 | "刷新本地会话"按钮 | `app.py:421` | ✅ 已实现 | 前端「刷新本地会话」按钮（见 B1）|
| A4 | 扫码登录（二维码弹窗 + 2s 轮询） | `start_qr` `app.py:327-334`、`poll_qr` `app.py:336-348`、弹窗 `app.py:500-504`、`ui.timer(2.0, poll_qr)` `app.py:506` | ✅ 已实现 | `qr.rs` + `start_qr_login`/`poll_qr_login` + `QrLoginDialog.svelte`（见 B2）|
| A5 | 账号摘要面板（姓名 / 手机号 / 学校 / puid 四行） | `set_account` `app.py:276-284`，面板 `app.py:433` | ⚠️ 部分：仅单行 `{name} · {school} · puid=`（`App.svelte:383-387`），**缺手机号** | 前端补齐（`AccountInfo` 已含 `phone`，`login.rs:23-29`） |
| A6 | 密码输入框"显示/隐藏密码"切换按钮 | `app.py:431` `password_toggle_button=True` | ❌ 无（`App.svelte:371-379` 固定 `type="password"`） | 前端补齐 |
| A7 | 课程表格（列：序号/课程名/老师/课程ID/状态） | `app.py:437-449` | ⚠️ 退化为课程按钮组（`App.svelte:405-417`），**无序号、无老师、无课程ID、无状态列** | 前端补齐（`CourseInfo` 含 `course_id/name/teacher_name/state`） |
| A8 | 课程**多选**（checkbox selection="multiple"） | `app.py:445`、`selected_command` `app.py:251-253` | ❌ 单选（`selectedCourseId` 单值，`App.svelte:25`） | 前端补齐（多选状态，驱动"启动任务"） |
| A9 | 课程表格分页，每页 12 条 | `app.py:446` `pagination=12` | ❌ 无分页（全量平铺按钮） | 前端补齐 |
| A10 | "当前勾选: X" 提示（逗号连接序号；空则"无"） | `refresh_selected_hint` `app.py:272-274`、`selected_hint` `app.py:451` | ❌ 完全没有 | 前端补齐 |
| A11 | 任务状态徽标 IDLE/RUNNING/SUCCESS/FAILED（文案 未启动/运行中/已完成/失败 + detail 后缀） | `render_task_badge` `app.py:255-270`、`task_state` `app.py:454` | ❌ 无四态徽标；仅有刷课 `进行中/已结束` 二元徽标（`App.svelte:527-531`） | 前端补齐（由 `brush-progress` 的 `kind` 映射） |
| A12 | **章节进度面板**：每章一张卡片，状态 `active`/`done`/`partial` 三色，含 label + `progress_text` + name，空态文案"章节状态会在任务开始后显示" | 渲染 `app.py:394-404`，容器 `app.py:459` | ⚠️ 有"章节一览"表格（`App.svelte:588-622`）但**无 active/done/partial 状态色、非任务驱动、空态文案不同** | 前端补齐（按刷课事件维护章节状态） |
| A13 | **任务进度面板**：7 格 status-grid —— 当前课程 / 当前章节 / 任务类型 / 当前任务 / 课程进度 / 章节进度 / 任务点进度 | `app.py:462-484`，更新 `app.py:372-384` | ❌ 完全没有（仅有单行"当前章节：X" `App.svelte:532-534`） | 前端补齐（事件已含 `chapter_index/total`、`point_index/total`） |
| A14 | **双进度条**：当前任务进度 + 冷却等待（琥珀色） | `app.py:485-492`，更新 `app.py:385-388` | ✅ 已实现 | 双进度条：`taskProgress` + `waitProgress`（见 B3）|
| A15 | **运行日志**：可折叠 `expansion`，内部只读 `textarea`（520px 高、等宽、深色底）、日志追加、**自动滚动到底部** | `app.py:494-498`、自动滚动 `app.py:405-411` | ⚠️ 有日志（`App.svelte:574-580`）但**不可折叠、非 textarea、无自动滚动** | 前端补齐 |
| A16 | 2s 定时轮询模型（`ui.timer(2.0, ...)` × 2） | `app.py:506-507` | ❌ 无轮询；当前是 `brush-progress` 推送（`App.svelte:80-110`） | 保留推送模型（更优），但需补齐等价状态呈现，见 C1 |
| A17 | "启动任务"以**逗号连接的多课程命令**驱动（`command` 字符串） | `selected_command` `app.py:251-253`、`start_task` `app.py:350-361` | ✅ 已实现 | `start_brush(courses: Vec<CourseInfo>)` 顺序执行（见 B4）|
| A18 | 顶部副标题文案"完全重构为 Web 工作流，不再依赖旧 CLI/TUI 包装层" | `app.py:419` | ⚠️ 文案不同（`App.svelte:347-349`） | 文案对齐（非功能项，按需） |

---

## 差距清单 B：当前分支有、`main` 没有

> 这些是重写版**新增**的能力。按"不新增原项目没有的产品功能"的约束，重构时**保留但不扩张**，不得作为继续加功能的理由。

| # | 功能 | 当前位置 |
| --- | --- | --- |
| B1 | 作业列表 Tab（表格：作业/状态/截止/成绩，`status_code` → 彩色徽标） | `App.svelte:26, 161-180, 430-460`；命令 `commands.rs:75` |
| B2 | 考试列表 Tab（表格：考试/状态/截止） | `App.svelte:26, 182-201, 462-490`；命令 `commands.rs:90` |
| B3 | 刷课 Tab：任务点状态表（类型/标题/状态/说明） | `App.svelte:543-572` |
| B4 | 视频倍速输入（0.5–2，步长 0.5）、"仅未完成章节"勾选 | `App.svelte:505-523`；`BrushOptions` `brush.rs:14-21` |
| B5 | 停止刷课 + `brush_running` 启动时状态恢复 | `App.svelte:276-284, 73-79`；`commands.rs:189, 204` |
| B6 | 任务点预览（按章节查 `list_task_points`） | `App.svelte:227-245, 623-647`；`commands.rs:121` |
| B7 | 顶部标题"学习通工具箱"、状态卡片、`brushMeta` 播放进度显示 | `App.svelte:344-350, 655-671, 535-539` |
| B8 | Tab 切换（作业/考试/刷课） | `App.svelte:423-428` |

---

## 差距清单 C：两边都有、但行为不一致

| # | 项目 | `main` 行为 | 当前分支行为 | 需对齐为 |
| --- | --- | --- | --- | --- |
| C1 | 进度获取模型 | 2s 轮询 `GET /api/tasks/{task_id}`，服务端维护 `WebTask`（`service.py:94-141`），有 `state/current_*/chapter_items/task_progress/wait_progress` | `brush-progress` 事件推送（`App.svelte:80-110`），前端自行拼状态 | 保留推送模型（Tauri 无 HTTP 任务服务），但**补齐与 main 等价的呈现字段**（A11–A15） |
| C2 | 课程"状态"列 | 后端返回**枚举名字符串**（`进行中`/`已结课`，`cxapi/schema.py:28-31`，`service.py:508` `item.state.name`） | `CourseInfo.state` 是 **i32**（`courses.rs:18`，注释 `0=进行中, 1=已结课`） | 前端映射 `0→进行中`、`1→已结课`，未知值回退原文 |
| C3 | 课程选择粒度 | 多选，可同时跑多门课（逗号命令） | ✅ 已对齐：勾选多门课按序号顺序执行 | 见 B4/A17（已实现）|
| C4 | 登录入口 | 三选一：本地会话 / 扫码 / 密码 | ✅ 已对齐：三种入口均可用 | 见 B1/B2（已实现）|
| C5 | 密码框 | 有显示/隐藏切换 | 无 | 补齐（A6） |
| C6 | 登录后行为 | 登录成功 → `set_account` + `refresh_classes` + `ui.notify("登录成功")`（`app.py:314-325`） | 登录成功 → `setStatus` + `loadCourses`（`App.svelte:128-130`） | 行为等价（保留），提示方式统一 |
| C7 | 错误提示 | `ui.notify(..., type="negative", multi_line=True)` 顶部浮层（`app.py:249-250`） | 页面底部"状态"卡片文字（`App.svelte:655-671`） | 统一为**就近 + 显式**的错误呈现；保留全局状态区 |
| C8 | 日志容器 | 只读 `textarea`，520px 固定高，等宽深色，自动滚底 | `ScrollArea` + `<pre>`（`App.svelte:577-579`），280px，无自动滚动 | 对齐（A15） |
| C9 | 章节空态文案 | "章节状态会在任务开始后显示" | 长段说明文字（`App.svelte:582-585`） | 对齐文案 |
| C10 | 异步三态（加载/空/失败+重试） | 无（NiceGUI 直接 notify） | 无（仅文案提示，`App.svelte:402-403` 等） | **按验收标准新增**：每个异步区都要有 loading/empty/error+重试 |
| C11 | 表单校验 | 无（直接提交） | 仅非空检查（`App.svelte:121-124`） | **按验收标准新增**：必填 + 格式校验 + 错误提示 + 提交中防重复 |
| C12 | 响应式 | NiceGUI `wrap xl:no-wrap`，`max-width:1320px` | `max-w-[1100px] p-4 md:p-6`，flex-wrap | 统一为一致的断点与栅格 |

---

## 后端缺口 → 已从 main 移植（状态更新）

> 本节原为"后端阻塞项"，结论**已被取代**：按用户决定「从 main 分支获取相关内容」，
> 下列能力已在本分支后端实现（仅移植，未改接口契约/数据格式/鉴权方式），前端不再有
> "禁用 + 说明原因"的占位控件。实现位置见下。

- **B1 本地会话持久化/载入** → ✅ 已实现。新增 `src-tauri/src/api/sessions.rs`（`SavedSession`、
  `SessionFile`、`mask_phone`/`mask_name`、`list_saved_sessions`/`read_session`/`save_session`、
  `ck_dump`/`ck_load`），命令 `list_saved_sessions` / `use_saved_session`（`commands.rs`）。
  文件布局与 main 一致：`~/.cxkitty-tauri/session/<phone>.json`，字段 `phone/puid/passwd/name/ck`。
  `use_saved_session` 保留 main 的语义：cookie 失效时用已存密码自动重登，无密码则报
  `会话已失效，且本地没有保存密码`。
  → 覆盖 A1/A2/A3。
- **B2 扫码登录** → ✅ 已实现。新增 `src-tauri/src/api/qr.rs`（`qr_get`/`login_qr`），命令
  `start_qr_login` / `poll_qr_login`。**关键实现细节**：`/createqr` 直接返回二维码图片
  （`Content-Type: image/jpeg`），因此后端只需 `scraper` + `base64` 转发为 data URI，
  **不需要新增二维码生成依赖**（main 用 Python `qrcode` 重新编码，此处不需要）。
  `/login` 必须用 **web UA**（移动端 UA 会鉴权失败，与原实现注释一致）。轮询间隔 2s，与
  main 的 `ui.timer(2.0, poll_qr)` 一致。
  → 覆盖 A4。
- **B3 冷却等待进度** → ✅ 已实现。`BrushProgressEvent` 新增 `wait_progress` /
  `wait_progress_text`，`kind` 新增 `wait`；`BrushRunner::wait_cooldown` 按 1s 倒计时推送，
  文案 `等待冷却 {elapsed}/{seconds}s`，与 main 的 `WebTaskRunner.wait` 一致。冷却时长
  `BrushOptions.wait_seconds` 默认 **15**，对应 main `config.yml` 的 `video.wait`/`document.wait`/`work.wait`。
  → 覆盖 A14 的冷却等待条。
- **B4 多课程命令任务** → ✅ 已实现。`start_brush` 签名由 `course: CourseInfo` 改为
  `courses: Vec<CourseInfo>`，`run()` 顺序遍历并逐门发送 `course` 事件（含 `course_index`/`course_total`），
  与 main 的逗号连接序号命令（`ClassSelector`）语义一致。前端勾选多门课即按序号顺序执行。
  → 覆盖 A8/A17。
- **B5 当前课程字段** → ✅ 已补。`BrushProgressEvent` 新增 `current_course`，A13 的
  "当前课程"由后端事件驱动，不再依赖前端上下文。

**未移植（main 有、本分支不做，属定位"明确不做"）**：验证码 OCR（`ddddocr`/`cv2`）与人脸识别
（`face_detection.py`）。README 定位明确排除人脸；验证码/风控处理为 Python 侧能力，
无 Rust 等价实现且会引入新依赖，超出"不新增依赖"约束。若实际登录触发风控，会以真实错误信息
呈现（不伪造成功）。

---

## 重构方案（第二步执行依据）

**范围**：前端整体重写 —— 目录结构、组件拆分、样式方案、状态管理、请求层。不是修补。
**硬约束**：功能对等（覆盖 A 类全部可实现项 + 保留 B 类全部）；不新增产品功能；不新增框架/依赖；入口 `/` 与 `index.html` 不变；后端零改动。

### 目标目录结构

```
src/
  main.js                    # 挂载入口（保持）
  app.css                    # Tailwind + 设计令牌 + 语义类（重写，去掉临时类）
  App.svelte                 # 仅做布局编排 + Provider
  lib/
    components/ui/*          # 复用既有 shadcn 原语（不改）
    app/                     # 新增：业务组件
      LoginPanel.svelte         # A5/A6 + 校验 + 防重复
      # A1-A3 的本地会话 UI 直接做在 AccountPanel.svelte 内（select + 载入/刷新按钮）
      QrLoginDialog.svelte      # A4 二维码弹窗（bits-ui Dialog，无新依赖）
      AccountPanel.svelte       # A5
      CourseTable.svelte        # A7-A10 表格/多选/分页(12)/勾选提示/状态列
      TaskControlBar.svelte     # 启动任务 + A11 四态徽标
      ChapterProgressPanel.svelte # A12 三色章节卡
      TaskProgressPanel.svelte  # A13/A14 七格 + 双进度条
      RunLogPanel.svelte        # A15 可折叠 + 自动滚动
      HomeworkPanel.svelte      # B1
      ExamPanel.svelte          # B2
      BrushPanel.svelte         # B3/B4/B5/B6
    states/                  # 请求层 + 状态层
      tauri.js                  # invoke 薄封装 + 错误规范化
      account.svelte.js         # 登录态
      courses.svelte.js         # 课程 + 多选 + 分页
      brush.svelte.js           # 刷课事件 → 任务/章节/进度状态机
    utils/
      format.js                 # 脱敏、状态映射（C2）、进度文本
```

### 状态管理

Svelte 5 runes 集中到 `*.svelte.js` 模块（`$state`/`$derived` 导出 getter），替代现在 672 行里 30+ 个散落 `$state`。

### 请求层

`states/tauri.js` 统一 `invoke` 包装：错误规范化（Tauri 的 `String` 错误 → `Error`）、`brush-progress` 订阅集中管理、每个异步资源暴露 `{ status: 'idle'|'loading'|'ready'|'empty'|'error', data, error, retry() }`，直接满足验收的"三态 + 可重试"。

### 样式方案

保留 Tailwind 4 + 既有令牌（`app.css` `:root`/`.dark`）。把 `badge-*`、`.brush-log` 等临时类收敛为语义化组件样式 + `tailwind-variants`（仓库已依赖），删除调试样式。

### 构建与检查脚本（补齐验收要求，零新依赖）

- `npm run typecheck`：`tsc`（已装 TS 7.0.2）`allowJs+checkJs+strict`，新增 `tsconfig.json`（**注意 TS7 已移除 `baseUrl`**，用 `paths`）。
- `npm run lint`：仓库无 eslint 且约束禁止新增依赖 → 用**已有工具**实现可执行的静态检查（`tsc --noEmit` + 结构/死代码检查），并在文档中如实说明其覆盖范围，不冒充 ESLint。

### 分步执行（每步保持可运行）

1. 搭骨架：`states/` + `components/app/` 空壳，`App.svelte` 改为编排 —— `npm run build` 通过。
2. 登录 + 账号面板（A5/A6/C11）—— 构建通过。
3. 课程表（A7–A10/C2）—— 构建通过。
4. 任务控制 + 徽标（A11）+ 章节面板（A12）—— 构建通过。
5. 任务进度面板（A13/A14）+ 运行日志（A15）—— 构建通过。
6. 保留项重构（B1–B8）+ 三态（C10）—— 构建通过。
7. 响应式（C12）+ 可访问性 + 清理死代码 —— `build`/`typecheck`/`lint` 全绿。
8. 浏览器实测：`npm run dev` 逐页/逐态渲染、控制台无 error。
