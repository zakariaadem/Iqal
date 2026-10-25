# Iqal — 简体中文

app-name = Iqal Blocker
app-description = 网站和应用拦截器
menu = 菜单
menu-import = 导入拦截组…
menu-export-all = 导出全部拦截组…
menu-activity = 最近活动
menu-settings = 设置
menu-about = 关于 Iqal

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
action-add-window-title = 添加窗口标题…
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

error-service-unreachable = Iqal 服务未运行
error-unexpected = Iqal 服务返回了意外的结果。
error-locked = 这项更改{ $reason }。已上锁的拦截组只能变得更严格。
error-wrong-answer = 回答错误
error-cannot-unlock-early = 此锁不能提前打开。
error-not-admin = 只有管理员可以更改 Iqal 的设置。

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
block-switch-on = 拦截组已开启
block-switch-off = 拦截组已关闭
confirm-delete = 删除“{ $name }”？
confirm-delete-detail = 它的网站、应用、时间和锁都会被移除。
rule-invalid = 无法使用“{ $rule }”：{ $problem }

web-blocked = 被拦截的网站
web-placeholder = example.com, example.com/page, contains:词, search:词语
web-syntax = example.com 拦截该网站及其所有子域名。example.com/videos 只拦截这些页面。contains:casino 拦截包含该词的任何地址。search:免费电影 拦截包含这些词的搜索。
web-allowed = 允许的网站
web-exceptions = 例外（始终允许）
web-safesearch = 强制安全搜索（Google、Bing、DuckDuckGo、Yandex）

import-done = 导入完成
import-result = 已添加 { $added } 条规则，跳过 { $skipped } 行。
import-added = 已添加：{ $name }
import-replaced = 已更新：{ $name }
import-rejected = 未导入：{ $name }（{ $why }）

apps-blocked = 被拦截的应用
apps-exceptions = 例外（始终允许）
apps-allowed = 允许的应用
apps-placeholder = steam, /opt/games/, title:YouTube, cmd:minecraft
apps-placeholder-windows = steam, C:\Games\, title:YouTube, cmd:minecraft
apps-syntax = 名称会拦截同名程序。以 / 结尾的文件夹会拦截其中的所有程序。title:文字 会关闭标题包含该文字的窗口。cmd:文字 会拦截命令行包含该文字的程序。改名后的副本可用 hash: 规则识别。
apps-syntax-windows = 名称会拦截同名程序。以 \ 结尾的文件夹会拦截其中的所有程序。title:文字 会关闭标题包含该文字的窗口。cmd:文字 会拦截命令行包含该文字的程序。改名后的副本可用 hash: 规则识别。
window-title-intro = Iqal 会关闭标题中包含这些词的任何窗口，无论它属于哪个程序。适用于没有独立程序的内容，例如启动器中的游戏。网站请改用“网站”选项卡：浏览器窗口会连同所有标签页一起被关闭。
window-title-words = 窗口标题中的词
window-title-open = 或选择一个当前打开的窗口
window-title-empty = 请输入标题中的一些词，或选择一个窗口。

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
allowance-detail = 每个被拦截的网站和应用每天可以使用这么多分钟；之后会被拦截到明天。
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
location-learn-detail = Iqal 还不认识这个地方。当这台电脑在那里时，按下按钮，让 Iqal 记住周围的 Wi-Fi 网络。在此之前，无论你在哪里，拦截组都保持上锁。
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
password-show = 显示密码
password-generate = 生成一个由别人保管的密码
password-generate-detail = 当你按下“上锁”时，Iqal 会生成一个密码并只显示一次。请把它交给你信任的人。
password-mismatch = 密码为空或两次不一致。
password-generated = 锁的密码
password-generated-detail = { $password }

    请交给你信任的人。它不会再次显示。
random-length = 需要输入的字符数
lock-restart-detail = 在这台电脑重启之前，拦截组会一直保持锁定。
lock-location-detail = 当这台电脑在区域内时，以及每当 Iqal 无法判断它在哪里时，拦截组都会保持锁定。
unlock-delay = 解锁后等待
unlock-delay-detail = 输入正确的密码或文字后，拦截组还会再锁这么多分钟。这样可以让一时的冲动过去。
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
unlocked-detail = 拦截组现已关闭。你可以更改它或重新开启。
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
settings-theme = 外观
settings-theme-system = 与 Windows 相同
settings-theme-light = 浅色
settings-theme-dark = 深色
settings-language-restart = 重启 Iqal 以使用新语言。
settings-hardened = 强化模式
settings-hardened-detail = 只要有拦截组处于锁定状态，受其约束的人就无法使用 sudo 或管理员密码提示，因此无法停止 Iqal。紧急访问：重启进入恢复模式。有内容被锁定时无法关闭。
settings-status = 状态
status-service = Iqal 服务 { $version }
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
stats-7 = 最近 7 天
stats-30 = 最近 30 天
stats-90 = 最近 90 天
stats-private = 统计数据只保存在这台电脑上。
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
notify-allowance-low-body = { $item }（“{ $block }”）的每日时间快用完了。
notify-allowance-used = 时间到了
notify-allowance-used-body = { $item }（“{ $block }”）的每日时间已用完。明天之前都会被拦截。

# Saving
unsaved-changes = 有未保存的更改
action-save = 保存
action-discard = 放弃更改
confirm-unsaved = 保存对“{ $name }”的更改吗？
confirm-unsaved-detail = 如果不保存，您的更改将会丢失。
save-has-errors = 请先修正这些行再保存
line-invalid = 第 { $line } 行：“{ $rule }”无法使用：{ $problem }
lines-invalid-more = （还有 { $count } 行需要修正）
list-empty-hint = 每行一项，例如：{ $examples }
lock-saves-changes = 此拦截组中其他未保存的更改也会一并保存。

# Statistics window
stats-today = 今天
stats-chart = 每日拦截次数
stats-card-total = 拦截次数
stats-card-websites = 已拦截网站
stats-card-apps = 已拦截应用
stats-card-busiest = 拦截最多的一天
stats-card-top = 拦截最多
stats-before = { $days ->
    [1] 昨天：{ $count }
   *[other] 之前 { $days } 天：{ $count }
}
stats-blocks = { $count } 次拦截
stats-distinct-sites = { $count } 个不同网站
stats-distinct-apps = { $count } 个不同应用
stats-show-all = 全部显示（{ $count }）
stats-show-fewer = 收起
stats-allowance-of = { $used } / { $total } 分钟

# Rule lists
web-mode-block = 拦截列表
web-mode-allow-only = 允许列表
apps-mode-block = 拦截列表
apps-mode-allow-only = 允许列表
add-websites = 添加网站
add-apps = 添加应用
add-exceptions = 添加例外
action-remove-selected = 移除所选
search = 搜索
error-service-unreachable-detail = 请安装或启动它，然后重试。
error-locked-title = 拦截组已上锁
error-wrong-password = 密码错误
error-wrong-text = 文字不一致
error-no-wifi = 没有找到 Wi-Fi 网络。请打开 Wi-Fi 后重试。
location-learn-failed = 无法记住这个地方
location-known = Iqal 通过周围的 Wi-Fi 网络认出这个地方，并能自己知道这台电脑何时在那里。
location-here-question = 这台电脑现在在这个地方吗？
location-here-yes = 是，现在就在那里
location-here-no = 否
location-here-detail = 如果是，Iqal 会记住周围的 Wi-Fi 网络，此后能自己知道电脑何时到达和离开。如果否，到了那里后在“锁”标签页中按“我现在就在这里”。
location-here-unanswered = 请选择这台电脑现在是否在该地方。
lock-confirm-timer = 时间到了以后，拦截组也会关闭。
schedule-explain = 时间决定这个拦截组什么时候拦截。锁决定拦截组什么时候不能被更改或关闭。例如：在这里设置工作日 9:00 到 17:00 拦截社交媒体，再加一个密码锁，让这些时间无法被放宽。“一段时间”或“到某个日期”的锁结束时会关闭拦截组。

# Windows: a place lock by name (no map)
location-name = 地点名称
location-name-placeholder = 例如：家
area-named = 在“{ $name }”
action-close = 关闭

lock-location-detail-named = 只要这台电脑在这个地方，以及 Iqal 无法判断它在哪里时，拦截组都会保持锁定。
time-source-network = 网络时间
time-source-system = 已同步的系统时间
time-source-counted = 从上次可信时间起计算（离线）
tray-open = 打开 Iqal

setup-window-title = Iqal Blocker 安装
setup-install-title = 安装 Iqal Blocker { $version }？
setup-install-detail = Iqal 将安装到 { $folder }。它作为 Windows 服务在后台运行，拦截你选择的网站和应用。
setup-update-title = 将 Iqal Blocker 更新到 { $version }？
setup-update-detail = 你的拦截组会保留。
action-install = 安装
action-update = 更新
setup-installed = Iqal Blocker 已安装
setup-installed-detail = 从“开始”菜单打开 Iqal Blocker，创建你的第一个拦截组。
setup-failed = 无法安装 Iqal
setup-locked = 现在无法更新 Iqal
setup-locked-detail = 有拦截组已上锁。锁结束后即可更新 Iqal。
uninstall-title = 移除 Iqal Blocker？
uninstall-detail = 服务、它的浏览器设置和程序将被移除。你的拦截组会保留在 { $folder } 中，以便再次安装 Iqal。
uninstall-action = 移除
uninstall-done = Iqal Blocker 已移除
uninstall-failed = 无法移除 Iqal
uninstall-locked = 现在无法移除 Iqal
uninstall-locked-detail = 有拦截组已上锁。所有锁结束后即可移除 Iqal。
setup-updated = Iqal 已更新
web-safesearch-detail = 开启时，其他搜索引擎以及以成人内容闻名的网站会被拦截（Reddit、X、Instagram、Tumblr、Bluesky、Imgur…）。
allowance-used-today-items = 今天已用：
allowance-used-item = { $item } { $used }/{ $total } 分钟
allowance-unused = 今天还没有使用。
unsaved-locked-weaker = 此拦截组已上锁，而这个更改{ $reason }。
schedule-old = 旧的时间表
schedule-old-detail = 此拦截组只在某些时间生效，这是在 Iqal 的旧版本中设置的。现在时间通过锁来设置（“在特定时间”）。
schedule-old-remove = 改为始终生效
lock-schedule-detail = 在这些时间内，拦截组开启并上锁。其余时间它是关闭的，可以更改。
