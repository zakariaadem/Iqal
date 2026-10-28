# Iqal — English

app-name = Iqal Blocker
app-description = Website and app blocker
menu = Menu
menu-import = Import blocks…
menu-export-all = Export all blocks…
menu-activity = Recent activity
menu-settings = Settings
menu-about = About Iqal

action-new-block = New block
action-remove = Remove
action-add = Add
action-cancel = Cancel
action-delete = Delete
action-create = Create
action-export-block = Export this block
action-delete-block = Delete this block
action-import-text = Import from a text file…
action-add-installed-app = Add an installed app…
action-add-program-file = Add a program file…
action-add-window-title = Add a window title…
action-add-times = Add these times
action-cancel-unlock = Cancel the unlock
action-learn-location = I am at this place now
action-unlock = Unlock…
action-remove-lock = Remove the lock
action-change-lock = Change the lock…
action-lock = Lock
action-choose-on-map = Choose on the map…
action-use-area = Use this area

list-empty = Nothing here yet.

state-off = Off
state-active = Active
state-scheduled = Scheduled
state-locked = Locked

error-service-unreachable = The Iqal service is not running
error-unexpected = The Iqal service gave an unexpected answer.
error-locked = This change { $reason }. A locked block can only be made stricter.
error-wrong-answer = Wrong answer
error-cannot-unlock-early = This lock cannot be opened early.
error-not-admin = Only administrators can change Iqal settings.

reason-disables = would turn it off
reason-fewer-users = would apply it to fewer people
reason-web-mode = would change how websites are blocked
reason-removes-websites = would remove blocked websites
reason-adds-allowed-websites = would allow more websites
reason-safesearch-off = would turn SafeSearch off
reason-app-mode = would change how apps are blocked
reason-removes-apps = would remove blocked apps
reason-adds-allowed-apps = would allow more apps
reason-adds-schedule = would limit it to certain times
reason-shortens-schedule = would shorten its times
reason-adds-allowance = would add a time allowance
reason-increases-allowance = would increase the time allowance
reason-removes-lock = would remove the lock
reason-shortens-delay = would shorten the unlock delay
reason-weakens-lock = would weaken the lock
reason-pending-unlock = would change a pending unlock
reason-deletes = would delete it
reason-hardened-off = would turn hardened mode off

empty-title = No blocks yet
empty-detail = A block is a group of websites and apps to block, with its own times and lock. Create one with “New block”.

block-summary = Website rules: { $web } · App rules: { $apps }
rules-count = { $count ->
    [one] { $count } rule
   *[other] { $count } rules
}

locked-banner = This block is locked. You can add restrictions, but not remove them.
tab-websites = Websites
tab-apps = Apps
tab-users = People
tab-schedule = Times
tab-lock = Lock
block-enabled = Block on or off
block-switch-on = Block is on
block-switch-off = Block is off
confirm-delete = Delete “{ $name }”?
confirm-delete-detail = Its websites, apps, times and lock will be removed.
rule-invalid = “{ $rule }” cannot be used: { $problem }

web-blocked = Blocked websites
web-placeholder = example.com, example.com/page, contains:word, search:words
web-syntax = example.com blocks the site and all its subdomains. example.com/videos blocks only those pages. contains:casino blocks any address containing the word. search:free movies blocks searches containing the words.
web-allowed = Allowed websites
web-exceptions = Exceptions (always allowed)
web-safesearch = Force SafeSearch (Google, Bing, DuckDuckGo, Yandex)

import-done = Import finished
import-result = { $added } rules added, { $skipped } lines skipped.
import-added = Added: { $name }
import-replaced = Updated: { $name }
import-rejected = Not imported: { $name } ({ $why })

apps-blocked = Blocked apps
apps-exceptions = Exceptions (always allowed)
apps-allowed = Allowed apps
apps-placeholder = steam, /opt/games/, title:YouTube, cmd:minecraft
apps-placeholder-windows = steam, C:\Games\, title:YouTube, cmd:minecraft
apps-syntax = A name blocks the program with that name. A folder ending in / blocks every program in it. title:Text closes windows whose title contains the text. cmd:Text blocks programs whose command line contains it. Renamed copies are caught with hash: rules.
apps-syntax-windows = A name blocks the program with that name. A folder ending in \ blocks every program in it. title:Text closes windows whose title contains the text. cmd:Text blocks programs whose command line contains it. Renamed copies are caught with hash: rules.
window-title-intro = Iqal closes any window whose title contains these words, whatever program it belongs to. This suits things without a program of their own, such as a game inside a launcher. For websites, use the Websites tab instead: a browser window would be closed with all its tabs.
window-title-words = Words in the window title
window-title-open = Or choose a window that is open now
window-title-empty = Type some words of the title, or choose a window.

users-applies-to = Who this block applies to
users-all = Everyone
users-all-except = Everyone except…
users-only = Only…
users-except-these = Not these people
users-only-these = Only these people
users-note = Administrators of this computer are blocked too, unless excluded here. The system itself is never blocked.

day-mon = Mon
day-tue = Tue
day-wed = Wed
day-thu = Thu
day-fri = Fri
day-sat = Sat
day-sun = Sun
time-from = from
time-to = to
time-at = at

schedule-limited = Only at certain times
schedule-limited-detail = The block is in force only during the times below.
schedule-always = The block is always in force.
schedule-times = Times
schedule-date-placeholder = One-off: 2026-10-20 08:00 2026-10-22 18:00
schedule-date-invalid = Write a one-off period as: start date, start time, end date, end time (2026-10-20 08:00 2026-10-22 18:00).
schedule-add-weekly = Add weekly times
schedule-no-days = Choose at least one day.
allowance = Daily time allowance
allowance-detail = Each blocked website and app may be used for this many minutes a day; after that it is blocked until tomorrow.
allowance-minutes = Minutes per day

area-circle = within { $radius } m of { $lat }, { $lon }
area-polygon = inside an area with { $count } corners
lock-desc-until = Locked until { $time }
lock-desc-schedule = Locked during { $times }
lock-desc-password = Locked with a password
lock-desc-random = Unlocked by typing { $length } random characters
lock-desc-restart = Locked until the computer restarts
lock-desc-location = Locked while this computer is { $area }
lock-desc-delay = unlocks { $minutes } minutes after the answer

lock-none = Not locked
lock-none-detail = Without a lock, this block can be changed or turned off at any time. With a lock, it can only be made stricter until the lock ends.
lock-active = Locked
lock-inactive = Lock set, not in force right now
lock-pending = The lock opens at { $time }.
location-learn-detail = Iqal does not know this place yet. When this computer is there, press the button. Until then, the block stays locked wherever you are.
location-learned = Place remembered
location-learned-detail = Iqal now recognises this place.
location-none-chosen = No area chosen yet.

lock-kind-duration = For a while
lock-kind-until = Until a date
lock-kind-password = With a password
lock-kind-random = By typing random text
lock-kind-restart = Until the computer restarts
lock-kind-schedule = During certain times
lock-kind-location = While at a place
lock-dialog-title = Lock this block
unit-days = days
unit-hours = hours
unit-minutes = minutes
unit-metres = metres
password = Password
password-repeat = Repeat the password
password-show = Show password
password-generate = Make a password for someone else to keep
password-generate-detail = When you press Lock, Iqal creates a password and shows it once. Give it to someone you trust.
password-mismatch = The passwords are empty or do not match.
password-generated = The lock password
password-generated-detail = { $password }

    Give it to someone you trust. It will not be shown again.
random-length = Characters to type
lock-restart-detail = The block stays locked until this computer is restarted.
lock-location-detail = The block stays locked while this computer is inside the area, and also whenever Iqal cannot tell where it is.
unlock-delay = Wait after unlocking
unlock-delay-detail = After the right password or text is entered, the block stays locked for this many more minutes. This gives a sudden urge time to pass.
lock-duration-zero = Choose a length of time.
lock-until-past = That time is in the past.
lock-confirm = Lock this block?
lock-confirm-detail = { $lock }. Until then, nothing in this block can be removed, loosened or turned off.

unlock-not-possible = This lock cannot be opened early
unlock-not-possible-detail = It opens by itself when its time is over, or when you leave the place.
unlock-title = Unlock
unlock-type-text = Type this text exactly (pasting is not possible):
unlock-enter-password = Enter the lock password:
unlocked = Unlocked
unlocked-detail = The block is off now. You can change it or turn it on again.
unlock-pending = Unlock started

map-title = Choose the area
map-help = Click the map to place the centre of the area, then set its size. Drag to move the map and scroll to zoom.
map-radius = Radius

new-block-name = Name of the block
new-block-placeholder = For example: Social media
export-done = Exported

activity-empty = Nothing has been blocked yet.
event-app-start = App stopped
event-app-killed = App closed
event-window-closed = Window closed
event-site-blocked = Website blocked
event-page-blocked = Page blocked

settings-language = Language
settings-language-system = Same as the system
settings-theme = Appearance
settings-theme-system = Same as Windows
settings-theme-light = Light
settings-theme-dark = Dark
settings-language-restart = Restart Iqal to use the new language.
settings-hardened = Hardened mode
settings-hardened-detail = While any block is locked, the people it applies to cannot use sudo or administrator password prompts, so they cannot stop Iqal. Cannot be turned off while something is locked.
settings-status = Status
status-service = Iqal service { $version }
status-time = Time: { $source }
status-location = Location: { $state }
status-extensions = Browser extensions: { $list }
status-no-extensions = none connected
status-not-admin = You can view the blocks but not change them (administrators only).

event-allowance-low = Allowance almost used
event-allowance-used = Allowance used up

web-lists = Pre-made lists
web-lists-detail = Ready-made collections of websites. Adult content, gambling and social media are updated daily from a large community list.
list-social = Social media
list-adult = Adult content
list-gambling = Gambling
list-search = Search engines
list-news = News
list-video = Video and streaming
list-games = Games
list-shopping = Shopping
list-count = { $count ->
    [one] { $count } website
   *[other] { $count } websites
}
allowance-used-today = Used today: { $used } of { $total } minutes

menu-stats = Statistics
stats-7 = Last 7 days
stats-30 = Last 30 days
stats-90 = Last 90 days
stats-private = Statistics stay on this computer.
stats-top = Most blocked
stats-by-block = By block
stats-allowance = Allowance time used
stats-minutes = { $minutes } min
stats-empty = Nothing has been blocked in this period.

notify-app-blocked = App blocked
notify-app-blocked-body = { $app } was stopped by “{ $block }”.
notify-app-closed = App closed
notify-app-closed-body = { $app } was closed by “{ $block }”.
notify-window-closed = Window closed
notify-window-closed-body = “{ $title }” was closed by “{ $block }”.
notify-allowance-low = 5 minutes left
notify-allowance-low-body = The daily time for { $item } (“{ $block }”) is almost used up.
notify-allowance-used = Time is up
notify-allowance-used-body = The daily time for { $item } (“{ $block }”) is used up. It is blocked until tomorrow.

# Saving
unsaved-changes = Unsaved changes
action-save = Save
action-discard = Discard
confirm-unsaved = Save the changes to “{ $name }”?
confirm-unsaved-detail = If you don’t save, your changes will be lost.
save-has-errors = Fix these lines before saving
line-invalid = Line { $line }: “{ $rule }” cannot be used: { $problem }
lines-invalid-more = ({ $count } more lines need fixing)
list-empty-hint = One per line, for example: { $examples }
lock-saves-changes = Your other unsaved changes to this block will be saved too.

# Statistics window
stats-today = Today
stats-chart = Blocks per day
stats-card-total = Times blocked
stats-card-websites = Websites stopped
stats-card-apps = Apps stopped
stats-card-busiest = Busiest day
stats-card-top = Most blocked
stats-before = { $days ->
    [1] Yesterday: { $count }
   *[other] Previous { $days } days: { $count }
}
stats-blocks = { $count ->
    [one] { $count } block
   *[other] { $count } blocks
}
stats-distinct-sites = { $count ->
    [one] { $count } site
   *[other] { $count } different sites
}
stats-distinct-apps = { $count ->
    [one] { $count } app
   *[other] { $count } different apps
}
stats-show-all = Show all ({ $count })
stats-show-fewer = Show fewer
stats-allowance-of = { $used } of { $total } min

# Rule lists
web-mode-block = Block list
web-mode-allow-only = Allow list
apps-mode-block = Block list
apps-mode-allow-only = Allow list
add-websites = Add websites
add-apps = Add apps
add-exceptions = Add exceptions
action-remove-selected = Remove selected
search = Search
error-service-unreachable-detail = Install or start it, then try again.
error-locked-title = The block is locked
error-wrong-password = Wrong password
error-wrong-text = The text does not match
error-no-wifi = No Wi-Fi networks were found. Turn Wi-Fi on and try again.
location-learn-failed = Could not remember the place
location-here-question = Is this computer at this place now?
location-here-yes = Yes, it is there now
location-here-no = No
location-here-detail = If yes, Iqal remembers the Wi-Fi networks around it and from then on knows by itself when the computer arrives and leaves. If no, press "I am at this place now" on the Lock tab once you are there.
location-here-unanswered = Choose whether this computer is at the place now.
lock-confirm-timer = When the time is up, the block turns off too.
schedule-explain = Times decide when this block blocks. A lock decides when the block cannot be changed or turned off. For example: block social media on weekdays from 9:00 to 17:00 here, and add a password lock so that the times cannot be loosened. A lock "For a while" or "Until a date" turns the block off when it ends.

# Windows: a place lock by name (no map)
location-name = Name of the place
location-name-placeholder = For example: Home
area-named = at “{ $name }”
action-close = Close

lock-location-detail-named = The block stays locked while this computer is at this place, and also whenever Iqal cannot tell where it is.
time-source-network = network time
time-source-system = synchronised system time
time-source-counted = counted from the last trusted time (offline)
tray-open = Open Iqal

setup-window-title = Iqal Blocker setup
setup-install-title = Install Iqal Blocker { $version }?
setup-install-detail = Iqal will be installed in { $folder }. It runs in the background as a Windows service and blocks the websites and apps you choose.
setup-update-title = Update Iqal Blocker to { $version }?
setup-update-detail = Your blocks are kept.
action-install = Install
action-update = Update
setup-installed = Iqal Blocker is installed
setup-installed-detail = Open Iqal Blocker from the Start menu to make your first block.
setup-failed = Iqal could not be installed
setup-locked = Iqal cannot be updated now
setup-locked-detail = A block is locked. Iqal can be updated when its lock has ended.
uninstall-title = Remove Iqal Blocker?
uninstall-detail = The service, its browser settings and the programs are removed. Your blocks stay in { $folder }, in case you install Iqal again.
uninstall-action = Remove
uninstall-done = Iqal Blocker was removed
uninstall-failed = Iqal could not be removed
uninstall-locked = Iqal cannot be removed now
uninstall-locked-detail = A block is locked. Iqal can be removed when all locks have ended.
setup-updated = Iqal is updated
web-safesearch-detail = While this is on, other search engines and sites known for adult content (Reddit, X, Instagram, Tumblr, Bluesky, Imgur…) are blocked.
allowance-used-today-items = Used today:
allowance-used-item = { $item } { $used } of { $total } min
allowance-unused = Nothing used today yet.
unsaved-locked-weaker = This block is locked, and this change { $reason }.
schedule-old = Old time schedule
schedule-old-detail = This block is only in force at certain times, set in an earlier version of Iqal. Times are now set with a lock (“During certain times”).
schedule-old-remove = Make it always in force
lock-schedule-detail = During these times the block is on and locked. The rest of the time it is off and can be changed.
settings-theme-desktop = Same as the desktop
action-learn-more = Also lock here
location-learn-more-detail = Press this to lock the block where the computer is now as well. Pressing it again at the same place helps Iqal recognise the place.
lock-opens-until = This lock opens by itself when its time is over.
lock-opens-schedule = This lock opens by itself when its times are over.
lock-opens-restart = This lock opens by itself when the computer restarts.
lock-opens-location = This lock opens by itself when the computer leaves the place.
