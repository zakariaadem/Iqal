# Al-Dir — 简体中文

app-name = Al-Dir
app-description = 网站和应用拦截器
menu = 菜单
menu-import = 导入拦截组…
menu-export-all = 导出全部拦截组…
menu-activity = 最近活动
menu-settings = 设置
menu-about = 关于 Al-Dir

action-new-block = 新建拦截组
action-remove = 移除
action-add = 添加
action-cancel = 取消
action-delete = 删除
action-create = 创建
action-export-block = 导出此拦截组
action-delete-block = 删除此拦截组
action-import-text = 从文本文件导入…
action-add-installed-app = 添加已安装的应用…
action-add-program-file = 添加程序文件…
action-add-times = 添加这些时间
action-cancel-unlock = 取消解锁
action-learn-location = 我现在就在这里
action-unlock = 解锁…
action-remove-lock = 移除锁
action-change-lock = 更改锁…
action-lock = 上锁
action-choose-on-map = 在地图上选择…
action-use-area = 使用此区域

list-empty = 暂无内容。

state-off = 已关闭
state-active = 生效中
state-scheduled = 按时间表
state-locked = 已上锁

error-title = 出错了
error-service-unreachable = Al-Dir 服务未运行。请安装或启动它，然后重试。
error-unexpected = Al-Dir 服务返回了意外的结果。
error-locked = 此拦截组已上锁，而这项更改{ $reason }。已上锁的拦截组只能变得更严格。
error-wrong-answer = 不正确。
error-cannot-unlock-early = 此锁不能提前打开。
error-not-admin = 只有管理员可以更改 Al-Dir 的设置。

reason-disables = 会关闭它
reason-fewer-users = 会让它适用于更少的人
reason-web-mode = 会改变网站的拦截方式
reason-removes-websites = 会移除被拦截的网站
reason-adds-allowed-websites = 会允许更多网站
reason-safesearch-off = 会关闭安全搜索
reason-app-mode = 会改变应用的拦截方式
reason-removes-apps = 会移除被拦截的应用
reason-adds-allowed-apps = 会允许更多应用
reason-adds-schedule = 会把它限制在特定时间
reason-shortens-schedule = 会缩短它的时间
reason-adds-allowance = 会增加使用时长额度
reason-increases-allowance = 会提高使用时长额度
reason-removes-lock = 会移除锁
reason-shortens-delay = 会缩短解锁前的等待时间
reason-weakens-lock = 会削弱锁
reason-pending-unlock = 会更改等待中的解锁
reason-deletes = 会删除它
reason-hardened-off = 会关闭强化模式

empty-title = 还没有拦截组
empty-detail = 拦截组是一组要拦截的网站和应用，有自己的时间和锁。点击“新建拦截组”创建一个。

block-summary = 网站规则 { $web } 条 · 应用规则 { $apps } 条
rules-count = { $count } 条规则

locked-banner = 此拦截组已上锁。你可以添加限制，但不能移除。
tab-websites = 网站
tab-apps = 应用
tab-users = 用户
tab-schedule = 时间
tab-lock = 锁
block-enabled = 开启或关闭拦截组
confirm-delete = 删除“{ $name }”？
confirm-delete-detail = 它的网站、应用、时间和锁都会被移除。
rule-invalid = 无法使用“{ $rule }”：{ $problem }

web-mode = 网站拦截方式
web-mode-block = 拦截列表中的网站
web-mode-allow-only = 只允许列表中的网站
web-blocked = 被拦截的网站
web-also-blocked = 即使被允许也拦截
web-placeholder = example.com, example.com/page, contains:词, search:词语
web-syntax = example.com 拦截该网站及其所有子域名。example.com/videos 只拦截这些页面。contains:casino 拦截包含该词的任何地址。search:免费电影 拦截包含这些词的搜索。
web-allowed = 允许的网站
web-exceptions = 例外（始终允许）
web-safesearch = 强制安全搜索（Google、Bing、DuckDuckGo、YouTube）
web-partial = 部分拦截
web-partial-detail = 只拦截网站中让人分心的部分，其余照常使用。
partial-youtube-shorts = YouTube Shorts
partial-youtube-comments = YouTube 评论
partial-youtube-recommendations = YouTube 推荐和首页
partial-reddit-home = Reddit 首页、r/all 和 r/popular
partial-facebook-feed = Facebook 动态、Watch 和 Reels
partial-instagram-reels = Instagram Reels
partial-instagram-explore = Instagram 发现
partial-x-trends = X（Twitter）探索和趋势

import-done = 导入完成
import-result = 已添加 { $added } 条规则，跳过 { $skipped } 行。
import-added = 已添加：{ $name }
import-replaced = 已更新：{ $name }
import-rejected = 未导入：{ $name }（{ $why }）

apps-mode = 应用拦截方式
apps-mode-block = 拦截列表中的应用
apps-mode-allow-only = 只允许列表中的应用
apps-blocked = 被拦截的应用
apps-also-blocked = 即使被允许也拦截
apps-exceptions = 例外（始终允许）
apps-allowed = 允许的应用
apps-placeholder = steam, /opt/games/, title:YouTube, cmd:minecraft
apps-syntax = 名称会拦截同名程序。以 / 结尾的文件夹会拦截其中的所有程序。title:文字 会关闭标题包含该文字的窗口。cmd:文字 会拦截命令行包含该文字的程序。改名后的副本可用 hash: 规则识别。

users-applies-to = 此拦截组适用于谁
users-all = 所有人
users-all-except = 所有人，除了…
users-only = 仅限…
users-except-these = 除了这些人
users-only-these = 仅限这些人
users-note = 本电脑的管理员也会被拦截，除非在这里排除。系统本身永远不会被拦截。

day-mon = 周一
day-tue = 周二
day-wed = 周三
day-thu = 周四
day-fri = 周五
day-sat = 周六
day-sun = 周日
time-from = 从
time-to = 到
time-at = 于

schedule-limited = 仅在特定时间
schedule-limited-detail = 拦截组只在下面的时间内生效。
schedule-always = 拦截组始终生效。
schedule-times = 时间
schedule-date-placeholder = 一次性：2026-10-20 08:00 2026-10-22 18:00
schedule-date-invalid = 一次性时段请这样写：开始日期、开始时间、结束日期、结束时间（2026-10-20 08:00 2026-10-22 18:00）。
schedule-add-weekly = 添加每周时间
schedule-no-days = 请至少选择一天。
allowance = 每日使用时长
allowance-detail = 每天允许在有限时间内使用被拦截的网站和应用；用完后即被拦截。
allowance-minutes = 每天分钟数

area-circle = 距 { $lat }, { $lon } { $radius } 米以内
area-polygon = 在一个有 { $count } 个角的区域内
lock-desc-until = 锁定至 { $time }
lock-desc-schedule = 在 { $times } 期间锁定
lock-desc-password = 用密码锁定
lock-desc-random = 输入 { $length } 个随机字符即可解锁
lock-desc-restart = 锁定至电脑重启
lock-desc-location = 当这台电脑{ $area }时锁定
lock-desc-delay = 回答后 { $minutes } 分钟打开

lock-none = 未上锁
lock-none-detail = 没有锁时，这个拦截组可以随时更改或关闭。上锁后，在锁结束前只能变得更严格。
lock-active = 已上锁
lock-inactive = 已设置锁，目前未生效
lock-pending = 锁将在 { $time } 打开。
location-learn-detail = 如果你现在就在这个地方，让 Al-Dir 记住这里的 Wi-Fi 网络。这样即使没有网络，Al-Dir 也知道你在这里。
location-learned = 已记住此地点
location-learned-detail = 此地点现已记住 { $count } 个 Wi-Fi 网络。
location-none-chosen = 尚未选择区域。

lock-kind-duration = 一段时间
lock-kind-until = 到某个日期
lock-kind-password = 用密码
lock-kind-random = 输入随机文字
lock-kind-restart = 到电脑重启
lock-kind-schedule = 在特定时间
lock-kind-location = 在某个地点时
lock-dialog-title = 锁定此拦截组
unit-days = 天
unit-hours = 小时
unit-minutes = 分钟
unit-metres = 米
password = 密码
password-repeat = 再次输入密码
password-generate = 生成一个由别人保管的密码
password-generate-detail = Al-Dir 会生成一个密码并只显示一次。请交给你信任的人。
password-mismatch = 密码为空或两次不一致。
password-generated = 锁的密码
password-generated-detail = { $password }

    请交给你信任的人。它不会再次显示。
random-length = 需要输入的字符数
lock-restart-detail = 在这台电脑重启之前，拦截组会一直保持锁定。
lock-location-detail = 当这台电脑在区域内时，以及每当 Al-Dir 无法判断它在哪里时，拦截组都会保持锁定。
unlock-delay = 解锁后等待
unlock-delay-detail = 设置等待后，正确的回答要过这段时间才会打开锁。这有助于克服一时的冲动。
lock-duration-zero = 请选择时长。
lock-until-past = 这个时间已经过去了。
lock-confirm = 锁定此拦截组？
lock-confirm-detail = { $lock }。在那之前，此拦截组中的任何内容都不能被移除、放宽或关闭。

unlock-not-possible = 此锁不能提前打开
unlock-not-possible-detail = 时间结束或你离开该地点时，它会自动打开。
unlock-title = 解锁
unlock-type-text = 请准确输入这段文字（无法粘贴）：
unlock-enter-password = 输入锁的密码：
unlocked = 已解锁
unlocked-detail = 现在可以再次更改此拦截组。
unlock-pending = 已开始解锁

map-title = 选择区域
map-help = 点击地图放置区域中心，然后设置大小。拖动可移动地图，滚动滚轮可缩放。
map-radius = 半径

new-block-name = 拦截组名称
new-block-placeholder = 例如：社交媒体
export-done = 已导出

activity-empty = 还没有拦截过任何内容。
event-app-start = 应用被阻止
event-app-killed = 应用被关闭
event-window-closed = 窗口被关闭
event-site-blocked = 网站被拦截
event-page-blocked = 页面被拦截

settings-language = 语言
settings-language-system = 与系统相同
settings-language-restart = 重启 Al-Dir 以使用新语言。
settings-hardened = 强化模式
settings-hardened-detail = 只要有拦截组处于锁定状态，受其约束的人就无法使用 sudo 或管理员密码提示，因此无法停止 Al-Dir。紧急访问：重启进入恢复模式。有内容被锁定时无法关闭。
settings-status = 状态
status-service = Al-Dir 服务 { $version }
status-time = 时间：{ $source }
status-location = 位置：{ $state }
status-extensions = 浏览器扩展：{ $list }
status-no-extensions = 未连接
status-not-admin = 你可以查看拦截组，但不能更改（仅限管理员）。

event-allowance-low = 使用时长即将用完
event-allowance-used = 使用时长已用完

web-lists = 预设列表
web-lists-detail = 现成的网站合集。成人内容、赌博和社交媒体列表每天从一个大型社区列表更新。
list-social = 社交媒体
list-adult = 成人内容
list-gambling = 赌博
list-search = 搜索引擎
list-news = 新闻
list-video = 视频和直播
list-games = 游戏
list-shopping = 购物
list-count = { $count } 个网站
allowance-used-today = 今天已用：{ $used } / { $total } 分钟

menu-stats = 统计
stats-period = 时段
stats-7 = 最近 7 天
stats-30 = 最近 30 天
stats-90 = 最近 90 天
stats-private = 统计数据只保存在这台电脑上。
stats-total = 此时段共拦截 { $count } 次
stats-top = 拦截最多
stats-by-block = 按拦截组
stats-allowance = 已用时长
stats-minutes = { $minutes } 分钟
stats-empty = 此时段内没有拦截任何内容。

notify-app-blocked = 应用已拦截
notify-app-blocked-body = “{ $block }”阻止了 { $app }。
notify-app-closed = 应用已关闭
notify-app-closed-body = “{ $block }”关闭了 { $app }。
notify-window-closed = 窗口已关闭
notify-window-closed-body = “{ $block }”关闭了“{ $title }”。
notify-allowance-low = 还剩 5 分钟
notify-allowance-low-body = “{ $block }”的每日时长即将用完。
notify-allowance-used = 时间到了
notify-allowance-used-body = “{ $block }”的每日时长已用完，明天之前将被拦截。
