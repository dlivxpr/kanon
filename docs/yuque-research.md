# 语雀产品与设计逻辑调研

> 调研对象：`https://www.yuque.com/`、用户已登录的 Edge Web 界面，以及语雀官方帮助、开发者文档、条款、公开 CSS 与帮助截图。
> 调研日期：2026-11-16。
> 方法：后台子任务查阅一手官方资料（§0–10）；主任务通过 Kimi WebBridge 实时访问用户已登录的 Edge，观察工作台、知识库、文档阅读/编辑态、小记与搜索，并读取可见元素的 computed style（§11）。**没有修改、保存或删除用户账号里的文档与小记。**
> 交付：[可交互单文件 HTML](yuque.html)。§7 是官方资料阶段的原型建议，不是已实现功能清单；当前界面与旧截图冲突时，以 §11 的实机观察为准，实际实现和验证范围见 §13。
> 数据边界：保留必要的导航/目录结构特征，正文使用演示内容；不收录私人小记正文、密钥或账号凭据。

## 证据等级

| 标记 | 含义 | 典型来源 |
|---|---|---|
| **A** | 官方帮助中心 / 官方开发者文档 / 官方条款原文 | `yuque.com/yuque/...`、`yuque.com/help`、`yuque.com/terms` |
| **B** | 官方页面标题与 meta、官方更新日志、官方下载信息 | `yuque.com/`、`yuque.com/about/price`、`yuque.com/yuque/yuque-desktop` |
| **C** | 官方帮助文档内截图（可证明当时界面，不保证等同 2026 年当前版本） | `cdn.nlark.com/yuque/...png` |
| **E** | 本次已登录 Edge 的直接观察、可访问性树与 computed style（仅代表此账号、此页面、此视口） | §11 的实机 URL；2026-11-16 |
| **D** | 本文推断 / 综合判断，非官方原文 | 明确标注 |

---

## 0. 结论摘要

1. **定位**：语雀是“专业的云端知识库”，官方站点标题是“语雀，为每一个人提供优秀的文档和知识库工具”，meta 描述强调“优雅高效的在线文档编辑与协同工具”“知识创作与管理”。来源：<https://www.yuque.com/>、<https://www.yuque.com/yuque/ng1qth/about>
2. **三元结构**：官方把产品概括为“知识 → 知识库 → 团队/空间”。知识是最小颗粒（文档、表格、脑图、讨论等），知识库按主题组织知识，团队/空间面向组织提供体系化知识管理。来源：<https://www.yuque.com/yuque/ng1qth/about>
3. **空间结构**：语雀空间是“空间 → 团队 → 知识库”三层；团队既可以是实际部门，也可以是虚拟项目组，团队内还有团队任务、话题库、资源库。来源：<https://www.yuque.com/yuque/organizations/rfwopm>、<https://www.yuque.com/yuque/organizations/mfxpek>
4. **文档与知识库的关系**：所有文档一定属于某个知识库；知识库像一本书，通过目录把多篇文档结构化；跨知识库引用通过“添加链接”实现，而不是让同一篇文档同时属于多个知识库。来源：<https://www.yuque.com/yuque/ng1qth/gxy3gk>、<https://www.yuque.com/yuque/thyzgp/vlki63>、<https://www.yuque.com/yuque/thyzgp/hof3ll>
5. **编辑器**：官方称“可视化文档编辑器”，核心是**块级编辑 + 卡片 + 斜杠命令 + Markdown 快捷语法**；支持多人同时编辑、@ 提及、语雀内容嵌入、知识网络、历史版本、发布/更新两态。来源：<https://www.yuque.com/yuque/gpvawt/gh4227>、<https://www.yuque.com/yuque/gpvawt/block>、<https://www.yuque.com/yuque/gpvawt/lnobo9>、<https://www.yuque.com/yuque/ng1qth/iaxgag>
6. **权限**：个人版权限是“知识库 → 文档”两层；知识库协作者有可管理/可编辑/可阅读三种身份；文档可以单独加协作者，文档公开性可以突破上级知识库。来源：<https://www.yuque.com/yuque/ng1qth/gxy3gk>
7. **视觉**：品牌绿（公开 CSS 中 `--yq-theme` 同时出现 `#2ed790` 与引用 `--yq-yuque-green-600:#00b96b`）、正文深灰 `#262626`、浅灰面 `#fafafa`、细边框 `#e7e9e8`；字体栈以 `Chinese Quote / -apple-system / PingFang SC / Microsoft YaHei` 为主。界面是“左侧导航/目录 + 中间内容 + 右侧大纲或评论”的克制型文档工具，不是卡片仪表盘。来源：<https://www.yuque.com/> 及其公开 CSS <https://gw.alipayobjects.com/os/chair-script/skylark/pc.9283cd96.css>、<https://gw.alipayobjects.com/os/chair-script/skylark/larkui.feecef61.css>；截图证据见 §6。
8. **对原型的判断（D）**：语雀没有独立的“项目”实体。个人场景的“待办”主要由**小记 + 文档任务列表**承担；团队/空间场景的“项目”由**空间/团队 + 知识库 + 团队任务 + 话题库**组合承担。原型若做“笔记 + 项目管理 + 待办”，应复刻这套组合，而不是另造一个 Yuque 没有的项目对象。

---

## 1. 来源清单与新鲜度

| 来源 | URL | 类型 | 本次用途 | 更新/新鲜度 |
|---|---|---|---|---|
| 官网首页 | <https://www.yuque.com/> | B | 站点标题、meta、登录方式、客户端下载版本 | 页面 appData 显示桌面端 `4.2.1.1333`、App 安装包 `1.2.2` |
| 官网定价页 | <https://www.yuque.com/about/price> | B | 定价入口；页面为 SPA，正文未在无渲染环境读取 | 仅确认标题“语雀定价” |
| 官网下载页 | <https://www.yuque.com/download> | B | 客户端下载入口 | 仅确认标题“下载” |
| 帮助中心 | <https://www.yuque.com/help> | A | 10 个分类、各分类书目入口 | 页面数据 |
| 语雀是什么 | <https://www.yuque.com/yuque/ng1qth/about> | A | 定位、缘起、三元结构、使用场景 | 2023-05-12 |
| 为什么选择语雀 | <https://www.yuque.com/yuque/ng1qth/why> | A | 编辑器、阅读页、评论点赞、目录、动态内容、安全、API | 2024-10-15 |
| 工作台页面介绍 | <https://www.yuque.com/yuque/ng1qth/dashboard> | A | 工作台 IA、⌘J 搜索、小记、知识库、消息、逛逛、回收站 | 2023-03-17 |
| 一文看懂语雀权限（个人版） | <https://www.yuque.com/yuque/ng1qth/gxy3gk> | A | 知识库→文档两层权限、三种协作者身份、公开性 | 2023-04-21 |
| 语雀会员 | <https://www.yuque.com/yuque/ng1qth/member-privilege> | A | 个人会员档位、价格、配额、公开分享限制 | 2026-06-25 |
| 1 分钟玩转语雀文档 | <https://www.yuque.com/yuque/ng1qth/iaxgag> | A | 多人同时编辑、文档能力、表格/数据表/画板、知识库、多端 | 2024-04-18 |
| 知识库是什么 | <https://www.yuque.com/yuque/thyzgp/repository> | A | 知识库定义、四大编辑器、目录、分享、统计、收藏 | 2023-04-25 |
| 如何新建知识库 | <https://www.yuque.com/yuque/thyzgp/gbdfpb> | A | 新建入口与归属 | 2023-03-20 |
| 如何编排目录 | <https://www.yuque.com/yuque/thyzgp/vlki63> | A | 目录树、拖拽、分组、批量操作、目录历史 | 2023-09-22 |
| 知识库分享 | <https://www.yuque.com/yuque/thyzgp/collaborators> | A | 指定人/公开分享、密码、公开搜索 | 2023-03-20 |
| 知识库权限 | <https://www.yuque.com/yuque/thyzgp/qnxyk7> | A | 知识库成员权限入口 | 未读取正文 |
| 知识库协作：添加知识库成员 | <https://www.yuque.com/yuque/thyzgp/nbkoqg> | A | 添加成员 | 未读取正文 |
| 如何发布文档 | <https://www.yuque.com/yuque/thyzgp/oe6ium> | A | 未发布/已发布两态、发布/更新、自动发布 | 2023-03-21 |
| 如何管理文档 | <https://www.yuque.com/yuque/thyzgp/my8uax> | A | 目录管理、批量复制/转移/删除 | 2023-04-26 |
| 如何分享文档 | <https://www.yuque.com/yuque/thyzgp/bosazp> | A | 文档分享、指定人/公开、密码、公开搜索、会员限制 | 2024-05-22 |
| 如何查看历史版本 | <https://www.yuque.com/yuque/thyzgp/fs0x2z> | A | 自动历史、手动版本、对比、恢复、版本链接 | 2023-08-29 |
| 如何关闭文档评论功能 | <https://www.yuque.com/yuque/thyzgp/doi9nz> | A | 知识库级关闭评论 | 2023-05-18 |
| 如何使用模板 | <https://www.yuque.com/yuque/thyzgp/yee00736hhtea3er> | A | 推荐模板、自定义模板三类 | 2023-03-21 |
| 如何使用标签 | <https://www.yuque.com/yuque/thyzgp/icl7ey> | A | 标签仅网页端、团队内生效、聚合页、交集筛选 | 2025-07-10 |
| 如何导入导出知识库 | <https://www.yuque.com/yuque/thyzgp/import-lake-to-lark> | A | 导入格式、导出 lakebook/PDF、200 篇限制 | 2023-09-12 |
| 如何导入导出内容 | <https://www.yuque.com/yuque/ng1qth/xnlf9g> | A | 单篇导出格式 | 2023-09-28 |
| 高级搜索语法 | <https://www.yuque.com/yuque/ng1qth/search-syntax> | A | 引号、in:title、NOT/AND/OR、updated、url、is:related/is:public | 2023-03-17 |
| 知识库回收站 | <https://www.yuque.com/yuque/thyzgp/recycle-bin> | A | 回收站 | 未读取正文 |
| 在线文档编辑器 | <https://www.yuque.com/yuque/gpvawt/gh4227> | A | 编辑器结构、菜单栏、工具条、卡片、大纲、正文区 | 2023-05-12 |
| 文档块级编辑 | <https://www.yuque.com/yuque/gpvawt/block> | A | 块定义、拖拽、缩进、嵌套、块转化 | 2023-03-21 |
| 插入特色卡片 | <https://www.yuque.com/yuque/gpvawt/fq1pad> | A | 卡片类型、斜杠命令、知识网络、附件、投票、打卡、日历 | 2023-05-12 |
| 支持哪些快捷键 | <https://www.yuque.com/yuque/gpvawt/shortcut> | A | 通用/编辑栏/表格/画板/桌面端快捷键、Markdown 语法 | 2024-09-13 |
| Markdown 语法 | <https://www.yuque.com/yuque/gpvawt/lnobo9> | A | Markdown 语法 | 未读取正文（快捷键文档已覆盖） |
| 如何嵌入语雀内容 | <https://www.yuque.com/yuque/gpvawt/aint7x> | A | 文档/表格/脑图/段落嵌入、10 个上限、权限一致 | 2023-05-12 |
| 如何构建知识网络 | <https://www.yuque.com/yuque/gpvawt/gh3mdb> | A | 粘贴链接、@ 提及、引入语雀内容、引用/被引用 | 2023-04-19 |
| 图片和附件大小限制 | <https://www.yuque.com/yuque/gpvawt/rms0so> | A | 图片/附件限制 | 未读取正文（会员文档给出上限） |
| 如何嵌入本地文件 | <https://www.yuque.com/yuque/gpvawt/uilmir> | A | PDF/Office/Keynote/XMind 等嵌入 | 未读取正文（卡片文档已覆盖） |
| 如何嵌入第三方服务 | <https://www.yuque.com/yuque/gpvawt/fuu6h3> | A | Figma/ProcessOn/Canva/CodePen 等 | 未读取正文（卡片文档已覆盖） |
| 如何进行文档评审 | <https://www.yuque.com/yuque/gpvawt/hig7ae> | A | 评审角色、通过/不通过/取消、历史评审 | 2023-05-12 |
| 雀雀 AI 正式上线 | <https://www.yuque.com/yuque/gpvawt/rgg508zoai0i1xfz> | A | AI 写作、润色、画板生成、阅读总结 | 2024-09-13 |
| 空间是什么 | <https://www.yuque.com/yuque/organizations/rfwopm> | A | 空间→团队→知识库三层、公共区、外部联系人、统计 | 2023-05-08 |
| 了解团队 | <https://www.yuque.com/yuque/organizations/mfxpek> | A | 团队=部门/项目组、团队动态、任务、话题 | 2023-05-05 |
| 成员权限 | <https://www.yuque.com/yuque/organizations/ac3t9l> | A | 空间/团队角色与详细权限 | 2023-03-17 |
| 团队任务 | <https://www.yuque.com/yuque/organizations/cmcg4n> | A | 任务四态、指派、标签、优先级、动态、订阅 | 2023-04-24 |
| 话题 | <https://www.yuque.com/yuque/organizations/pe0o80> | A | 话题库、日报/周报模板、定时发起/归档 | 2023-05-06 |
| 如何购买语雀空间 | <https://www.yuque.com/yuque/organizations/wvcdiy> | A | 空间支付、试用、扩容、续费、到期限制 | 2025-11-18 |
| 语雀小记使用指南 | <https://www.yuque.com/yuque/ekrzd1/dqdt6f> | A | 小记入口、便签/列表视图、标签、归档、分享、转文档 | 2023-12-08 |
| 开放 API 概览 | <https://www.yuque.com/yuque/developer/api> | A | Token 认证、个人/团队身份、权限范围 | 2024-01-18 |
| 语雀开放 API 接口文档 | <https://www.yuque.com/yuque/developer/openapi> | A | 域名、频控、重载接口、OAS/HTML 附件 | 2026-07-22 |
| 开放 API 访问范围变更 | <https://www.yuque.com/yuque/developer/vzippmige58g7r9t> | A | 2022-04-10 起个人/团队 Token 范围 | 2026-04-08 |
| 语雀服务协议 | <https://www.yuque.com/terms> | A | 服务提供者、账号、内容权属、数据存储、收费 | 2024-11-29 |
| 语雀数据安全 | <https://www.yuque.com/yuque/ng1qth/safety> | A | 金融级基础设施、多地备份、双重加密、安全认证 | 2023-03-17 |
| 桌面端 4.2.x | <https://www.yuque.com/yuque/yuque-desktop/nttax88unfg9f8qw> | B | 当前桌面端版本线索 | 2025-12-24 |
| 移动端 1.1.7 | <https://www.yuque.com/yuque/mobile-app/gcv0lme7qwlhvx29> | B | 当前移动端修复记录 | 2025-01-13 |
| 桌面端快捷键 | <https://www.yuque.com/yuque/yuque-desktop/keyboard-shortcut> | A | 桌面端快捷键（未逐条读取） | — |
| 帮助文档截图（工作台） | <https://cdn.nlark.com/yuque/0/2022/png/85538/1667800326819-4329e987-21f3-476b-a453-0d6ab1cb1cfe.png> | C | 工作台布局 | 2022 年截图 |
| 帮助文档截图（知识库首页） | <https://cdn.nlark.com/yuque/0/2022/png/519985/1666856932541-c67d6bf5-5d49-4a87-9f11-4682fe47a669.png> | C | 知识库首页与目录 | 2022 年截图 |
| 帮助文档截图（编辑器） | <https://cdn.nlark.com/yuque/0/2023/png/34713865/1683877694529-4a41cc64-9bc0-4db2-a6c3-8deb60dc6696.png> | C | 文档编辑器 | 2023 年截图 |
| 帮助文档截图（插入卡片） | <https://cdn.nlark.com/yuque/0/2023/png/21674429/1682402707177-6e5f1d4a-b0c9-448d-95bf-caf7225d4958.png> | C | 斜杠/加号卡片面板 | 2023 年截图 |
| 帮助文档截图（公开知识库） | <https://cdn.nlark.com/yuque/0/2022/png/303152/1666063589803-5ec0747d-993d-4d0c-9490-167ab102cc3f.png> | C | 公开知识库头部、作者、目录列表 | 2022 年截图 |

**未采用**：第三方评测、博客、社交媒体、非官方转载；第三方文章只用于发现官方入口，未作为结论依据。

---

## 2. 产品定位与目标用户

### 2.1 官方定义

- 站点标题：`语雀，为每一个人提供优秀的文档和知识库工具 · 语雀`。来源：<https://www.yuque.com/>
- 站点 meta：`优雅高效的在线文档编辑与协同工具，让每个企业轻松拥有文档中心……写文档，就用语雀！` 另一条 meta：`十万阿里人都在用的笔记与文档知识库，面向企业、组织或个人，提供全新的体系化知识管理……`。来源：<https://www.yuque.com/>
- 官方自我介绍：`「语雀」是一个「专业的云端知识库」，孵化自蚂蚁集团，是体验科技理念下的一款创新产品，已是 10 万阿里员工进行文档编写、知识沉淀的标配。` 来源：<https://www.yuque.com/yuque/ng1qth/about>
- 官方愿景：`语雀，成为人们创作和交流知识的场所`；`让再小的个体也可以拥有自己的知识库`。来源：<https://www.yuque.com/yuque/ng1qth/about>
- 官方知识载体：`文`（文字、图片、表格、公式、代码、视频）、`图`（画板/图形）、`表`（表格/数据表）。来源：<https://www.yuque.com/yuque/ng1qth/about>
- 官方产品结构：`知识：最小颗粒度的知识，可以是一个文档、一个表格、一个脑图、一个讨论...`；`知识库：知识的集合，按照某个主题对知识进行组织`；`团队/空间：面向企业或组织，提供全新的体系化知识管理`。来源：<https://www.yuque.com/yuque/ng1qth/about>

### 2.2 价值主张（官方文案归纳）

| 支柱 | 官方证据 | 来源 |
|---|---|---|
| 像书一样的知识库 | `知识库有点像电脑上的文件夹……当一个知识库有多篇文档时，这个知识库会形成一个目录，这也是语雀的特色` | <https://www.yuque.com/yuque/ng1qth/iaxgag> |
| 专业编辑器 | `支持 Markdown、代码块、LaTeX 公式、PlantUML 等专业编辑能力`；`可视化文档编辑器` | <https://www.yuque.com/yuque/thyzgp/repository>、<https://www.yuque.com/yuque/ng1qth/why> |
| 多人协同 | `语雀文档支持多人同时编辑一篇文档` | <https://www.yuque.com/yuque/ng1qth/iaxgag> |
| 知识网络 | `让文档可以跨越组织地关联到一起，形成一张知识网络` | <https://www.yuque.com/yuque/gpvawt/gh3mdb> |
| 安全可靠 | `金融级别网络基础设施`、`多重多地备份`、`双重密钥加密`、`多项安全认证` | <https://www.yuque.com/yuque/ng1qth/safety> |
| 多端同步 | `移动端 app、ipad 端、web 端、桌面端，并且支持多端同步` | <https://www.yuque.com/yuque/ng1qth/iaxgag> |
| 开放能力 | `Open API`、`语雀编辑器组件` | <https://www.yuque.com/yuque/developer/api>、<https://www.yuque.com/yuque/developer> |

### 2.3 官方使用场景

| 用户类型 | 官方场景 | 入口/对象 |
|---|---|---|
| 个人用户 | 个人博客、私密仓库、收集整理资料、构建自己的知识库 | 新建知识库 |
| 小型组织 | 团队博客、集体写书/翻译、分工编写文档、帮助手册、产品说明、API 手册、团队内异步交流 | 新建团队 |
| 企业用户 | 项目文档、团队事务管理、知识沉淀、规章制度、流程、产品说明书、人事流程 | 开通空间 |

来源：<https://www.yuque.com/yuque/ng1qth/about>

### 2.4 目标用户判断（D）

官方没有给出用户画像、访谈或使用数据。结合官方场景与界面密度，可以推断主力用户是：

- 需要长期沉淀文档、又希望文档有“书一样目录”的知识工作者；
- 产品、研发、设计、运营等需要多人协作编写与评审的团队；
- 需要把“项目文档 + 任务 + 讨论 + 资料”放在同一空间里的中小组织；
- 对数据安全、权限、审计有要求，但仍希望低运维成本的企业。

以上为**推断**，不是官方结论。依据：<https://www.yuque.com/yuque/ng1qth/about>、<https://www.yuque.com/yuque/organizations/rfwopm>。

### 2.5 官方边界（不做什么）

> 以下为基于官方资料的边界归纳（D），不是官方逐字声明。

- 不是纯个人笔记工具：官方把“团队/空间”作为三元结构的一层，企业协同是核心场景。来源：<https://www.yuque.com/yuque/ng1qth/about>
- 不是网盘：资源库/附件是文档的补充，核心仍是知识库与文档。来源：<https://www.yuque.com/yuque/thyzgp/repository>
- 不是无权限的公开博客：公开分享、公开搜索、密码访问、水印等都可配置。来源：<https://www.yuque.com/yuque/thyzgp/collaborators>、<https://www.yuque.com/yuque/thyzgp/bosazp>
- 不是默认开放的 AI 工具：AI 是后加能力，官方文档把 AI 写作、AI 阅读、AI 画板作为独立功能入口。来源：<https://www.yuque.com/yuque/gpvawt/rgg508zoai0i1xfz>

---

## 3. 信息架构与对象模型

### 3.1 官网 IA（可确认部分）

- 站点标题与 meta 见 §2.1。
- 官网首页、`/about/price`、`/download` 都是客户端渲染页面；静态 HTML 只含标题、meta 与脚本，**未在无渲染环境下确认顶部导航、Hero 文案与页脚链接**。
- 官方帮助中心是公开的产品信息架构样本，共 10 个分类：

| 分类 | 入口 | 代表性文档 |
|---|---|---|
| 常见疑问 | <https://www.yuque.com/yuque/qa> | 语雀是什么、权限设计、数据安全、支付问题 |
| 基础能力 | — | 工作台、权限设计、注册 |
| 知识库 | <https://www.yuque.com/yuque/thyzgp> | 知识库是什么、新建、目录、成员、分享、导入导出、回收站 |
| 编辑器 | <https://www.yuque.com/yuque/gpvawt> | 在线编辑器、块级编辑、Markdown、快捷键、卡片、嵌入、评审 |
| 团队与空间 | <https://www.yuque.com/yuque/rvzzxz> | 空间、团队、成员、权限、购买、通讯录 |
| 小记 | <https://www.yuque.com/yuque/ekrzd1> | 小记使用指南、碎片化记录、微信记小记 |
| 客户端 | <https://www.yuque.com/download> | 下载、移动端、桌面端、更新日志 |
| 权益相关 | — | 会员、稻谷、定价、流量、发票 |
| 账户相关 | — | 账户设置、绑定/解绑、注销、换手机号 |
| 其它问题 | — | 语雀的天空、开发者文档、加入语雀 |

来源：<https://www.yuque.com/help>

### 3.2 应用 IA

#### 全局左侧导航（工作台）

依据官方工作台截图（C）与帮助文档（A）：

- 顶部：语雀 logo、消息铃铛、头像；
- 搜索框：占位“搜索”，右侧显示 `⌘J`；
- 一级导航：`开始`、`小记`、`收藏`、`逛逛`；
- `个人知识库` 分组：知识库列表，可分组、可决定是否在左侧导航展示；
- 底部：`更多`（含回收站、客户端下载等）、帮助浮标。

来源：<https://www.yuque.com/yuque/ng1qth/dashboard>；截图 <https://cdn.nlark.com/yuque/0/2022/png/85538/1667800326819-4329e987-21f3-476b-a453-0d6ab1cb1cfe.png>

> 知识库首页与文档页的左侧是知识库目录/导航，不是这条全局导航；见下两节与 §7.2。

#### 工作台「开始」

- 顶部三个新建卡：`新建文档`（文档、表格、画板、数据表）、`新建知识库`、`模板中心`。
- Tab：`编辑过` / `浏览过` / `协作过`。
- 右侧筛选：`类型`、`归属`、`创建者`。
- 列表列：文档类型图标、标题、归属路径（知识库/分组）、更新时间。
- 其他模块：收藏、消息、逛逛、回收站、客户端下载。

来源：<https://www.yuque.com/yuque/ng1qth/dashboard>；截图同上。

#### 知识库

- 知识库首页：封面/背景、知识库图标与名称、文档数与字数、协作者头像、`收藏`/`分享`/编辑按钮；
- Tab：`目录`、`全部文档`、`目录管理`；
- 左侧目录树：`首页`、`搜索`、`目录`，其下是分组与文档节点；
- 目录节点支持拖拽排序、调整层级、新建分组、批量操作、目录历史；
- 知识库可设置公开性、成员权限、密码、水印、是否允许被搜索、自动发布、回收站。对应官方文档：<https://www.yuque.com/yuque/thyzgp/yztpzg>、<https://www.yuque.com/yuque/thyzgp/ay4aga>、<https://www.yuque.com/yuque/thyzgp/lk1xr8>、<https://www.yuque.com/yuque/thyzgp/recycle-bin>；首页背景/封面见 <https://www.yuque.com/yuque/thyzgp/wqfo6xvda756e1rf>。

来源：<https://www.yuque.com/yuque/thyzgp/repository>、<https://www.yuque.com/yuque/thyzgp/vlki63>、<https://www.yuque.com/yuque/thyzgp/collaborators>；截图 <https://cdn.nlark.com/yuque/0/2022/png/519985/1666856932541-c67d6bf5-5d49-4a87-9f11-4682fe47a669.png>

#### 文档页

- 顶部：返回、文档标题、路径（团队/知识库/文档）、加载状态、协作者头像、`分享`、`更新/发布`；
- 工具条：撤销/重做、段落样式、字号、粗体/斜体/下划线/删除线、文字颜色、高亮、列表、任务列表、引用、分割线、图片、附件、表格、更多；
- 正文：居中窄栏，标题层级、链接、引用、列表、卡片；
- 右侧：大纲（可隐藏/钉住）、帮助浮标；
- 底部/侧边：评论、历史版本、评审、知识网络等入口。

来源：<https://www.yuque.com/yuque/gpvawt/gh4227>；截图 <https://cdn.nlark.com/yuque/0/2023/png/34713865/1683877694529-4a41cc64-9bc0-4db2-a6c3-8deb60dc6696.png>

#### 团队/空间

- 空间：`空间 → 团队 → 知识库` 三层；
- 空间级：公共区、空间公告、通讯录、外部联系人、统计、空间管理/费用中心；
- 团队级：团队导航、团队动态、知识库（可分组）、团队任务、话题、资源（文件库/图片库）、成员与权限；
- 团队可以是实际部门，也可以是虚拟项目组。

来源：<https://www.yuque.com/yuque/organizations/rfwopm>、<https://www.yuque.com/yuque/organizations/mfxpek>

### 3.3 领域对象模型

| 对象 | 关键字段/行为 | 证据 |
|---|---|---|
| 用户 | 账号、头像、昵称、个人知识库、会员等级、Token | <https://www.yuque.com/yuque/ng1qth/dashboard>、<https://www.yuque.com/yuque/ng1qth/member-privilege> |
| 空间 | 名称、域名、成员、公共区、团队、统计、费用中心；空间角色：管理员/成员/外部联系人 | <https://www.yuque.com/yuque/organizations/rfwopm>、<https://www.yuque.com/yuque/organizations/ac3t9l> |
| 团队 | 名称、成员、知识库、团队任务、话题、资源、动态；团队角色：管理员/成员/只读成员 | <https://www.yuque.com/yuque/organizations/mfxpek>、<https://www.yuque.com/yuque/organizations/ac3t9l> |
| 知识库 | 名称、图标、封面/背景、简介、目录、成员/权限、公开性、密码、水印、统计、收藏、回收站 | <https://www.yuque.com/yuque/thyzgp/repository>、<https://www.yuque.com/yuque/thyzgp/collaborators> |
| 文档 | 标题、路径、正文块、标签、协作者、公开性、评论开关、历史版本、评审状态 | <https://www.yuque.com/yuque/thyzgp/oe6ium>、<https://www.yuque.com/yuque/thyzgp/fs0x2z>、<https://www.yuque.com/yuque/gpvawt/hig7ae> |
| 目录节点 | 文档或分组；层级、排序、默认展开层级、右侧文档信息（更新时间/创建时间/作者/路径） | <https://www.yuque.com/yuque/thyzgp/vlki63> |
| 块 | 最小可单独成行的内容；段落、卡片；可拖拽、缩进、嵌套、转化 | <https://www.yuque.com/yuque/gpvawt/block> |
| 卡片 | 图片、表格、附件、状态、画板、思维导图、流程图、数据表、代码块、公式、高亮块、折叠块、分栏、提及、语雀内容、日历、投票、打卡、加密文本、本地文件、第三方嵌入 | <https://www.yuque.com/yuque/gpvawt/fq1pad> |
| 标签 | 名称、颜色（团队任务标签可设颜色）；文档标签跨知识库、团队内生效、仅网页端、多选交集 | <https://www.yuque.com/yuque/thyzgp/icl7ey>、<https://www.yuque.com/yuque/organizations/cmcg4n> |
| 模板 | 推荐模板；自定义模板分“本知识库/团队/我的” | <https://www.yuque.com/yuque/thyzgp/yee00736hhtea3er> |
| 评论/点赞 | 文档评论、点赞；知识库可整体关闭评论；评审意见写在文档下方评论框 | <https://www.yuque.com/yuque/ng1qth/why>、<https://www.yuque.com/yuque/thyzgp/doi9nz>、<https://www.yuque.com/yuque/gpvawt/hig7ae> |
| 评审 | 发起人、评审人、抄送人；通过/不通过/取消；历史评审 | <https://www.yuque.com/yuque/gpvawt/hig7ae> |
| 历史版本 | 自动记录、手动保存版本、版本命名、对比、恢复、版本链接、删除版本 | <https://www.yuque.com/yuque/thyzgp/fs0x2z> |
| 分享 | 指定人（协作者）/公开；阅读或编辑默认权限；审批；密码；公开搜索；复制链接 | <https://www.yuque.com/yuque/thyzgp/bosazp>、<https://www.yuque.com/yuque/thyzgp/collaborators> |
| 小记 | 碎片内容、便签/列表视图、标签、归档、分享、转文档、文档内插入 | <https://www.yuque.com/yuque/ekrzd1/dqdt6f> |
| 团队任务 | 标题、描述、状态（未开始/进行中/已完成/已取消）、指派（唯一）、标签、优先级、动态、订阅、评论 | <https://www.yuque.com/yuque/organizations/cmcg4n> |
| 话题 | 话题库、日报/周报模板、定时发起、定时归档、心情、评论、附件/图片 | <https://www.yuque.com/yuque/organizations/pe0o80> |
| 资源 | 文件库、图片库；上传、预览、下载、重命名、替换、移动、删除、评论 | <https://www.yuque.com/yuque/organizations/mfxpek>、<https://www.yuque.com/yuque/organizations/ac3t9l> |
| AI 应用 | AI 帮你写、写作助手、画板生成、阅读总结/提问；可发布自定义 AI 应用 | <https://www.yuque.com/yuque/gpvawt/rgg508zoai0i1xfz> |
| 开放 API Token | 个人 Token/团队 Token；`X-Auth-Token`；频控；权限范围 | <https://www.yuque.com/yuque/developer/api>、<https://www.yuque.com/yuque/developer/openapi> |

### 3.4 关系模型

```text
账号（个人）
├── 个人知识库 1..n
│   └── 目录节点 0..n
│       └── 文档 0..n
│           ├── 块/卡片 1..n
│           ├── 标签 0..n
│           ├── 评论/点赞 0..n
│           ├── 历史版本 1..n
│           └── 评审 0..n
├── 小记 0..n（可转文档、可插入文档）
└── 空间 0..10
    └── 团队 1..n
        ├── 知识库 1..n
        │   └── 目录/文档（同上）
        ├── 团队任务 0..n
        ├── 话题库 0..n
        ├── 资源库 0..n
        └── 成员/角色 1..n
```

说明：

- “文档一定属于某个知识库”是官方明确规则；跨知识库引用是“添加链接”，不是同一文档多归属。来源：<https://www.yuque.com/yuque/ng1qth/gxy3gk>、<https://www.yuque.com/yuque/thyzgp/hof3ll>
- 个人账号也可以创建空间；官方会员文档 FAQ 说个人可创建空间上限为 10 个。来源：<https://www.yuque.com/yuque/ng1qth/member-privilege>
- 团队任务、话题、资源存在于“空间内的团队”，不是个人知识库的通用对象。来源：<https://www.yuque.com/yuque/organizations/mfxpek>

### 3.5 术语与口径

| 术语 | 官方用法 | 注意 |
|---|---|---|
| 知识 | 最小颗粒度的内容，可以是文档、表格、脑图、讨论 | 不是数据库实体名，更像产品概念 |
| 知识库 | 知识内容的载体，像一本书；有目录、成员、权限、统计 | 英文 API 里对应 `repo` / `book` |
| 文档 | 知识库中的一篇内容；有发布/未发布两态 | 官方也把文档、表格、画板、数据表统称“文稿”（会员计量） |
| 文稿 | 会员计量单位，包含小记、文档、表格、画板、数据表 | 不是一种独立编辑器类型 |
| 团队 | 空间内的组织单元，可以是部门或虚拟项目组 | 个人版也有“团队”入口，但团队任务/话题属于空间能力 |
| 空间 | 面向企业的知识协同产品，包含公共区、团队、通讯录、统计 | 不是“知识库”的同义词 |
| 小记 | 碎片化记录工具，可标签、归档、分享、转文档 | 个人私密，默认仅自己可见 |
| 话题 | 团队内的讨论型知识库 | 有日报/周报模板、定时发起/归档 |
| 卡片 | 文档中的功能块/独立编辑器/小组件 | 不是“页面卡片”UI 组件 |

来源：<https://www.yuque.com/yuque/ng1qth/about>、<https://www.yuque.com/yuque/ng1qth/member-privilege>、<https://www.yuque.com/yuque/organizations/mfxpek>、<https://www.yuque.com/yuque/ekrzd1/dqdt6f>、<https://www.yuque.com/yuque/gpvawt/gh4227>

---

## 4. 核心功能与操作流

> 本节只记录官方明确支持的内容。官方文档未写明的字段、动画、快捷键、默认值，统一放入 §9“未能确认”。

### 4.1 首次进入：工作台

官方把工作台定义为“用户登录后，访问语雀时的首页，它可以帮助你快速到达语雀上任何地方”。来源：<https://www.yuque.com/yuque/ng1qth/dashboard>

官方给出的工作台结构：

1. **开始**：最近编辑/浏览、协作过、收藏、新建文档/知识库/模板中心。
2. **小记**：碎片化记录、收藏网页、整理待办；支持标签分类与归档；可合并成文档。
3. **知识库**：知识内容的载体；可在工作台分组陈列，决定是否在左侧导航展示。
4. **消息**：他人与你互动产生的消息。
5. **逛逛**：原“广场”；`推荐` 展示公开推荐内容及你关注的人/知识库，`关注` 展示关注花园的动态。
6. **更多**：回收站、客户端下载等；回收站存放 2 周内删除的文档、画板、知识库，可恢复；团队内他人删除的文档要到知识库设置里的回收站恢复。
7. **搜索**：点击搜索入口或 `cmd + j` 唤起，可搜索语雀公开内容或与自己相关的内容。

来源：<https://www.yuque.com/yuque/ng1qth/dashboard>

**原型含义（D）**：工作台是“入口”，不是“仪表盘”。不要把它做成统计图表页；它的核心是“快速到达 + 最近内容 + 新建”。

### 4.2 新建知识库

官方流程与规则：

- 新建入口有三处：左侧导航、个人知识库页、团队页；无论从哪里发起都会进入新建页面。
- 知识库可以新建在“个人”下，也可以新建在“团队”下；在团队选择新建，知识库归属该团队。
- 官方文档只说明了入口与归属，**没有列出新建表单的全部字段**。不要臆造“模板/权限/封面”必填项。

来源：<https://www.yuque.com/yuque/thyzgp/gbdfpb>

知识库创建后的官方能力：

- 支持文档、画板、数据表、表格四大自研编辑器；
- 目录编排；
- 分享；
- 统计（文档明细、近 30 天/近一年、可导出）；
- 收藏。

来源：<https://www.yuque.com/yuque/thyzgp/repository>

### 4.3 目录组织

官方流程（A）：

1. 在知识库中新建文档；在某个文档下新建，新文档会成为该文档的子节点；在顶部 `+` 新建，文档排序到目录最上面。
2. 拖拽文档调整层级与顺序；蓝色横线表示插入位置，蓝色框表示成为子级。
3. 新建分组，把多篇文档归类；也可以直接在分组右侧 `+` 新建文档。
4. 选中文档，右侧更多操作支持重命名、复制、移动、导出、删除。
5. 批量操作：知识库左上角 `...` 进入“目录管理”，可勾选多篇文档删除、复制、移动、关闭分享；一次最多 500 篇。
6. 目录管理支持“查看历史”，所有目录变更都有记录，可选择时间恢复。
7. 目录设置：默认展开层级、右侧文档信息展示（更新时间/创建时间/作者/文档路径，默认更新时间）。
8. 目录编排实时生效，不需要“发布保存”。

来源：<https://www.yuque.com/yuque/thyzgp/vlki63>

补充：

- 文档路径可以在文档右上角“更多操作 → 文档设置”中修改；新版目录不支持直接在目录中改路径。来源：<https://www.yuque.com/yuque/thyzgp/vlki63>
- 跨知识库引用通过“添加链接”插入其他知识库/文档的网页链接；原文档移动、删除、改 URL 会导致链接失效。来源：<https://www.yuque.com/yuque/thyzgp/hof3ll>
- 移出目录的文档可以在“更多设置 → 文档管理”中查看。来源：<https://www.yuque.com/yuque/thyzgp/vlki63>

### 4.4 新建、编辑、发布文档

官方规则（A）：

- 文档有两个状态：**未发布**时，内容只在编辑状态下团队或自己可见；**已发布**后，公开知识库所有人可访问，设置权限的知识库只有自己或团队成员可查看。
- 编辑后点右上角“发布”；发布成功后进入“更新”状态，再次编辑后的操作变为“更新”。
- 发布快捷键 `⌘/Ctrl + Enter`；编辑快捷键 `E`。
- 知识库设置 → 高级设置可开启“自动发布”；开启后每次保存都会自动同步到发布态。

来源：<https://www.yuque.com/yuque/thyzgp/oe6ium>

**原型含义（D）**：编辑页顶部按钮应根据状态在“发布/更新”之间切换；不要同时显示两个主按钮。未发布状态需要有一个明确的“仅自己/团队可见”提示。

### 4.5 协作、评论、评审

#### 多人编辑

- 官方明确：`语雀文档支持多人同时编辑一篇文档`。来源：<https://www.yuque.com/yuque/ng1qth/iaxgag>
- 编辑器支持邀请别人参与编辑；文档右上角有协作者入口。来源：<https://www.yuque.com/yuque/gpvawt/gh4227>
- 知识库协作者始终拥有知识库下所有文档的权限；单篇文档可以单独加协作者。来源：<https://www.yuque.com/yuque/ng1qth/gxy3gk>

#### 评论与点赞

- 官方把“评论 & 点赞”列为产品能力：`让读者和作者之间可以就内容进行互动，帮助作者完善内容`。来源：<https://www.yuque.com/yuque/ng1qth/why>
- 知识库设置 → 更多设置可关闭评论；取消勾选后，知识库内所有文档、所有用户都无法评论。来源：<https://www.yuque.com/yuque/thyzgp/doi9nz>
- 文档评审的意见填写在文档下方的评论框。来源：<https://www.yuque.com/yuque/gpvawt/hig7ae>

#### 文档评审

官方流程（A）：

1. 有编辑权限的人可在文档右上角“更多 → 评审”发起评审。
2. 填写评审名称（必填）、评审内容说明、评审人（必填）、抄送人。
3. 评审人提交“通过/不通过”；所有评审人通过则评审通过，1 人否决则不通过；发起人可取消。
4. 文档标题上方显示评审状态；点击状态可在右侧查看评审详情；历史评审可查看上次评审内容。
5. 只有文档支持评审，表格不支持；一篇文档可以多次发起评审。

来源：<https://www.yuque.com/yuque/gpvawt/hig7ae>

### 4.6 分享与权限

#### 个人版权限模型

官方明确是“两层模型：知识库 → 文档”，上一级知识库权限影响下一级文档权限。来源：<https://www.yuque.com/yuque/ng1qth/gxy3gk>

| 层级 | 设置 | 官方规则 |
|---|---|---|
| 知识库公开性 | 仅协作者可访问 / 互联网所有人可访问 | “互联网所有人可访问”仅会员可用；设为公开后，任意游客无需登录即可看到 |
| 知识库协作者 | 可管理 / 可编辑 / 可阅读 | 协作者始终拥有知识库下所有文档的权限 |
| 文档协作者 | 单独添加 | 只拥有该文档权限，没有知识库权限 |
| 文档公开性 | 公开 / 私密 | 文档公开性可以突破上级知识库：私密知识库下可有公开文档，公开知识库下也可有私密文档 |

来源：<https://www.yuque.com/yuque/ng1qth/gxy3gk>

#### 知识库分享

- 分享面板支持“分享给指定人”和“公开分享”。
- 指定人：邀请对方成为协作者，获得链接的人需要登录语雀；默认权限“可阅读 + 无需审批”，可修改；作者会收到站内信通知。
- 公开分享：所有游客无需加入协作即可看到；个人知识库对互联网所有人，空间知识库默认空间内所有成员，也可切换为互联网所有人。
- 知识库公开后，下辖所有文档都会被设置为公开；新建文档默认公开；但公开知识库下仍可把单篇文档设为私密。
- 密码访问：知识库设密码后，下辖所有文档继承该密码，不可再独立设置；只能重置/关闭知识库密码。
- “允许语雀内公开搜索”默认关闭；开启后知识库下所有文档继承，文档不可关闭；收录延迟约 15 分钟内。

来源：<https://www.yuque.com/yuque/thyzgp/collaborators>

#### 单篇文档分享

- 阅读页/编辑页都可分享；分享面板同样分“分享给指定人/公开分享”。
- 公开分享目前只支持会员使用（2024-05-22 文档口径）。
- 阅读态默认链接权限“可阅读 + 无需审批”；编辑态默认“可编辑 + 需要审批”，均可修改。
- 文档是否公开与知识库无绑定关系：公开知识库可有私密文档，私密知识库可有公开文档。
- 密码、公开搜索规则与知识库分享一致；知识库密码会被文档继承。
- 管理已分享文档：知识库 → 更多设置 → 文档管理 → 状态筛选“分享中”。

来源：<https://www.yuque.com/yuque/thyzgp/bosazp>

**口径冲突**：2023-03-20 的《知识库分享》没有写“公开分享仅会员”；2024-05-22 的《如何分享文档》明确写“公开分享目前只支持会员使用”。原型应按后者处理，并在 UI 上给非会员一个升级提示。

### 4.7 搜索

- 工作台搜索：点击搜索入口或 `cmd + j`，可搜索语雀公开内容或与自己相关的内容。来源：<https://www.yuque.com/yuque/ng1qth/dashboard>
- 高级搜索语法（官方明确支持）：
  - 带空格的关键词组用英文引号 `"知识 大会"`；
  - `in:title` 限定标题；
  - `NOT` / `AND` / `OR`；
  - `updated:>YYYY-MM-DD`、`>=`、`<`、`<=`、`YYYY-MM-DD..YYYY-MM-DD`；
  - `url:团队主页URL`、`url:知识库主页URL`、`url:用户主页URL`、`url:团队ID`、`url:团队ID/知识库slug`；
  - `is:related`、`is:public`。
- 搜索结果页右上角会展示解析后的语法；语法错误时该条不参与查询，其余正确语法继续生效。

来源：<https://www.yuque.com/yuque/ng1qth/search-syntax>

**未确认**：搜索排序、分词策略、是否支持正文/评论/附件全文检索、搜索历史与快捷键冲突处理。

### 4.8 历史版本

官方规则（A）：

- 文档、表格、思维图、演示文稿在编辑过程中按策略自动保存历史记录。
- 编辑者可主动“保存为版本”，为版本命名。
- 支持对比任意选中的 2 个记录/版本、还原到指定记录/版本、访问指定版本。
- 版本可复制链接，分享给对该文档有访问权限的人。
- 版本可删除。

入口：阅读页或编辑页右上角“更多 → 查看历史版本”；左侧面板分“全部记录/版本”。来源：<https://www.yuque.com/yuque/thyzgp/fs0x2z>

**未确认**：自动保存的具体时间间隔、保留天数、版本数量上限。

### 4.9 模板

官方规则（A）：

- 两类模板：**语雀推荐模板**、**自定义模板**。
- 自定义模板分三类：
  - 本知识库：知识库所有协作者可用；
  - 团队：团队所有成员可用；
  - 我的：任意知识库可用。
- 两种添加方式：模板中心“新建模板”；文档右上角“另存为模板”。
- 保存为自定义模板后，每次新建文档时会出现，可快速选择。

来源：<https://www.yuque.com/yuque/thyzgp/yee00736hhtea3er>

### 4.10 导入与导出

#### 导入

- 入口：工作台和知识库的新建入口 → 搜索框右边 `+` → 导入。
- 支持格式：
  - 三方：Word（`.docx/.doc`）、Excel（`.xlsx/.xls/.csv`）、Markdown（`.md/.markdown/.mark/.txt`）、Zip、Evernote（`.html + zip`）；
  - 语雀自有：Lakebook（知识库）。
- 打包导入 Markdown 和图片时，需要把 Markdown 文件与图片打成 zip 再导入。
- 导入状态：等待中（并发上传数 2）、上传中、处理中、失败、成功。

来源：<https://www.yuque.com/yuque/thyzgp/import-lake-to-lark>、<https://www.yuque.com/yuque/ng1qth/xnlf9g>

#### 导出

- 整个知识库：知识库设置 → 更多设置；导出格式 Lakebook、PDF；支持 200 篇以内文档；过大时建议拆分。
- 单篇文档：Word、Markdown、PDF、JPG；
- 画板：JPG、PNG；
- 数据表：Excel；
- 表格：Excel。

来源：<https://www.yuque.com/yuque/thyzgp/import-lake-to-lark>、<https://www.yuque.com/yuque/ng1qth/xnlf9g>

### 4.11 小记（碎片捕获）

官方规则（A）：

- 入口：网页端/桌面端左侧导航“小记”；桌面端快捷键 `Cmd/Ctrl + Shift + Y` 唤起全局新建窗口。
- 两个视图：便签视图（碎片化记录，右侧列表一目了然）、列表视图（编辑区域更大，适合长文）；所有编辑和整理在两个视图同步。
- 创建：在编辑框输入即可；默认支持图片、任务列表、书签、链接；可全屏进入完整文档编辑器。
- 保存：自动保存；离开页面后保存并显示在右侧列表；连续新建点“记一笔”；快捷键 `Cmd/Ctrl + Enter`。
- 整理：标签分类、删除、归档、分享、批量操作。
- 与知识库打通：小记可“移动到知识库”，生成一篇普通文档，原小记归档；多篇小记可合并后转知识库；文档内可通过“素材库”插入已有小记。
- 隐私：小记是个人私密内容，其他人无法查看；分享后获得链接的人可看。

来源：<https://www.yuque.com/yuque/ekrzd1/dqdt6f>

### 4.12 团队任务、话题与资源（项目协同）

#### 团队任务

官方定义：`团队任务，是语雀为团队协同场景打造任务管理工具`，可用于知识协同中的任务管理。来源：<https://www.yuque.com/yuque/organizations/cmcg4n>

- 入口：每个团队的导航栏，开箱即用。
- 新建：右上角“新建任务”；输入标题即可，可补充描述、状态、指派、标签、优先级；支持快速新建（回车）、连续新建、展开详情页。
- 状态：未开始（默认）、进行中、已完成、已取消；状态变更记录在动态流并触发通知。
- 指派：每个任务分配给唯一负责人；可选择/搜索团队成员。
- 标签：新建标签自动分配颜色；可在标签管理改颜色/名称/排序；每个任务可多个标签；筛选支持“同时满足/满足其一/都不满足”。
- 优先级：默认空；具体档位官方截图未在正文列出（**未确认**）。
- 动态：任务生命线展示所有变更与评论；可订阅；创建、被指派、评论默认成为订阅人；可开邮件+站内信，绑定钉钉可收钉钉通知。

来源：<https://www.yuque.com/yuque/organizations/cmcg4n>

#### 话题

- 话题库是团队讨论型知识库；适用于日报/周报收集、团队活动、产品建议。
- 支持日报/周报模板、定时发起、定时归档、心情、评论、附件/图片。
- 与知识库一样支持关注、收藏、分享、邀请成员。

来源：<https://www.yuque.com/yuque/organizations/pe0o80>

#### 资源

- 团队有“资源”菜单，包含文件库、图片库；成员权限表列出上传、新建文件夹、预览、下载、重命名、替换、移动、删除、评论等操作。来源：<https://www.yuque.com/yuque/organizations/ac3t9l>、<https://www.yuque.com/yuque/organizations/mfxpek>

**对“项目 + 待办”的映射（D）**：

| 原型对象 | 语雀对应 | 说明 |
|---|---|---|
| 项目 | 团队 / 知识库 | 小项目用一个知识库；跨职能项目用团队（虚拟项目组）承载多个知识库 |
| 项目文档 | 知识库目录中的文档 | 目录即项目结构 |
| 团队待办 | 团队任务 | 有状态、指派、标签、优先级、动态 |
| 个人待办 | 小记 + 文档任务列表 | 语雀没有独立个人任务对象 |
| 讨论区 | 话题库 | 日报/周报、活动、建议 |
| 资料区 | 资源库 / 附件 | 文件库、图片库、文档附件 |

---

## 5. 编辑器设计逻辑

### 5.1 官方结构

官方把编辑器分为：

- **菜单栏**：左侧新建/跳转、返回知识库；右侧文档操作（页面尺寸、历史版本、邀请编辑等）；
- **工具条**：常规格式操作 + 插入卡片；
- **正文区域**：居中编辑区；右上角 `...` 可切换排版风格、超宽模式；
- **大纲**：最右侧中间，可隐藏为 mini 状态或钉住；
- **卡片**：正文中的特殊组成部分，可以是功能、独立编辑器或小组件；卡片有独立工具条、设置，甚至独立编辑界面。

来源：<https://www.yuque.com/yuque/gpvawt/gh4227>

### 5.2 块级模型

- 块的最小粒度是“能单独成行的内容”，如一个段落、一个卡片；布局卡片可以嵌套其他块。
- 鼠标悬浮块附近出现锚点；点击打开块操作，拖拽调整位置。
- 支持光标划选多块，批量拖拽、转化、剪切。
- 拖拽可以：排序、缩进（拖进列表）、嵌套（拖进高亮块/引用块/分栏块/折叠块）、拖到正文左右创建分栏。
- 块转化：正文、各级标题（1–4）、无序/有序/任务列表、代码块等；可加/去容器（高亮块、引用、分栏、折叠）。
- 所有块可缩进，分栏卡片暂不支持缩进；文本块可在行首按 `Tab` 缩进。

> 口径差异：块转化文档列出的标题级别是 1–4（<https://www.yuque.com/yuque/gpvawt/block>），快捷键文档列出的是 1–6（<https://www.yuque.com/yuque/gpvawt/shortcut>）。原型按 1–6 做快捷键，按 1–4 做块转化菜单即可，不必强行统一。

来源：<https://www.yuque.com/yuque/gpvawt/block>

### 5.3 插入方式

官方支持三种插入路径：

1. 工具条左侧“插入卡片”图标；
2. 段落首行输入 `/`，或行内 `Command + /`；
3. `/` 后输入卡片中文名、拼音、英文别名，直接激活对应卡片；没有匹配则当文字处理。

来源：<https://www.yuque.com/yuque/gpvawt/gh4227>、<https://www.yuque.com/yuque/gpvawt/fq1pad>

官方卡片分类：

| 分类 | 卡片 |
|---|---|
| 基础 | 图片、表格、附件、状态 |
| 画板 | 画板、思维导图、流程图 |
| 数据表 | 数据表、画册、看板 |
| 程序员专区 | 代码块、公式、UML 图、文本绘图（PlantUML 等） |
| 布局和样式 | 高亮块、折叠块、分栏卡片、引用、分割线、表情 |
| 小工具 | 提及（人和内容）、语雀内容、日历、投票、打卡、加密文本 |
| 嵌入本地文件 | 本地文件（Word、PDF、Keynote 等）、本地音频、本地视频 |
| 嵌入第三方 | 优酷、哔哩哔哩、网易云音乐、高德地图、Figma、码上掘金等 |

来源：<https://www.yuque.com/yuque/gpvawt/gh4227>、<https://www.yuque.com/yuque/gpvawt/fq1pad>

**限制（A）**：

- 一个文档中嵌入卡片最大 10 个，和嵌入 PDF/Office 文件累计。
- 暂不支持相互嵌入；嵌入一个有嵌入卡片的文档，嵌入卡片不会显示。
- 嵌入文档权限与原文档一致；私密文档可能无法被他人查看，可用分享链接解决。

来源：<https://www.yuque.com/yuque/gpvawt/aint7x>

### 5.4 Markdown 与快捷键

官方快捷键文档（2024-09-13）给出的关键规则：

| 功能 | Markdown / 快捷键（macOS） | Windows/Linux |
|---|---|---|
| 标题 1–6 | `# Space` … `###### Space` | `Ctrl+Alt+1..6` |
| 正文 | — | `Ctrl+Alt+0` |
| 粗体 | `**粗体** Space` / `__粗体__ Space` | `Ctrl+B` |
| 斜体 | `_斜体_ Space` / `*斜体* Space` | `Ctrl+I` |
| 下划线 | 官方表格写成 `*斜体* Space`（疑为笔误） | `Ctrl+U` |
| 删除线 | `~~删除~~ Space` | `Ctrl+Shift+X` |
| 行内代码 | `` `Code` Space `` | — |
| 代码块 | ` ``` Enter` | `Ctrl+E` |
| 链接 | `[标题](URL) Space` | `Ctrl+K` |
| 图片 | `![标题](URL) Space` | — |
| 分割线 | `--- Enter` / `*** Enter` / `___ Enter` | `Ctrl+Alt+S` |
| 有序列表 | `1. Space` | `Ctrl+Shift+7` |
| 无序列表 | `* Space` / `- Space` / `+ Space` | `Ctrl+Shift+8` |
| 任务列表 | `[] Space` | `Ctrl+Alt+T` |
| 引用 | `> Space` / `》 Space` | `Ctrl+Shift+U` |
| 增加/减少缩进 | `Tab` / `Shift+Tab` | `Ctrl+]` / `Ctrl+[` |
| 插入卡片 | `⌘+/ 卡片中文名/拼音/英文别名` | `Ctrl+/` |
| 提及他人 | `@` | `@` |
| 撤销/重做 | `⌘Z` / `⌘Shift+Z`、`⌘Y` | `Ctrl+Z` / `Ctrl+Shift+Z` |
| 保存草稿 | `⌘S` | `Ctrl+S` |
| 发布 | `⌘Enter` | `Ctrl+Enter` |
| 查找替换 | `⌘Shift+F` | `Ctrl+Shift+F` |
| 翻译 | `⌘Opt+E` | `Ctrl+Alt+E` |
| 素材库 | `⌘Opt+R` | `Ctrl+Alt+R` |

来源：<https://www.yuque.com/yuque/gpvawt/shortcut>

**官方文档内部错误**：快捷键表把“下划线”的 Markdown 写成 `*斜体* Space`，与斜体行重复；这明显是文档笔误，原型不要照抄，下划线只保留 `⌘U / Ctrl+U`。

### 5.5 提及、嵌入与知识网络

- 正文输入 `@` 可提及语雀内容和人；可搜索到你访问过的语雀内容范围，本知识库、本团队优先显示。来源：<https://www.yuque.com/yuque/gpvawt/gh3mdb>
- 粘贴语雀链接支持四种形态：原链接、标题、卡片、嵌入。来源：<https://www.yuque.com/yuque/gpvawt/gh3mdb>
- 可通过左上角加号面板或 `/语雀内容` 搜索插入内容。来源：<https://www.yuque.com/yuque/gpvawt/aint7x>
- 阅读页右上角“知识网络”可查看本篇文档引用过和被引用的语雀内容。来源：<https://www.yuque.com/yuque/gpvawt/gh3mdb>
- 分享链接、编辑状态链接（带 `edit`）不会进入知识网络。来源：<https://www.yuque.com/yuque/gpvawt/gh3mdb>

### 5.6 排版与阅读

- 阅读页左侧是知识库目录，右侧是文档大纲，正文区域居中；官方强调“大屏小屏专门适配”。来源：<https://www.yuque.com/yuque/ng1qth/why>
- 正文右上角 `...` 可切换“排版风格”和“超宽模式”；开启超宽后编辑区变宽，发布后阅读页同步。来源：<https://www.yuque.com/yuque/gpvawt/gh4227>
- 文档可以设置标题、路径；公开知识库可对单篇文档加密，标题显示小锁。来源：<https://www.yuque.com/yuque/gpvawt/gh4227>

### 5.7 AI 能力（官方 2024 口径）

- 新建文档时可“AI 帮你写”：左侧“开始”或知识库 `+`，选择 AI 应用，输入简介/类型生成内容，再“生成文档”。
- 文档内可通过 `/` 或 `⌘/` 调出“语雀 AI 助手-写作助手”，润色、生成表格/代码块等。
- 选中内容出现 AI 标识，可替换或插入优化结果。
- 画板内可让 AI 生成流程图/时间线。
- 阅读页右上角 AI 可速读文章、在侧边栏提问。
- 支持创建自定义 AI 应用并发布到语雀；只有“对话型/文本型”应用支持发布。

来源：<https://www.yuque.com/yuque/gpvawt/rgg508zoai0i1xfz>

**注意**：这是 2024-09-13 的产品公告，2026 年入口与命名可能变化；原型若做 AI，只做入口/空状态，不要伪造生成结果。

---

## 6. 视觉与交互设计逻辑

> 本节把“官方公开 CSS 可测值”与“官方截图测量/推断”分开。前者可复核；后者是近似值，不是官方设计规范。

### 6.1 品牌与色彩

官方公开 CSS（`pc.css` / `larkui.css`）中的关键 token：

| Token | 值 | 用途 | 证据 |
|---|---|---|---|
| `--yq-theme` | `#2ed790`；同时引用 `--yq-yuque-green-600` = `#00b96b` | 品牌绿、主按钮、链接/选中态 | 公开 CSS |
| `--yq-yuque-green-500` | `#0bd07d` | 亮绿 | 公开 CSS |
| `--yq-yuque-green-600` | `#00b96b` | 深绿、主色引用 | 公开 CSS |
| `--yq-yuque-green-100` | `#daf6ea`（浅色主题） | 浅绿底 | 公开 CSS |
| `--yq-yuque-grey-900` | `#262626` | 主文字 | 公开 CSS |
| `--yq-yuque-grey-700` | `#848484` | 次级文字 | 公开 CSS |
| `--yq-yuque-grey-500` | `#424242` | 正文/深灰 | 公开 CSS |
| `--yq-yuque-grey-400` | `#e7e9e8` | 边框 | 公开 CSS |
| `--yq-yuque-grey-300` | `#eff0f0`（浅色主题） | 浅分隔线 | 公开 CSS |
| `--yq-yuque-grey-100` | `#fafafa` | 浅背景 | 公开 CSS |
| `--yq-white` | `#fff` | 页面底 | 公开 CSS |
| `--yq-black` | `#000` | 深色主题底 | 公开 CSS |
| 蓝 | `#3b82ce` | 链接/信息 | 公开 CSS 出现 |
| 红 | `#ca3f4f` | 错误/警示 | 公开 CSS 出现 |
| 黄 | `#d2a638` | 警告/强调 | 公开 CSS 出现 |
| 绿（辅助） | `#51b88d` | 成功/辅助 | 公开 CSS 出现 |

来源：<https://www.yuque.com/> 页面公开 CSS：<https://gw.alipayobjects.com/os/chair-script/skylark/pc.9283cd96.css>、<https://gw.alipayobjects.com/os/chair-script/skylark/larkui.feecef61.css>

**注意**：公开 CSS 同时包含浅色/深色主题的覆盖值；上表只取浅色主题常用值。`#2ed790` 与 `#00b96b` 同时存在，说明“品牌绿”有亮/深两个层次，不应只取一个。

**截图取样（C/D）**：官方帮助截图中，主按钮、加号、选中态、链接接近 `#00b96b`；页面底为白色，侧栏与内容区用 `#fafafa`/`#f5f5f5` 和 `1px` 浅灰线分层；正文深灰接近 `#262626`，次级文字 `#848484`。

### 6.2 字体

公开 CSS 的字体栈：

```css
font-family: Chinese Quote, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
  "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", "Helvetica Neue",
  Helvetica, Arial, sans-serif;
```

来源：<https://gw.alipayobjects.com/os/chair-script/skylark/pc.9283cd96.css>

官方没有公开“应用内字号规范”。从截图（C）可观察到：

- 工作台页面标题约 24–28px，字重 600；
- 文档标题约 28–32px，字重 600；
- 正文约 15–16px，行高约 1.8–2.0；
- 目录/列表文字约 13–14px；
- 元信息约 12px，颜色 `#848484`；
- 编辑器工具条字号选择显示 `15px`（截图可见）。

以上为截图测量近似值（D），不是官方 token。

### 6.3 布局与栅格

依据官方帮助截图（C）测量，按 1440×900 逻辑画布折算：

| 区域 | 近似值 | 说明 |
|---|---|---|
| 左侧全局导航 | 240–250px | 工作台/知识库/文档共用；白底、右侧 1px 浅线 |
| 知识库目录栏 | 260–280px | 知识库页/文档页左侧；含返回、知识库名、首页/搜索/目录 |
| 主内容区 | 720–820px | 文档正文居中窄栏；知识库首页可到 1000px |
| 右侧大纲 | 180–220px | 文档页右侧；可隐藏/钉住 |
| 评论/详情侧栏 | 300–340px | 覆盖或挤压正文；官方未给固定宽度 |
| 页面左右留白 | 32–48px | 工作台/知识库首页 |
| 列表行高 | 52–56px | 工作台文档列表 |
| 目录行高 | 36–40px | 目录树节点 |
| 工具条高度 | 44–48px | 编辑器格式条 |
| 面板圆角 | 8–16px | 知识库首页白色面板约 16px；按钮/输入 6–8px |

以上均为**截图测量/推断（D）**，不能当作官方设计规范。

### 6.4 组件语言

#### 左侧导航

- 顶部 logo：绿色圆角方块 + 白色抽象雀鸟负形（C）；旁边可显示“语雀”字标。
- 搜索框：浅灰底、圆角 6–8px、左侧放大镜、右侧 `⌘J` 键帽；高度约 32–36px。
- 一级项：图标 18px + 文字 14px；选中项浅灰底/浅绿底，不使用高饱和填充。
- 分组标题：12px 灰字，可折叠；右侧 `+`。
- 底部“更多”：常驻，不随内容滚动。

#### 按钮

- 主按钮：品牌绿底、白字、圆角 6px、高 32px；文字如“发布/更新/分享/新建”。
- 次按钮：白底、1px 边框、深灰字；如“收藏/取消”。
- 文字按钮：无底色，绿色或深灰；如“全部文档/目录管理”。
- 危险操作：红色文字或红色描边，不默认填充。

#### 文档列表

- 行：类型图标 + 标题 + 归属路径 + 更新时间；底部 1px 浅线。
- 悬浮：浅灰底；当前行可浅绿/浅灰底。
- 知识库首页的文档列表使用“标题 + 虚线引导 + 日期”的书目式排版（C），这是语雀区别于普通表格列表的特征。

#### 编辑器

- 顶部：返回、标题、路径、协作者头像、分享、发布/更新。
- 工具条：图标按钮 28×28，选中态浅灰底；段落样式/字号为下拉；`+` 为绿色圆形。
- 正文：无卡片边框，靠字号、间距、分割线建立层次；引用/高亮块才有浅色底。
- 右侧大纲：细线刻度 + 当前章节高亮；可收起为悬浮按钮。
- 评论：文档下方评论区；评审意见也写在这里。

#### 弹窗/抽屉

- 分享弹窗：约 480px 宽，顶部标题 + 关闭；两个 Tab（分享给指定人/公开分享）；权限下拉；复制链接；公开分享开关/密码/搜索开关。
- 历史版本：左侧列表（全部记录/版本），右侧预览/对比；顶部“保存为版本/恢复”。
- 搜索：居中浮层，输入框 + 结果分组；`Esc` 关闭，`↑/↓` 选择，`Enter` 打开。
- 模板中心：网格卡片，分类“推荐/本知识库/团队/我的”。

### 6.5 交互原则

官方没有公开动效规范。依据截图与产品类型，建议（D）：

- 以“即时反馈、低动效”为主：hover 120–160ms、弹窗 160–200ms、抽屉 200–240ms，`ease-out`；
- 不使用大面积位移、缩放入场、视差；
- 保存状态用文字/小图标提示（“已保存/更新”），不要用夸张 toast；
- 危险操作二次确认；
- 目录拖拽用蓝色插入线/蓝色框表示落点（官方文档明确）；
- 链接/按钮 focus 可见，键盘可达。

### 6.6 视觉禁区

- 不要把语雀做成 Notion 式的“全白 + 大图标 + 卡片网格”；语雀的知识库首页是“书目式列表 + 轻封面”，文档页是“窄栏正文 + 目录/大纲”。
- 不要用 Woday 的黑色外壳、红色强调、圆角大面板。
- 不要给每个文档/目录项加阴影；语雀主要用 1px 线和浅灰底。
- 不要默认展开所有目录；目录应可折叠，默认层级可设置。
- 不要把“公开分享”做成默认开启；官方默认关闭公开搜索，公开分享需要主动开启。
- 不要伪造官方 logo 的高精度矢量；原型可用简化绿色方块 + 文字标识。

---

## 7. 单文件 HTML 高保真原型规格

> 本节是**综合建议（D）**，不是语雀官方设计规范。数值来自 §6 的公开 CSS 与截图测量；主 agent 应以 Edge 中实际看到的页面为准，发现冲突时优先真实页面。

### 7.1 默认屏幕与视图优先级

**建议默认屏幕：工作台「开始」（登录态），1440×900。**

理由：

- 工作台是官方定义的“登录后首页”，能一次展示语雀的 IA：搜索、开始、小记、收藏、逛逛、个人知识库、更多。来源：<https://www.yuque.com/yuque/ng1qth/dashboard>
- 它比直接打开一篇文档更能说明“知识库 + 文档 + 协作”的产品结构。
- 从工作台点一篇最近文档，一步进入文档页，适合演示。

**关键视图优先级：**

| 优先级 | 视图 | 作用 |
|---|---|---|
| P0 | 工作台「开始」 | 默认屏幕；展示 IA 与最近内容 |
| P0 | 知识库首页 + 目录 | 展示“像书一样”的组织方式 |
| P0 | 文档阅读/编辑页 | 展示编辑器、发布/更新、分享、评论、历史 |
| P1 | 分享弹窗 | 展示权限模型 |
| P1 | 搜索浮层 | 展示 ⌘J 与结果分组 |
| P1 | 历史版本抽屉 | 展示版本/恢复 |
| P2 | 小记 | 展示碎片捕获 |
| P2 | 团队空间/任务 | 展示项目协同（若时间允许） |
| P2 | 模板中心 | 展示模板分类 |

如果主 agent 在 Edge 中看到的是**未登录的官网营销首页**，不要硬套工作台；用 §7.7 的营销页规格，并以浏览器实见文案为准。本报告不虚构 Hero 文案。

### 7.2 全局外壳

#### 工作台

```css
:root {
  --yq-green: #00b96b;
  --yq-green-bright: #2ed790;
  --yq-green-soft: #e8f8f0;
  --yq-ink: #262626;
  --yq-text: #424242;
  --yq-muted: #848484;
  --yq-faint: #b3b3b3;
  --yq-line: #e7e9e8;
  --yq-line-soft: #eff0f0;
  --yq-bg: #ffffff;
  --yq-bg-soft: #fafafa;
  --yq-bg-hover: #f5f5f5;
  --yq-blue: #3b82ce;
  --yq-red: #ca3f4f;
  --yq-yellow: #d2a638;
  --r-sm: 4px;
  --r-md: 6px;
  --r-lg: 8px;
  --r-panel: 16px;
  --font-ui: "Chinese Quote", -apple-system, BlinkMacSystemFont, "Segoe UI",
    Roboto, "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei",
    "Helvetica Neue", Helvetica, Arial, sans-serif;
}

* { box-sizing: border-box; }
html, body { margin: 0; height: 100%; }
body {
  font-family: var(--font-ui);
  color: var(--yq-ink);
  background: var(--yq-bg);
  -webkit-font-smoothing: antialiased;
}

.app {
  display: grid;
  grid-template-columns: 248px minmax(0, 1fr);
  height: 100vh;
  overflow: hidden;
}

.sidebar {
  display: flex;
  flex-direction: column;
  background: var(--yq-bg);
  border-right: 1px solid var(--yq-line-soft);
  padding: 12px 10px 8px;
  overflow: hidden;
}

.sidebar__scroll { flex: 1; overflow: auto; }
.sidebar__footer { flex: none; padding-top: 8px; }

.main {
  overflow: auto;
  background: var(--yq-bg);
}
```

#### 知识库首页/文档页

```css
.doc-shell {
  display: grid;
  grid-template-columns: 264px minmax(0, 1fr);
  height: 100vh;
  overflow: hidden;
}
.catalog {
  border-right: 1px solid var(--yq-line-soft);
  background: var(--yq-bg-soft);
  overflow: auto;
  padding: 12px 10px;
}
.doc-main {
  overflow: auto;
  position: relative;
  background: var(--yq-bg);
}
.doc-body {
  width: min(760px, calc(100% - 96px));
  margin: 0 auto;
  padding: 40px 0 120px;
  font-size: 15px;
  line-height: 1.9;
  color: var(--yq-text);
}
.outline {
  position: fixed;
  right: 24px;
  top: 120px;
  width: 180px;
  font-size: 12px;
  color: var(--yq-muted);
}
```

**说明**：官方文档页截图没有常驻全局导航，而是“返回 + 知识库目录 + 正文”；工作台使用全局导航，知识库首页/文档页使用知识库目录侧栏。原型应按视图切换外壳，而不是所有页面都塞同一条侧栏。

### 7.3 屏幕 A：工作台「开始」

```text
┌───────────────────────┬──────────────────────────────────────────────────────────────┐
│ [logo] 语雀    🔔  (头像) │  开始                                                        │
│ ┌───────────────────┐ │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐             │
│ │ 🔍 搜索        ⌘J │ │  │ 新建文档     │ │ 新建知识库   │ │ 模板中心     │             │
│ └───────────────────┘ │  │ 文档/表格/...│ │ 使用知识库...│ │ 从模板中...  │             │
│ ● 开始                │  └─────────────┘ └─────────────┘ └─────────────┘             │
│ ✎ 小记                │                                                              │
│ ☆ 收藏                │  编辑过   浏览过   协作过                 类型 归属 创建者   │
│ ◎ 逛逛                │  ─────────────────────────────────────────────────────────── │
│                       │  [图标] 2026 Q4 产品规划        产品设计手册 / 规划   11-16 10:24│
│ ▾ 个人知识库          │  [图标] 需求评审流程            产品设计手册 / 流程   11-15 18:02│
│   [图标] 产品设计手册 │  [图标] 项目立项                产品设计手册 / 项目   11-14 09:40│
│   [图标] 团队周报     │  [图标] 设计系统使用指南        产品设计手册 / 基础   11-12 15:10│
│   [图标] 项目文档     │  [图标] 周会纪要 11-12          团队周报 / 2026-W46   11-12 20:30│
│   [图标] 个人笔记     │  ...                                                         │
│                       │                                                              │
│ ...                   │                                                              │
│ ⚙ 更多                │                                                              │
└───────────────────────┴──────────────────────────────────────────────────────────────┘
```

规格：

- 侧栏 248px；顶部 logo 28px；搜索框高 34px，圆角 6px；一级项高 36px，圆角 6px；选中项 `#f5f5f5`。
- 主区左右 padding 40px，顶部 32px；标题 24px/600；三个新建卡高 84px，白底、1px `#e7e9e8`、圆角 8px；卡片内图标 28px、标题 14px/600、说明 12px `#848484`。
- Tab 高 36px，14px；激活项深灰字 + 2px 深色下划线（或浅灰 pill，二者选一，不要混用）。
- 筛选文字 12px `#848484`，右侧对齐。
- 列表行高 54px；底部 1px `#f0f0f0`；标题 14px；路径/时间 12px `#848484`；hover `#fafafa`。
- 空状态：一句“还没有内容”+ 主按钮“新建文档”，不要插画堆叠。

### 7.4 屏幕 B：知识库首页

```text
┌───────────────────────┬──────────────────────────────────────────────────────────────┐
│ ‹  [图标] 产品设计手册 🔒 │  ┌────────────────────────────────────────────────────────┐  │
│                    +  │  │  [图标] 产品设计手册                    ☆ 收藏   [分享] ✎ │  │
│ 🏠 首页               │  │  32 文档   12,480 字   (头像)(头像)(头像)                 │  │
│ 🔍 搜索               │  │                                                        │  │
│ ☰ 目录           ☰☰  │  │  👋 欢迎来到产品设计手册                                │  │
│                       │  │  知识库就像书一样，让多篇文档结构化……                    │  │
│ ▾ 00 开始             │  │                                                        │  │
│   产品设计原则        │  │  ▾ 00 开始                                              │  │
│   设计系统使用指南    │  │    产品设计原则  ························  11-10 10:20  │  │
│ ▾ 01 项目             │  │    设计系统使用指南 ····················  11-08 16:40  │  │
│   项目立项            │  │  ▾ 01 项目                                              │  │
│   需求评审流程        │  │    项目立项 ····························  11-14 09:40  │  │
│ ▾ 02 复盘             │  │    需求评审流程 ························  11-15 18:02  │  │
│   2026 Q4 复盘        │  │  ...                                                    │  │
└───────────────────────┴──────────────────────────────────────────────────────────────┘
```

规格：

- 左栏 264px，`#fafafa` 底；顶部返回 + 知识库名 + 锁/公开图标 + `+`；下面 `首页/搜索/目录`；目录树缩进 16px，行高 36px。
- 主区背景可用低饱和渐变/模糊封面（官方截图是柔和彩色背景），上覆白色圆角面板；面板最大宽 1040px，圆角 16px，padding 48px。
- 头部：图标 44px，标题 36px/600；统计 13px `#848484`；协作者头像 24px 叠放；按钮高 32px。
- Tab：`目录 / 全部文档 / 目录管理`；激活项黑色下划线。
- 文档列表：行高 52px；标题 15px；日期 12px `#848484`；标题与日期之间用虚线引导（可用 `border-bottom: 1px dashed` 或伪元素）。
- 目录树当前项：浅绿底 `#e8f8f0` + 绿色左条/绿色文字；hover `#f0f0f0`。

### 7.5 屏幕 C：文档阅读/编辑页

```text
┌───────────────────────┬──────────────────────────────────────────────────────────────┐
│ ‹ [图标] 产品设计手册  │  ‹  项目立项                         (头像)(头像)  [分享] [更新] │
│                    +  │  体验科技 / 产品设计手册 / 项目立项   已加载最新版本           │
│ 🏠 首页               │  ─────────────────────────────────────────────────────────── │
│ 🔍 搜索               │  ↶ ↷  │ 正文 ▾ │ 15px ▾ │ B I U S │ A ▾ │ … │ + │ …      │
│ ☰ 目录               │  ─────────────────────────────────────────────────────────── │
│ ▾ 00 开始             │                                                              │
│ ▾ 01 项目             │  项目立项                                                    │
│   ● 项目立项          │                                                              │
│     需求评审流程      │  一、背景                                                    │
│ ▾ 02 复盘             │  当前产品设计手册缺少统一的项目立项模板，导致……              │
│                       │                                                              │
│                       │  二、目标                                                    │
│                       │  - 统一立项信息结构                                          │
│                       │  - 明确评审人与时间节点                                      │
│                       │                                                              │
│                       │  > 立项通过后，文档进入“进行中”状态。                        │
│                       │                                                              │
│                       │  [嵌入卡片：需求评审流程]                                    │
│                       │                                                              │
│                       │  ── 评论 (2) ─────────────────────────────────────────────  │
│                       │  (头像) 林知远  11-14 10:20                                  │
│                       │  第二节的目标建议补一个可量化指标。                          │
└───────────────────────┴──────────────────────────────────────────────────────────────┘
```

规格：

- 顶部文档头：高 72px；标题 28px/600；路径 12px `#848484`；右侧头像 24px、分享次按钮、更新主按钮（绿色）。
- 工具条：高 44px，sticky top 0，白底，底部 1px `#f0f0f0`；图标按钮 28×28，hover `#f5f5f5`，激活 `#eff0f0`；`+` 为 28px 绿色圆形。
- 正文：宽 760px，15px/1.9；H2 24px/600，H3 18px/600；段落间距 16px；列表缩进 24px；引用左侧 3px 绿/灰线 + 浅灰底；分割线 `#e7e9e8`。
- 目录当前项用绿色文字 + 浅绿底；父级可折叠。
- 右侧大纲：固定在正文右侧，宽 180px；只显示 H2/H3；当前章节绿色；可收成悬浮圆按钮。
- 评论：正文下方，标题“评论 (2)”；头像 24px、名字 13px/600、时间 12px `#848484`、正文 14px；输入框 40px 高，发送按钮绿色。
- 编辑态：正文 `contenteditable`，工具条可用；保存后顶部显示“已保存/更新于刚刚”；发布按钮文案按状态切换。

### 7.6 浮层与抽屉

#### 分享弹窗

```text
┌──────────────────────────────────────────────┐
│ 分享「项目立项」                         ×  │
│ ┌────────────────┬─────────────────┐         │
│ │ 分享给指定人   │ 公开分享        │         │
│ └────────────────┴─────────────────┘         │
│ 邀请协作                                     │
│ [输入姓名或邮箱____________]  [可阅读 ▾]     │
│ [复制链接]                                   │
│ ─────────────────────────────────────────── │
│ 公开分享  [开关]                             │
│ 访问密码  [未设置]                           │
│ 允许语雀内公开搜索 [开关]                    │
│ 链接：https://www.yuque.com/...   [复制]     │
└──────────────────────────────────────────────┘
```

- 宽 480px，圆角 12px，遮罩 `rgba(0,0,0,.35)`。
- Tab 激活项绿色下划线；权限下拉“可阅读/可编辑/可管理”。
- 公开分享开关关闭时，密码/搜索设置置灰。
- 复制后 toast“链接已复制”，不要跳转。

#### 搜索浮层

- 宽 640px，距顶 120px，圆角 12px，阴影轻。
- 输入框 48px，左侧放大镜，右侧 `Esc`。
- 结果分组：`文档`、`知识库`、`小记`；每项图标 + 标题 + 路径；选中项浅灰底。
- 空结果：“没有找到相关内容”，并给“搜索公开内容”提示。
- 快捷键：`⌘J` 打开，`↑/↓` 选择，`Enter` 打开，`Esc` 关闭。

#### 历史版本抽屉

- 宽 360px，从右侧滑出；顶部“历史版本”，Tab“全部记录/版本”。
- 列表项：版本名/时间/作者；当前版本标记。
- 底部“恢复此版本”主按钮；选中两个版本可“对比”。
- 恢复前二次确认：“恢复后将覆盖当前内容，是否继续？”

#### 模板中心

- 全屏/大抽屉；顶部搜索；分类“推荐/本知识库/团队/我的”。
- 卡片：预览缩略图（用灰块/线框，不用真实截图）、模板名、来源、使用按钮。
- 使用后创建“无标题文档”，标题可编辑。

#### 小记

- 桌面端可用 380px 浮窗；网页端可用全页两栏（左编辑、右列表）。
- 顶部“小记” + 视图切换（便签/列表）+ 关闭。
- 编辑框：支持任务列表、图片占位、链接；底部标签 + “记一笔”。
- 右侧列表：按时间倒序；标签筛选；悬浮显示更多（移动到知识库/归档/分享/删除）。

### 7.7 营销页规格（仅在未登录首页时使用）

由于 `https://www.yuque.com/` 为 SPA，本次未在无渲染环境确认 Hero 文案与导航，因此只给出**结构建议**，不给出虚构文案：

1. 顶部：左侧绿色 logo + “语雀”，中间产品/解决方案/定价/资源，右侧“登录/免费注册”。
2. Hero：一句话价值主张 + 副文案 + 主 CTA“免费注册” + 次 CTA“下载客户端”；右侧产品界面截图。
3. 能力区：知识库、文档编辑器、多人协作、权限安全、AI、开放 API。
4. 场景区：个人/团队/企业三列。
5. 价格区：个人会员/空间版本。
6. 页脚：关于语雀、使用帮助、数据安全、服务协议、隐私政策、English。

**文案必须以 Edge 实见为准**；可引用的官方短句只有站点标题、meta 与 §2.1 的官方定义。

### 7.8 内容 fixture（可直接使用）

#### 账号与团队

- 当前用户：`林知远`，产品设计师，头像用姓氏首字母圆形色块。
- 团队：`体验科技`。
- 协作者：`陈默`（产品）、`周予安`（设计）、`苏晚`（研发）。

#### 知识库

| 名称 | 文档数 | 字数 | 说明 |
|---|---|---|---|
| 产品设计手册 | 32 | 12,480 | 默认打开的示例知识库 |
| 团队周报 | 128 | 48,200 | 周报/月报 |
| 项目文档 | 56 | 21,600 | 项目立项/评审/复盘 |
| 个人笔记 | 18 | 6,400 | 私密 |

#### 目录

```text
产品设计手册
├── 00 开始
│   ├── 产品设计原则
│   └── 设计系统使用指南
├── 01 项目
│   ├── 项目立项          ← 当前文档
│   ├── 需求评审流程
│   └── 上线检查清单
└── 02 复盘
    └── 2026 Q4 复盘
```

#### 文档「项目立项」

- 路径：`体验科技 / 产品设计手册 / 项目立项`
- 元信息：`32 文档`、`12,480 字`、`8 位作者`（知识库级）
- 正文：

```markdown
# 项目立项

## 一、背景
当前产品设计手册缺少统一的项目立项模板，导致项目目标、范围和里程碑口径不一致。

## 二、目标
- 统一立项信息结构
- 明确评审人与时间节点
- 让项目文档可以被复用和追溯

## 三、范围
| 阶段 | 产出 | 负责人 |
|---|---|---|
| 立项 | 立项文档 | 林知远 |
| 评审 | 评审结论 | 陈默 |
| 上线 | 检查清单 | 苏晚 |

## 四、里程碑
- [x] 完成立项模板初稿
- [ ] 11-18 前完成评审
- [ ] 11-25 前上线

> 立项通过后，文档进入“进行中”状态。

## 五、相关文档
[嵌入卡片：需求评审流程]
```

- 评论：
  - `林知远 11-14 10:20`：第二节的目标建议补一个可量化指标。
  - `陈默 11-14 11:05`：收到，今天下午更新。
- 历史版本：
  - `当前版本` 11-16 10:24
  - `评审版` 11-14 18:30
  - `初稿` 11-12 09:10

#### 小记

- `把首页信息架构改成三段式，周五评审前出稿`，标签 `设计`、`待办`。
- `补一下空状态插画的需求说明`，标签 `设计`。
- `周会纪要：项目立项模板本周定稿`，标签 `复盘`。

#### 团队任务

| 标题 | 状态 | 指派 | 优先级 | 标签 |
|---|---|---|---|---|
| 补充空状态插画 | 进行中 | 周予安 | 高 | 设计 |
| 完成立项模板评审 | 未开始 | 陈默 | 中 | 流程 |
| 更新设计系统文档 | 已完成 | 林知远 | 低 | 文档 |

### 7.9 交互清单

| 操作 | 结果 |
|---|---|
| 点击侧栏「开始」 | 回到工作台 |
| 点击侧栏知识库 | 打开知识库首页 |
| 点击最近文档 | 打开文档页 |
| 点击 `+ 新建文档` | 在目录顶部插入「无标题文档」，进入编辑态，标题全选 |
| 点击目录 chevron | 展开/收起分组 |
| 点击目录文档 | 切换文档，更新正文、标题、路径、大纲、评论 |
| 悬浮文档行 | 显示更多 `…`；菜单：重命名、复制、移动、导出、删除 |
| 编辑正文 | 工具条按钮 active；800ms 后显示“已保存”；发布按钮保持“更新” |
| 点击「更新」 | 按钮短暂显示“更新中”，完成后 toast“已更新” |
| 输入 `/` | 打开插入面板；搜索/最近使用/分类；`Esc` 关闭 |
| 点击 `+`（工具条） | 同上 |
| 点击「分享」 | 打开分享弹窗；Tab 切换；复制链接 toast |
| 切换公开分享 | 显示密码/公开搜索设置；关闭时置灰 |
| 点击「历史」 | 打开版本抽屉；选择版本预览；恢复二次确认 |
| 点击「评论」 | 展开评论区；输入并发送后追加；计数 +1 |
| `⌘J` | 打开搜索浮层；输入过滤；`Enter` 打开第一条 |
| 点击「小记」 | 打开小记浮窗/页面；记一笔后进入右侧列表 |
| 点击「模板中心」 | 打开模板网格；使用模板创建文档 |
| 点击「任务」（团队空间） | 打开团队任务表；状态/优先级筛选；切换状态 |
| 点击头像 | 菜单：账户设置、语雀会员、退出登录（仅展示，不实现登录） |

### 7.10 边界与不要做

**必须做：**

- 中文界面、中文标点、真实文档密度；不要用英文占位。
- 左侧目录 + 中间正文 + 右侧大纲的阅读骨架。
- 知识库首页的“书目式列表”（标题 + 虚线 + 日期）。
- 绿色只用于主操作、选中、链接、成功状态。
- 目录树层级、当前项高亮、折叠。
- 发布/更新两态；分享/公开/密码的权限关系。
- 评论、历史版本、搜索至少各有一个可用入口。

**不要做：**

- 不要实现真实登录、真实多人协同、真实上传、真实 AI、真实支付、真实公开分享链接。
- 不要伪造语雀后端接口或声称数据已同步。
- 不要添加语雀没有确认的对象：独立“项目”实体、文档看板视图、个人任务中心；文档封面如无官方依据也不要加。
- 不要默认开启公开分享/公开搜索。
- 不要把所有页面都做成卡片网格或仪表盘。
- 不要使用 Woday 的视觉语言。
- 不要复制官方 logo 的高精度矢量/图片；用简化标识。
- 不要在未确认的情况下写“免费版可公开分享”“AI 默认开启”“API 对所有人免费”。

### 7.11 验收清单

- [ ] 1440×900 下无横向滚动；侧栏、正文、大纲不重叠。
- [ ] 工作台 → 知识库 → 文档 → 返回，路径可走通。
- [ ] 目录折叠/展开、当前项高亮正确。
- [ ] 文档编辑后“已保存/更新”状态有变化。
- [ ] 分享弹窗的 Tab、权限、公开开关、密码状态联动。
- [ ] 搜索 `⌘J` 可打开、过滤、Esc 关闭。
- [ ] 历史版本可选择/恢复（至少前端状态）。
- [ ] 评论可新增，计数变化。
- [ ] 小记可新增并出现在列表。
- [ ] 所有按钮有 hover/focus 态；键盘可达。
- [ ] 没有未标注的“官方”声称；演示数据不冒充真实用户数据。

---

## 8. 版本、价格、平台与开放能力

### 8.1 个人会员

官方《语雀会员》更新于 2026-06-25，给出三档：

| 权益 | 普通用户 | 专业会员 | 超级会员 |
|---|---|---|---|
| 价格 | 免费 | 99 元/人/年 | 299 元/人/年 |
| 文稿数 | 100 篇/月 | 无限 | 无限 |
| 知识库数 | 100 | 1500 | 无限 |
| 协作人数 | 5 | 50 | 500 |
| 月度流量 | 1 GB | 10 GB | 50 GB |
| 单图片上限 | 10 MB | 20 MB | 50 MB |
| 单附件上限 | 30 MB | 500 MB | 2 GB |
| 单视频上限 | 不支持上传 | 500 MB | 2 GB |
| 小记/文档/导入导出 | ✅ | ✅ | ✅ |
| 附件嵌入查看 | ❌ | ✅ | ✅ |
| 知识库高级功能 | ❌ | ✅ | ✅ |
| 智能翻译等基础特色 | ❌ | ✅ | ✅ |
| 互联网公开 | ❌ | ✅ | ✅ |
| 公开我的花园 | ❌ | ✅ | ✅ |
| 花园特色功能 | ❌ | ❌ | ✅ |

来源：<https://www.yuque.com/yuque/ng1qth/member-privilege>

补充官方规则：

- 只能年付，不支持按月购买。
- 会员到期自动降级为普通用户；已上传附件、已分享文档不受影响；但关闭分享后，重新开启需要会员。
- 个人可创建空间上限 10 个（普通/会员相同）。
- App Store 价格可能与官网不同，因为含渠道佣金；官网购买权益一致。
- 流量超限不影响新建纯文字文档，只影响图片/附件/视频上传。
- 文稿数包含小记、文档、表格、画板、数据表；删除/归档不释放计数；每月 1 日清零。
- “协作人数”上限指**单篇文档和单个知识库**的上限，不是账号总数。来源：<https://www.yuque.com/yuque/ng1qth/member-privilege>

来源：<https://www.yuque.com/yuque/ng1qth/member-privilege>

### 8.2 空间版本

官方《空间是什么》（2023-05-08）给出：

| 版本 | 价格 | 适用 |
|---|---|---|
| 标准版 | 免费 | 10 位成员以内小团队，长期免费 |
| 专业版 | 99 元/人/年 | 付费空间 |
| 旗舰版 | 199 元/人/年 | 付费空间 |

- 空间可免费试用专业版/旗舰版 15 天；到期续费或降级。
- 1 人起售；按席位数购买。
- 非营利公益组织、公立教育、创办三年内初创企业可申请免费旗舰版。
- 标准版限制（来自购买文档 FAQ）：10 人、100 团队、100 篇文档/月、1 GB 流量/月。
- 付费到期后恢复标准版限制；超出限制无法新建/编辑，但原有内容可查看。
- 外部联系人不占空间人数，但上限与空间人数一致；离职成员删除后释放账号数。

来源：<https://www.yuque.com/yuque/organizations/rfwopm>、<https://www.yuque.com/yuque/organizations/wvcdiy>

**时效风险**：空间价格来自 2023-05-08 文档；2025-11-18 的购买文档中价格是图片，无法机读；官网定价页是 SPA。原型若展示价格，建议标注“以官网定价页为准”，或只展示“免费/专业/旗舰”三档而不写具体数字。

### 8.3 平台与客户端

| 平台 | 官方信息 | 来源 |
|---|---|---|
| Web | `yuque.com`；支持搜索、文档、知识库、空间 | <https://www.yuque.com/> |
| Windows | 桌面端安装包；首页 appData 显示 `4.2.1.1333` | <https://www.yuque.com/download> |
| macOS | 桌面端安装包；同上 | <https://www.yuque.com/download> |
| iOS | App Store（首页 appData 提供链接） | <https://www.yuque.com/download> |
| Android | 官方下载页（首页 appData 提供链接） | <https://www.yuque.com/download> |
| iPad | 官方文档提到 `ipad 端` | <https://www.yuque.com/yuque/ng1qth/iaxgag> |
| 浏览器扩展 | Chrome 插件 `0.5.9`、Edge 插件 `0.5.7`（首页 appData） | <https://www.yuque.com/> |
| 桌面端更新日志 | 4.2.x（2025-12-24） | <https://www.yuque.com/yuque/yuque-desktop/nttax88unfg9f8qw> |
| 移动端更新日志 | 1.1.7（2025-01-13） | <https://www.yuque.com/yuque/mobile-app/gcv0lme7qwlhvx29> |

**版本口径冲突**：官网 appData 的 `appDownloadVersion`（App 安装包）为 `1.2.2`、`desktopDownloadVersion` 为 `4.2.1.1333`；帮助中心移动端日志最新为 `1.1.7`、桌面端日志为 `4.2.x`。这可能是“安装包版本/日志分组/灰度版本”不同，报告不强行统一。

### 8.4 登录与账号

首页 appData 的登录方式包含：`dingtalk`、`alipay`、`teambition`、`apple`、`wechat`；官方帮助说可通过手机号、微信、钉钉等方式注册。来源：<https://www.yuque.com/>、<https://www.yuque.com/yuque/ng1qth/how-to-register>

### 8.5 开放 API

官方规则（A）：

- 所有开放 API 都需要 `X-Auth-Token`。
- 个人 Token 在“个人设置”获取；团队 Token 在“团队设置”获取，**仅旗舰版空间可使用**。
- Token 依据用户权限决定能获取/修改的数据。
- 接口域名 `https://www.yuque.com`；访问空间内资源要用空间子域名。
- 频控：每小时最多 5000 次，每秒最多 100 次；个人或团队下所有 Token 共享同一额度。
- 响应头含 `X-RateLimit-Limit`、`X-RateLimit-Remaining`。
- 日期时间用 ISO 8601 / UTC。
- 接口有重载：可用 `group_login/book_slug/doc_slug` 或 `book_id/doc_id` 获取文档。
- 2022-04-10 起，个人 Token 只能访问其有权限的文档；团队 Token 可访问团队内文档。

来源：<https://www.yuque.com/yuque/developer/api>、<https://www.yuque.com/yuque/developer/openapi>、<https://www.yuque.com/yuque/developer/vzippmige58g7r9t>

**未确认**：

- 官方提供 OAS YAML / HTML 附件（`yuque_openapi_20260722.yaml`、`.html`），但附件下载需要登录，本次未绕过，因此没有逐条核对 endpoint 清单。
- 个人认证段落带“超级会员专享权益”标签，但官方未明确说明免费用户是否完全不可调用；原型不要展示 API 调用成功。

### 8.6 条款、数据与安全

- 服务提供者：蚂蚁智能（杭州）科技有限公司；服务包括文档编辑、知识管理等。来源：<https://www.yuque.com/terms>
- 账号注销不可恢复；注销后删除账号内所有信息（法律法规另有规定除外）。来源：<https://www.yuque.com/terms>
- 用户原创内容著作权归用户；公开内容授予语雀免费的、非独家使用许可。来源：<https://www.yuque.com/terms>
- 在中国收集和产生的个人信息和其他服务数据存储于中国境内。来源：<https://www.yuque.com/terms>
- 数据安全：金融级网络基础设施、多重多地备份、双重密钥加密、安全认证。来源：<https://www.yuque.com/yuque/ng1qth/safety>
- 隐私政策：`https://www.yuque.com/privacy` 会 302 到 `https://nemo.yuque.com/privacy`，该页面要求登录；本次未能读取正文。**这是调研缺口，不是“没有隐私政策”。**

### 8.7 AI 能力

- 2024-09-13 官方公告：AI 帮你写、写作助手、画板生成、阅读总结/提问、自定义 AI 应用。
- 入口：开始页、知识库 `+`、文档内 `/`、选中文本、阅读页右上角 AI。
- 只有“对话型/文本型”应用支持发布到语雀。

来源：<https://www.yuque.com/yuque/gpvawt/rgg508zoai0i1xfz>

---

## 9. 矛盾、缺口与调研限制

### 9.1 官方资料内部矛盾/时效风险

| # | 矛盾/缺口 | 证据 A | 证据 B | 对原型的处理 |
|---|---|---|---|---|
| 1 | 公开分享是否仅会员 | 《知识库分享》2023-03-20 未提会员限制（<https://www.yuque.com/yuque/thyzgp/collaborators>） | 《如何分享文档》2024-05-22 明确“公开分享目前只支持会员使用”（<https://www.yuque.com/yuque/thyzgp/bosazp>） | 按 2024 文档：公开分享需会员；非会员显示升级提示 |
| 2 | 空间价格 | 《空间是什么》2023-05-08：专业 99、旗舰 199（<https://www.yuque.com/yuque/organizations/rfwopm>） | 《如何购买语雀空间》2025-11-18 价格是图片，无法机读（<https://www.yuque.com/yuque/organizations/wvcdiy>） | 只展示档位，不写具体价格；或标注“以官网为准” |
| 3 | 桌面/App 版本 | 首页 appData：桌面 `4.2.1.1333`、App `1.2.2`（<https://www.yuque.com/>） | 更新日志：桌面 `4.2.x`、移动端 `1.1.7`（<https://www.yuque.com/yuque/yuque-desktop/nttax88unfg9f8qw>、<https://www.yuque.com/yuque/mobile-app/gcv0lme7qwlhvx29>） | 原型不显示版本号，或显示“4.2.x / 1.2.x” |
| 4 | 下划线 Markdown | 快捷键表“下划线”写 `*斜体* Space`（<https://www.yuque.com/yuque/gpvawt/shortcut>） | 同一表“斜体”也是 `*斜体* Space`；下划线快捷键 `⌘U` | 视为文档笔误；原型只保留 `⌘U` |
| 5 | 文档“发布”还是“更新” | 《如何发布文档》说未发布/已发布，发布后按钮变“更新”（<https://www.yuque.com/yuque/thyzgp/oe6ium>） | 编辑器截图顶部按钮显示“更新”（<https://cdn.nlark.com/yuque/0/2023/png/34713865/1683877694529-4a41cc64-9bc0-4db2-a6c3-8deb60dc6696.png>） | 两态都实现：首次“发布”，再次“更新” |
| 6 | 知识库公开与文档私密 | 《知识库分享》说知识库公开后下辖所有文档公开（<https://www.yuque.com/yuque/thyzgp/collaborators>） | 《个人版权限》说文档公开性可突破上级知识库（<https://www.yuque.com/yuque/ng1qth/gxy3gk>） | 解释为“默认继承，单篇可覆盖”；UI 给单篇开关 |
| 7 | 标签端支持 | 《如何使用标签》说“目前仅支持网页端”（<https://www.yuque.com/yuque/thyzgp/icl7ey>） | 会员/桌面文档未提标签 | 原型只在 Web 视图显示标签 |
| 8 | 团队任务优先级档位 | 正文说“支持以下几种优先级设置”，但档位在图片中（<https://www.yuque.com/yuque/organizations/cmcg4n>） | 正文未列具体值 | 不写“高/中/低”以外的档位；原型可用三档 |
| 9 | API 个人权限 | API 概览个人认证段带“超级会员专享权益”标签（<https://www.yuque.com/yuque/developer/api>） | 历史 API 对个人开放；文档未明确免费用户是否可调用 | 原型不展示真实 API 调用；只做“开放能力”介绍 |

### 9.2 无法确认的内容

- 完整的 2026 年 design token 系统、阴影和动效曲线仍未确认；§11 已补充所访问页面的实测字号、行高、圆角与颜色，不能推广为全产品规范。
- 官网首页 Hero 文案、顶部导航、页脚链接（SPA，未渲染）。
- 隐私政策正文（`nemo.yuque.com/privacy` 登录墙）。
- 开放 API 的完整 endpoint 清单与字段（OAS 附件登录墙）。
- 文档评论的具体交互：是否支持划线评论、段落评论、@、表情、解决状态、通知设置。
- 多人同时编辑的冲突解决、光标/选区展示、离线策略。
- 自动保存的具体间隔、历史版本保留策略。
- 搜索排序、分词、是否覆盖评论/附件。
- 知识库封面/背景的自定义范围与默认样式。
- 模板中心的具体模板数量与分类。
- 移动端/iPad 的实际布局与手势。
- 团队任务优先级的完整档位、看板视图是否存在（官方只给任务列表/详情描述）。
- 空间价格在 2026 年的最新数字。
- 个人免费用户是否可调用开放 API。

### 9.3 调研限制

1. **官方资料子任务不登录**：§0–10 依据公开官方文档与截图；主任务随后完成已登录 Edge 实机观察，见 §11。未安装或运行语雀原生客户端，也未验证多人协作和全部边缘状态。
2. **帮助文档滞后**：大量文档更新于 2023 年，而产品已到 2025/2026 版本；截图也可能是旧版 UI。报告已标注每篇文档的更新时间。
3. **官网 SPA**：官方资料子任务只读取静态 HTML 的标题/meta 与 appData；主任务打开 `yuque.com` 后因已有登录态进入工作台，因此仍未核对未登录营销页、定价页与下载页的完整渲染布局。
4. **截图测量误差**：布局数值来自官方帮助截图，受缩放、裁切、版本差异影响；只能作为原型近似值。
5. **官方文档自身错误**：如快捷键表下划线 Markdown 笔误；报告已指出，不照抄。
6. **无用户研究**：目标用户、项目映射为推断（D），没有官方用户画像或使用数据。
7. **未做竞品对比**：本文只研究语雀官方资料，不引用第三方评测。

---

## 10. 官方 URL 索引

**官网与产品页**

- <https://www.yuque.com/>
- <https://www.yuque.com/help>
- <https://www.yuque.com/about/price>
- <https://www.yuque.com/download>
- <https://www.yuque.com/terms>
- <https://www.yuque.com/privacy>（302 到登录墙）
- <https://www.yuque.com/yuque/developer>
- <https://www.yuque.com/yuque/blog>

**产品与帮助中心**

- <https://www.yuque.com/yuque/ng1qth/about>
- <https://www.yuque.com/yuque/ng1qth/why>
- <https://www.yuque.com/yuque/ng1qth/dashboard>
- <https://www.yuque.com/yuque/ng1qth/gxy3gk>
- <https://www.yuque.com/yuque/ng1qth/member-privilege>
- <https://www.yuque.com/yuque/ng1qth/iaxgag>
- <https://www.yuque.com/yuque/ng1qth/search-syntax>
- <https://www.yuque.com/yuque/ng1qth/xnlf9g>
- <https://www.yuque.com/yuque/ng1qth/how-to-register>
- <https://www.yuque.com/yuque/ng1qth/safety>
- <https://www.yuque.com/yuque/thyzgp/repository>
- <https://www.yuque.com/yuque/thyzgp/gbdfpb>
- <https://www.yuque.com/yuque/thyzgp/vlki63>
- <https://www.yuque.com/yuque/thyzgp/hof3ll>
- <https://www.yuque.com/yuque/thyzgp/collaborators>
- <https://www.yuque.com/yuque/thyzgp/qnxyk7>
- <https://www.yuque.com/yuque/thyzgp/nbkoqg>
- <https://www.yuque.com/yuque/thyzgp/oe6ium>
- <https://www.yuque.com/yuque/thyzgp/my8uax>
- <https://www.yuque.com/yuque/thyzgp/bosazp>
- <https://www.yuque.com/yuque/thyzgp/fs0x2z>
- <https://www.yuque.com/yuque/thyzgp/doi9nz>
- <https://www.yuque.com/yuque/thyzgp/yee00736hhtea3er>
- <https://www.yuque.com/yuque/thyzgp/icl7ey>
- <https://www.yuque.com/yuque/thyzgp/import-lake-to-lark>
- <https://www.yuque.com/yuque/thyzgp/recycle-bin>
- <https://www.yuque.com/yuque/gpvawt/gh4227>
- <https://www.yuque.com/yuque/gpvawt/block>
- <https://www.yuque.com/yuque/gpvawt/fq1pad>
- <https://www.yuque.com/yuque/gpvawt/shortcut>
- <https://www.yuque.com/yuque/gpvawt/lnobo9>
- <https://www.yuque.com/yuque/gpvawt/aint7x>
- <https://www.yuque.com/yuque/gpvawt/gh3mdb>
- <https://www.yuque.com/yuque/gpvawt/rms0so>
- <https://www.yuque.com/yuque/gpvawt/uilmir>
- <https://www.yuque.com/yuque/gpvawt/fuu6h3>
- <https://www.yuque.com/yuque/gpvawt/hig7ae>
- <https://www.yuque.com/yuque/gpvawt/rgg508zoai0i1xfz>
- <https://www.yuque.com/yuque/ekrzd1/dqdt6f>

**空间与团队**

- <https://www.yuque.com/yuque/organizations/rfwopm>
- <https://www.yuque.com/yuque/organizations/mfxpek>
- <https://www.yuque.com/yuque/organizations/ac3t9l>
- <https://www.yuque.com/yuque/organizations/cmcg4n>
- <https://www.yuque.com/yuque/organizations/pe0o80>
- <https://www.yuque.com/yuque/organizations/wvcdiy>
- <https://www.yuque.com/yuque/organizations/ghaihu>
- <https://www.yuque.com/yuque/organizations/cf5gr8>

**开发者**

- <https://www.yuque.com/yuque/developer/api>
- <https://www.yuque.com/yuque/developer/openapi>
- <https://www.yuque.com/yuque/developer/vzippmige58g7r9t>
- <https://www.yuque.com/yuque/developer/fadm9di79hgo6ggw>（接口列表入口，重定向到 openapi）

**版本**

- <https://www.yuque.com/yuque/yuque-desktop>
- <https://www.yuque.com/yuque/yuque-desktop/nttax88unfg9f8qw>
- <https://www.yuque.com/yuque/yuque-desktop/keyboard-shortcut>
- <https://www.yuque.com/yuque/mobile-app>
- <https://www.yuque.com/yuque/mobile-app/gcv0lme7qwlhvx29>

**公开 CSS**

- <https://gw.alipayobjects.com/os/chair-script/skylark/pc.9283cd96.css>
- <https://gw.alipayobjects.com/os/chair-script/skylark/larkui.feecef61.css>

**官方帮助截图**

- 工作台：<https://cdn.nlark.com/yuque/0/2022/png/85538/1667800326819-4329e987-21f3-476b-a453-0d6ab1cb1cfe.png>
- 知识库首页：<https://cdn.nlark.com/yuque/0/2022/png/519985/1666856932541-c67d6bf5-5d49-4a87-9f11-4682fe47a669.png>
- 文档编辑器：<https://cdn.nlark.com/yuque/0/2023/png/34713865/1683877694529-4a41cc64-9bc0-4db2-a6c3-8deb60dc6696.png>
- 插入卡片面板：<https://cdn.nlark.com/yuque/0/2023/png/21674429/1682402707177-6e5f1d4a-b0c9-448d-95bf-caf7225d4958.png>
- 公开知识库页：<https://cdn.nlark.com/yuque/0/2022/png/303152/1666063589803-5ec0747d-993d-4d0c-9490-167ab102cc3f.png>
- 画板编辑器：<https://cdn.nlark.com/yuque/0/2023/png/21674429/1682402751867-74f217d3-f443-4b0e-aecc-ddc9050e540a.png>

---

### 附：一句话给主 agent

**做“语雀”，先做“书”：左侧目录、中间窄栏正文、右侧大纲；绿色只给主操作，细线替阴影，文档列表用“标题 + 虚线 + 日期”。项目与待办不要另造对象——个人用小记/任务列表，团队用空间/团队 + 知识库 + 团队任务。**


---

## 11. 已登录 Edge 实机观察（E）

### 11.1 访问范围与安全边界

2026-11-16，使用 Kimi WebBridge 访问用户 Edge 中的语雀登录态。打开官网后进入工作台，随后访问知识库、文档阅读页、文档编辑态、小记，使用 `Ctrl+J` 唤起搜索。操作仅限导航、阅读与进入编辑态；未输入或保存真实文档，未发布小记，未修改分享设置。

| 页面 | 实际 URL | 观察内容 |
|---|---|---|
| 工作台 | <https://www.yuque.com/dashboard> | 全局导航、六个快捷入口、最近文档及筛选 |
| 知识库 | <https://www.yuque.com/grreesd/aqkcie> | 知识库首页、双份目录、分组、统计、收藏/分享 |
| 文档阅读/编辑态 | <https://www.yuque.com/grreesd/aqkcie/oai2e4ni6eas297z> | 标题、阅读/编辑切换、格式工具条、模板推荐、评论区 |
| 小记 | <https://www.yuque.com/dashboard/notes> | 双栏记录与时间流、置顶、选择、搜索 |
| 全局搜索 | 小记页按 `Ctrl+J` | 搜索对话框语义与占位文案 |

这些是账号内页面，其他用户未必有访问权限。URL 用于定位观察来源，不代表公开分享。文档正文和小记均未作为公开调研素材；临时截图已清理。HTML 中的正文与小记使用通用演示内容，不嵌入账号数据接口。

### 11.2 当前工作台：不是旧版三卡入口

实机左侧一级导航为「开始 / 小记 / 收藏 / 逛逛」，下接知识库列表；底部「更多」。工作台是最近内容的再进入入口，不是经营数据仪表盘。

顶部实际出现六个快捷入口：

1. **新建文档**：文档、表格、画板、数据表。
2. **新建知识库**。
3. **模板中心**。
4. **百宝箱 TboxClaw**：PPT、生图、可视化页面等说明。
5. **AI 对话写文档**：界面显示 Deepseek。
6. **语雀 Board**：标记「即将上线」，不能据此认为功能已经可用。

下方文档区域以「编辑过 / 浏览过 / 我点赞的 / 我评论过」分段，配合类型、归属、创建者筛选，表格显示标题、归属和时间。这与 §3、§7 基于 2022–2023 年帮助截图的「三个快捷入口、协作过」不同。HTML 以当前实见结构为主。来源：<https://www.yuque.com/dashboard>。

### 11.3 知识库：相同目录，两种密度

实机知识库首页展示标题、锁、文档数/字数、收藏/分享/更多；中央有欢迎语：

> 知识库就像书一样，让多篇文档结构化，方便知识的创作与沉淀。

左栏提供紧凑、可折叠的目录树，中央再次展示较宽松的目录，使用标题、点状引导线和时间。两处不是无意义重复：左栏服务持续定位和跳转，中央目录服务整体浏览、理解知识结构。这一目的判断为 **D**；两份目录及其样式为 **E**。

原型保留了所见知识库列表与 `Goal → Learn / Do`、每日记录等目录结构特征，正文另用演示内容。没有从目录名称推断真实项目状态。来源：<https://www.yuque.com/grreesd/aqkcie>。

### 11.4 文档：阅读与编辑分态

阅读态保留知识库目录、正文和操作栏；正文下可见点赞、元信息与评论编辑区。因此不能把「评论」概括为必然占用右侧抽屉。空文档出现「若有收获，就点个赞吧」。

点击「编辑」后，右上主操作变为「更新」，出现横向格式工具条；标题为可编辑输入区，正文具有 `contenteditable`，样式类包含 `ne-typography-classic` 与 `ne-paragraph-spacing-relax`。空正文还展示「我的模板」、会议记录、工作周报、项目复盘等起步入口。

**验证限制**：本次真实编辑态持续显示「正在加载」，部分工具条为 disabled；等待加载提示消失超时。因此只确认上述可见结构，不能声称语雀完整编辑器已经加载成功、能保存或支持本次未操作的格式命令。没有为了验证而在账号中输入内容。来源：<https://www.yuque.com/grreesd/aqkcie/oai2e4ni6eas297z>。

### 11.5 小记与搜索

实机小记是双栏：左侧记录区，提供附件、标签、「小记一下」与 `Ctrl+Enter`；右侧是时间流卡片，提供置顶、选择与搜索。它降低记录前必须选择知识库和目录的负担；后一句是设计推断 **D**。本次不收录右侧私人记录正文。来源：<https://www.yuque.com/dashboard/notes>。

`Ctrl+J` 成功唤起全局搜索，语义为 dialog，占位文案为：

> 搜索内容，或输入 > 唤醒更多

这表明入口除了查找内容还暴露命令模式。未测得该浮层的精确尺寸，也未验证搜索排序、全部命令及搜索权限规则。原型中的搜索面板尺寸为实现近似值，不冒充实机测量。

### 11.6 实测视觉值与旧版估算修正

下表为所访问页面的 computed style 或元素边界，不能当作所有平台/缩放下的固定规范。工作台观察视口为 2166×1274、DPR 1；知识库观察视口为 2166×1257。

| 项目 | 本次实测值（E） | 原型处理 |
|---|---|---|
| UI 字体 | `Chinese Quote, -apple-system, BlinkMacSystemFont, Segoe UI, Roboto, PingFang SC, Hiragino Sans GB, Microsoft YaHei, Helvetica Neue, Helvetica, Arial, sans-serif` | 使用相同字体栈，不加载外部字体 |
| 基础文字 | 14px；`#262626` | 作为 UI 基础 |
| 工作台标题 | 18px / 500；行高约 28.287px | 使用 18px / 500，不沿用旧截图的 24–28px |
| 工作台导航项 | 32px 高；选中底 `#EFF0F0`；圆角 6px | 保留紧凑灰阶导航，局部高度近似 |
| 工作台侧栏 | 约 252px | 桌面工作台使用 252px |
| 知识库侧栏 | 该宽屏视口下约 389px | 使用响应式宽度，非所有尺寸固定 389px |
| 文档 H1 | 36px / 700；行高约 50.004px | 使用 36px 粗体标题 |
| 正文 | 15px；行高 26.1px | 采用接近 1.74 的正文行高 |
| 主操作「编辑」 | `#00B96B`、白字、32px 高、6px 圆角 | 绿色用于主要操作 |
| 次按钮边框 | `#E7E9E8` | 细边框而非阴影 |
| 次级文字 | 观察元素为 `#8A8F8D` | 用于元信息和弱提示 |
| 知识库欢迎正文 | 该视口下约 960px 宽 | 宽松主区，与左侧紧凑目录形成对比 |

来源分别为上表页面的实时 DOM/样式读取；§6 的旧帮助截图估算仍保留作历史参考，冲突时以上表为准。未验证暗色主题、原生移动客户端或平板布局。

---

## 12. 设计逻辑综合：对 kanon 有什么启发（D）

以下为官方结构与实机观察的综合解释，不是语雀团队公开的设计意图。

### 12.1 核心不是「页面」，而是「成书」

知识库用主题确定边界，用目录确定顺序与层级，用文档承载具体知识。首页的书目式列表与阅读页的常驻目录，共同让用户知道「我在一本什么书里，这篇在什么位置」。官方知识库定义与实机欢迎语相互印证。依据：<https://www.yuque.com/yuque/thyzgp/repository>、<https://www.yuque.com/grreesd/aqkcie>。

### 12.2 同一份内容，提供三个入口层级

- **工作台**回答「我刚才处理了什么」：以行为和时间重返内容。
- **知识库**回答「它属于什么体系」：以结构定位内容。
- **搜索**回答「我记得某个词或要执行什么」：跨目录查找和调用命令。

这不是三个独立数据系统，而是围绕同一内容提供不同检索路径。依据：<https://www.yuque.com/dashboard>、<https://www.yuque.com/yuque/ng1qth/dashboard>、§11.5。

### 12.3 捕获与整理分离，阅读与编辑分离

小记先容纳碎片，知识库再负责整理与沉淀；阅读态减少工具干扰，编辑态再暴露格式与更新操作。两组分离都避免用户在每一步承担完整复杂度。依据：<https://www.yuque.com/yuque/ekrzd1/dqdt6f>、<https://www.yuque.com/dashboard/notes>、§11.4。

### 12.4 灰阶承担结构，绿色只标少数关键动作

界面层次主要来自留白、字重、浅灰选中底和细线；绿色主按钮因稀少而容易识别。导航紧凑、正文舒展，两种密度服务不同任务，而不是整页统一「大卡片化」。依据：§6 的官方 CSS 与 §11.6 实测值。

### 12.5 不要把知识组织误当任务管理

已查资料把项目组映射为团队，把任务作为团队协作能力；本次个人登录态也未看到独立「项目」实体或个人任务中心。这个证据范围不足以证明全产品永远没有此类实体，但足以说明：**不能仅因目录叫 TODO，就将它解读为专门任务系统**。依据：<https://www.yuque.com/yuque/organizations/mfxpek>、<https://www.yuque.com/yuque/organizations/cmcg4n>、§11。

对 kanon 的可借鉴点是「轻量捕获 → 结构化组织 → 专注阅读/编辑」与多入口检索；是否引入明确的项目、任务实体，应由 kanon 自己的需求决定，而不是把语雀目录名称直接当领域模型。

---

## 13. HTML 交付、验证与边界

### 13.1 交付形式

文件：[yuque.html](yuque.html)。CSS、JavaScript、SVG 均内联，不需安装依赖或运行构建；可直接用浏览器打开。默认进入工作台，可沿侧栏打开知识库与文档。

这是本地交互原型，不是语雀客户端，不访问语雀 API，不读取浏览器的语雀登录凭据。演示状态使用 `localStorage` 键 `yuque-prototype-v1`；从不同来源（例如 `file://` 与本机 HTTP 地址）打开时，存储空间可能不同。浏览器禁止本地存储时，不保证刷新后保留数据。

### 13.2 已实现范围

| 区域 | 本地可交互行为 |
|---|---|
| 工作台 | 最近文档、浏览/点赞/评论筛选、类型/归属筛选、知识库导航 |
| 知识库 | 首页、层级目录、折叠、文档跳转、收藏、新建知识库 |
| 文档 | 阅读/编辑切换、标题与正文输入、基础格式、更新、正文大纲、点赞、收藏 |
| 搜索 | `Ctrl/⌘+J`、关键词检索、文档/知识库/小记结果、`>` 命令入口 |
| 新建与模板 | 新建文档、选择类型、使用示例模板起步 |
| 小记 | 发布、搜索、置顶、选择删除 |
| 分享 | 弹窗与本地演示地址复制；不改变真实访问权限 |
| 存储 | 文档、知识库、小记与收藏等写入本地浏览器存储 |

### 13.3 验证记录

在 Edge 的本机 HTTP 预览中，以 DOM 点击、输入事件、表单提交执行了 15 项检查，均通过：

1. 初始工作台渲染。
2. 点赞筛选。
3. 新建文档弹窗。
4. 创建文档并进入编辑。
5. 更新后返回阅读。
6. 收藏状态切换。
7. 分享弹窗。
8. 全局搜索命中。
9. 搜索结果打开文档。
10. 发布小记。
11. 小记置顶。
12. 删除小记。
13. 新建知识库。
14. 本地存储写入。
15. 该流程无运行时 error。

随后刷新并读取新建文档和知识库，确认本次浏览器环境中的持久化成功；测试数据已恢复为初始演示数据。另检查过 1440px 桌面的工作台、知识库与文档，以及 390px 手机宽度的工作台、目录和阅读布局；修复了手机阅读布局在隐藏侧栏后落入错误网格列的问题。这里的手机布局是原型的响应式适配，不宣称复制了语雀原生 App。

内联 JavaScript 语法检查通过。此次只交付 HTML 与调研文档，未修改 Rust/NSH 应用，因此没有运行与本原型无关的 Cargo 或 NSH 测试。

### 13.4 明确未实现或未完整还原

- 没有真实账号、服务端同步、权限校验、公开分享、多人协作、文件上传、支付与 AI 生成。
- 编辑器使用 `contenteditable` 与浏览器基础格式命令，不是完整 Lake 编辑器；未实现完整块拖拽、嵌套卡片、富媒体与协同冲突解决。
- 表格、画板、数据表可选择类型和显示相应标识，但没有对应的专业编辑引擎。
- 历史、协作、AI、演示等部分入口仅展示说明，不提供真实功能；§7 的历史恢复、完整分享权限等是建议规格，不是本次已完成项。
- 评论区可演示回复计数变化，但没有完整保存/展示评论正文，不属于完整评论系统。
- 图标和字标为手工近似；尺寸按实机与响应式需求折算，并非逐像素截屏复刻。
- 未验证暗色主题、全部键盘/辅助技术流程、语雀原生客户端和完整移动交互；不能以本次通过的主要流程推导全部产品行为都已覆盖。

**最终边界：这是一份基于真实登录态观察与官方资料的可交互界面研究样本，不是可以替代语雀的生产系统。**
