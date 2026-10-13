# Iqal — English

app-name = Iqal
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

error-title = Something went wrong
error-service-unreachable = The Iqal service is not running. Install or start it, then try again.
error-unexpected = The Iqal service gave an unexpected answer.
error-locked = This block is locked, and this change { $reason }. Locked blocks can only be made stricter.
error-wrong-answer = That is not correct.
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

block-summary = { $web } website rules · { $apps } app rules
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
confirm-delete = Delete “{ $name }”?
confirm-delete-detail = Its websites, apps, times and lock will be removed.
rule-invalid = “{ $rule }” cannot be used: { $problem }

web-blocked = Blocked websites
web-placeholder = example.com, example.com/page, contains:word, search:words
web-syntax = example.com blocks the site and all its subdomains. example.com/videos blocks only those pages. contains:casino blocks any address containing the word. search:free movies blocks searches containing the words.
web-allowed = Allowed websites
web-exceptions = Exceptions (always allowed)
web-safesearch = Force SafeSearch (Google, Bing, DuckDuckGo, YouTube)
web-partial = Partial blocks
web-partial-detail = Block only the distracting part of a site and keep the rest.
partial-youtube-shorts = YouTube Shorts
partial-youtube-comments = YouTube comments
partial-youtube-recommendations = YouTube recommendations and home feed
partial-reddit-home = Reddit front page, r/all and r/popular
partial-facebook-feed = Facebook news feed, Watch and Reels
partial-instagram-reels = Instagram Reels
partial-instagram-explore = Instagram Explore
partial-x-trends = X (Twitter) Explore and trends

import-done = Import finished
import-result = { $added } rules added, { $skipped } lines skipped.
import-added = Added: { $name }
import-replaced = Updated: { $name }
import-rejected = Not imported: { $name } ({ $why })

apps-blocked = Blocked apps
apps-exceptions = Exceptions (always allowed)
apps-allowed = Allowed apps
apps-placeholder = steam, /opt/games/, title:YouTube, cmd:minecraft
apps-syntax = A name blocks the program with that name. A folder ending in / blocks every program in it. title:Text closes windows whose title contains the text. cmd:Text blocks programs whose command line contains it. Renamed copies are caught with hash: rules.

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
allowance-detail = Allows the blocked websites and apps for a limited time each day; after that they are blocked.
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
location-learn-detail = If you are at this place now, let Iqal remember its Wi-Fi networks. This is how Iqal knows you are there, even without internet.
location-learned = Place remembered
location-learned-detail = { $count } Wi-Fi networks are now remembered for this place.
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
password-generate = Make a password for someone else to keep
password-generate-detail = Iqal creates a password and shows it once. Give it to someone you trust.
password-mismatch = The passwords are empty or do not match.
password-generated = The lock password
password-generated-detail = { $password }

    Give it to someone you trust. It will not be shown again.
random-length = Characters to type
lock-restart-detail = The block stays locked until this computer is restarted.
lock-location-detail = The block stays locked while this computer is inside the area, and also whenever Iqal cannot tell where it is.
unlock-delay = Wait after unlocking
unlock-delay-detail = With a wait, a correct answer opens the lock only after this time. It helps with sudden urges.
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
unlocked-detail = The block can be changed again.
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
settings-language-restart = Restart Iqal to use the new language.
settings-hardened = Hardened mode
settings-hardened-detail = While any block is locked, the people it applies to cannot use sudo or administrator password prompts, so they cannot stop Iqal. Emergency access: restart into recovery mode. Cannot be turned off while something is locked.
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
notify-allowance-low-body = The daily time for “{ $block }” is almost used up.
notify-allowance-used = Time is up
notify-allowance-used-body = The daily time for “{ $block }” is used up. It is blocked until tomorrow.

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
