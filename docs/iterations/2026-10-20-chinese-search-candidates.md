# 主搜索中文反查与候选扩容

2026-10-20。执行计划见 `2026-10-20-chinese-search-candidates-plan.md`，API说明见 `docs/api/suggestions.md`。

## 行为

- 主搜索含汉字时用现有glm-embedding-3查询已初始化词汇；混合输入整体检索，德语变音字符仍走原词形/拼写候选。单字中文可查，输入法组词时暂停，确认后约200ms防抖刷新。
- 中德文主候选一次最多100条、初显20、按20展开。更多按钮只扩展本地显示；新查询重置数量与滚动。其他API调用方仍默认lexical/10。
- 保留原Tab联想线索入口。无向量重建、新依赖、数据库迁移或CLI/MCP工具扩展；核对所有候选调用方，文章两处调用保持默认参数。
- 请求有版本检查与取消，覆盖旧成功、旧错误、A→B→A、清空/关闭。回车复用当前在途请求，不能选择过期首项；中文无结果或错误不会初始化原中文。重试先将焦点移回输入框，防止状态替换移除按钮后面板意外关闭。
- 首页聚焦与结果页展开面板均按实际位置、visualViewport和可见底栏限高；标题不滚，列表最大420px并内部滚动。组件样式使用两套主题token。

## 验证

- Rust库测试49通过、1项原有隔离数据库测试跳过；前端check无错误，保留原有2条警告。前后端生产镜像构建成功。
- 公开API：`ver&limit=100`返回100、缺省返回10；非法模式、limit=0/101返回400。“收据”返回100个非重复有效entry_id，前5为Kassenzettel/Rechnung/Rezept/Kontoauszug/Quittung，首项可用详情接口读取；约2.39秒含详情请求。
- 真实浏览器首页：canvas/canvas-dark，1280×900、900×650、390×844、320×568、844×390；展开20→100未增高面板、document或main-area，未新增HTTP请求，末项可到达，无横向溢出或页面异常。中文“预约”首批约2.32秒，后续列表能找到Termin。
- 结果页展开搜索：同样的双主题、5种尺寸通过限高与页面高度断言；模拟visualViewport高度360、offsetTop=20时，面板底部368仍在可见底部380以内。此项是浏览器模拟，不是手机实测。
- 浏览器模拟异常：A→B→A、旧成功/失败、清空、关闭后迟到响应、中文空结果与错误不提交、重试、Tab切换、回车等待在途候选均通过。德语候选失败仍保留原词查询回退。模拟composition事件期间零查询/提交，确认单字“爱”后仅一次semantic请求。
- 自动化入口为 `frontend-web/tests/searchCandidates.browser.js`、`searchCandidateLifecycle.browser.js`、`searchCandidateResultLayout.browser.js`，通过playwright-cli的 `run-code --filename=...` 执行。生命周期测试拦截生成请求，不写数据库；真实检索检查使用已有词汇。
- 截图保存在本地 `output/playwright/search-candidates-*`（含改前）、`search-result-candidates-*`，不将大批图片提交到仓库。

## 局限

语义近邻仍会误排；本轮不加翻译/生成重排和可靠性阈值。浏览器自动化的composition事件和visualViewport模拟不能替代真实手机键盘验证。

架构复核结论：结构合理。handler校验请求、query服务协调模型与候选、knowledge仓库拥有只读SQL；模式定义在models，服务不反向依赖handler。前端请求/焦点/viewport生命周期均随现有搜索组件销毁清理，没有另加缓存服务、分页框架或重复状态层。
