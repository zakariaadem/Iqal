# Iqal — اردو

app-name = Iqal Blocker
app-description = ویب سائٹ اور ایپ بلاکر
menu = مینو
menu-import = بلاک درآمد کریں…
menu-export-all = تمام بلاک برآمد کریں…
menu-activity = حالیہ سرگرمی
menu-settings = ترتیبات
menu-about = Iqal کے بارے میں

action-new-block = نیا بلاک
action-remove = ہٹائیں
action-add = شامل کریں
action-cancel = منسوخ کریں
action-delete = حذف کریں
action-create = بنائیں
action-export-block = یہ بلاک برآمد کریں
action-delete-block = یہ بلاک حذف کریں
action-import-text = ٹیکسٹ فائل سے درآمد کریں…
action-add-installed-app = انسٹال شدہ ایپ شامل کریں…
action-add-program-file = پروگرام فائل شامل کریں…
action-add-window-title = کھڑکی کا عنوان شامل کریں…
action-add-times = یہ اوقات شامل کریں
action-cancel-unlock = تالا کھولنا منسوخ کریں
action-learn-location = میں اس وقت اسی جگہ پر ہوں
action-unlock = تالا کھولیں…
action-remove-lock = تالا ہٹائیں
action-change-lock = تالا بدلیں…
action-lock = تالا لگائیں
action-choose-on-map = نقشے پر منتخب کریں…
action-use-area = یہ علاقہ استعمال کریں

list-empty = ابھی کچھ نہیں ہے۔

state-off = بند
state-active = فعال
state-scheduled = طے شدہ
state-locked = مقفل

error-service-unreachable = عِقال سروس نہیں چل رہی
error-unexpected = Iqal سروس نے غیر متوقع جواب دیا۔
error-locked = یہ تبدیلی { $reason }۔ مقفل بلاک کو صرف مزید سخت کیا جا سکتا ہے۔
error-wrong-answer = غلط جواب
error-cannot-unlock-early = یہ تالا وقت سے پہلے نہیں کھولا جا سکتا۔
error-not-admin = صرف منتظمین Iqal کی ترتیبات بدل سکتے ہیں۔

reason-disables = اسے بند کر دے گی
reason-fewer-users = اسے کم لوگوں پر لاگو کرے گی
reason-web-mode = ویب سائٹس بلاک کرنے کا طریقہ بدل دے گی
reason-removes-websites = بلاک شدہ ویب سائٹس ہٹا دے گی
reason-adds-allowed-websites = مزید ویب سائٹس کی اجازت دے گی
reason-safesearch-off = SafeSearch بند کر دے گی
reason-app-mode = ایپس بلاک کرنے کا طریقہ بدل دے گی
reason-removes-apps = بلاک شدہ ایپس ہٹا دے گی
reason-adds-allowed-apps = مزید ایپس کی اجازت دے گی
reason-adds-schedule = اسے مخصوص اوقات تک محدود کر دے گی
reason-shortens-schedule = اس کے اوقات کم کر دے گی
reason-adds-allowance = وقت کی اجازت شامل کر دے گی
reason-increases-allowance = وقت کی اجازت بڑھا دے گی
reason-removes-lock = تالا ہٹا دے گی
reason-shortens-delay = تالا کھلنے کا انتظار کم کر دے گی
reason-weakens-lock = تالا کمزور کر دے گی
reason-pending-unlock = زیرِ التوا تالا کھلنے کو بدل دے گی
reason-deletes = اسے حذف کر دے گی
reason-hardened-off = سخت موڈ بند کر دے گی

empty-title = ابھی کوئی بلاک نہیں
empty-detail = بلاک ویب سائٹس اور ایپس کا ایک گروپ ہے جنہیں بلاک کرنا ہے، اس کے اپنے اوقات اور تالے کے ساتھ۔ «نیا بلاک» سے ایک بنائیں۔

block-summary = ویب سائٹ کے اصول: { $web } · ایپ کے اصول: { $apps }
rules-count = { $count ->
    [one] { $count } اصول
   *[other] { $count } اصول
}

locked-banner = یہ بلاک مقفل ہے۔ آپ پابندیاں شامل کر سکتے ہیں، ہٹا نہیں سکتے۔
tab-websites = ویب سائٹس
tab-apps = ایپس
tab-users = لوگ
tab-schedule = اوقات
tab-lock = تالا
block-enabled = بلاک آن یا آف
block-switch-on = بلاک آن ہے
block-switch-off = بلاک آف ہے
confirm-delete = «{ $name }» حذف کریں؟
confirm-delete-detail = اس کی ویب سائٹس، ایپس، اوقات اور تالا ہٹا دیے جائیں گے۔
rule-invalid = «{ $rule }» استعمال نہیں ہو سکتا: { $problem }

web-blocked = بلاک شدہ ویب سائٹس
web-placeholder = ⁦example.com⁩ یا ⁦example.com/page⁩ یا ⁦contains:لفظ⁩ یا ⁦search:الفاظ⁩
web-syntax = ⁦example.com⁩ سائٹ اور اس کے تمام ذیلی ڈومین بلاک کرتا ہے۔ ⁦example.com/videos⁩ صرف وہ صفحات بلاک کرتا ہے۔ ⁦contains:casino⁩ ہر وہ پتا بلاک کرتا ہے جس میں یہ لفظ ہو۔ ⁦search:مفت فلمیں⁩ ان الفاظ والی تلاش بلاک کرتا ہے۔
web-allowed = اجازت یافتہ ویب سائٹس
web-exceptions = استثنا (ہمیشہ اجازت)
web-safesearch = محفوظ تلاش لازمی کریں (Google، Bing، DuckDuckGo، Yandex)

import-done = درآمد مکمل
import-result = { $added } اصول شامل ہوئے، { $skipped } سطریں چھوڑ دی گئیں۔
import-added = شامل: { $name }
import-replaced = تازہ کیا گیا: { $name }
import-rejected = درآمد نہیں ہوا: { $name } ({ $why })

apps-blocked = بلاک شدہ ایپس
apps-exceptions = استثنا (ہمیشہ اجازت)
apps-allowed = اجازت یافتہ ایپس
apps-placeholder = ⁦steam⁩ یا ⁦/opt/games/⁩ یا ⁦title:YouTube⁩ یا ⁦cmd:minecraft⁩
apps-placeholder-windows = ⁦steam⁩ یا ⁦C:\Games\⁩ یا ⁦title:YouTube⁩ یا ⁦cmd:minecraft⁩
apps-syntax = نام اسی نام والے پروگرام کو بلاک کرتا ہے۔ ⁦/⁩ پر ختم ہونے والا فولڈر اس کے اندر ہر پروگرام بلاک کرتا ہے۔ ⁦title:متن⁩ وہ ونڈوز بند کرتا ہے جن کے عنوان میں یہ متن ہو۔ ⁦cmd:متن⁩ وہ پروگرام بلاک کرتا ہے جن کی کمانڈ لائن میں یہ ہو۔ نام بدلی ہوئی نقلیں ⁦hash:⁩ اصولوں سے پکڑی جاتی ہیں۔
apps-syntax-windows = نام اسی نام والے پروگرام کو بلاک کرتا ہے۔ ⁦\⁩ پر ختم ہونے والا فولڈر اس کے اندر ہر پروگرام بلاک کرتا ہے۔ ⁦title:متن⁩ وہ ونڈوز بند کرتا ہے جن کے عنوان میں یہ متن ہو۔ ⁦cmd:متن⁩ وہ پروگرام بلاک کرتا ہے جن کی کمانڈ لائن میں یہ ہو۔ نام بدلی ہوئی نقلیں ⁦hash:⁩ اصولوں سے پکڑی جاتی ہیں۔
window-title-intro = Iqal ہر وہ کھڑکی بند کر دیتا ہے جس کے عنوان میں یہ الفاظ ہوں، چاہے وہ کسی بھی پروگرام کی ہو۔ یہ ان چیزوں کے لیے ہے جن کا اپنا پروگرام نہیں، جیسے لانچر کے اندر کوئی گیم۔ ویب سائٹس کے لیے ویب سائٹس والا ٹیب استعمال کریں: براؤزر کی کھڑکی اپنے تمام ٹیبز سمیت بند ہو جائے گی۔
window-title-words = کھڑکی کے عنوان کے الفاظ
window-title-open = یا ابھی کھلی ہوئی کوئی کھڑکی چنیں
window-title-empty = عنوان کے کچھ الفاظ لکھیں، یا کوئی کھڑکی چنیں۔

users-applies-to = یہ بلاک کن پر لاگو ہے
users-all = سب
users-all-except = سب، سوائے…
users-only = صرف…
users-except-these = ان لوگوں کے سوا
users-only-these = صرف یہ لوگ
users-note = اس کمپیوٹر کے منتظمین بھی بلاک ہوتے ہیں، جب تک یہاں انہیں مستثنیٰ نہ کیا جائے۔ سسٹم خود کبھی بلاک نہیں ہوتا۔

day-mon = پیر
day-tue = منگل
day-wed = بدھ
day-thu = جمعرات
day-fri = جمعہ
day-sat = ہفتہ
day-sun = اتوار
time-from = سے
time-to = تک
time-at = بجے

schedule-limited = صرف مخصوص اوقات میں
schedule-limited-detail = بلاک صرف نیچے دیے گئے اوقات میں نافذ ہوتا ہے۔
schedule-always = بلاک ہمیشہ نافذ رہتا ہے۔
schedule-times = اوقات
schedule-date-placeholder = ایک بار: ⁦2026-10-20 08:00 2026-10-22 18:00⁩
schedule-date-invalid = ایک بار کی مدت یوں لکھیں: شروع کی تاریخ، شروع کا وقت، ختم کی تاریخ، ختم کا وقت (⁦2026-10-20 08:00 2026-10-22 18:00⁩)۔
schedule-add-weekly = ہفتہ وار اوقات شامل کریں
schedule-no-days = کم از کم ایک دن منتخب کریں۔
allowance = روزانہ وقت کی اجازت
allowance-detail = ہر بلاک شدہ ویب سائٹ اور ایپ روزانہ اتنے منٹ استعمال ہو سکتی ہے؛ اس کے بعد کل تک بلاک رہتی ہے۔
allowance-minutes = روزانہ منٹ

area-circle = { $lat }، { $lon } سے { $radius } میٹر کے اندر
area-polygon = { $count } کونوں والے علاقے کے اندر
lock-desc-until = { $time } تک مقفل
lock-desc-schedule = { $times } کے دوران مقفل
lock-desc-password = پاس ورڈ سے مقفل
lock-desc-random = { $length } بے ترتیب حروف لکھنے پر کھلتا ہے
lock-desc-restart = کمپیوٹر دوبارہ شروع ہونے تک مقفل
lock-desc-location = جب تک یہ کمپیوٹر { $area } ہے، مقفل
lock-desc-delay = جواب کے { $minutes } منٹ بعد کھلتا ہے

lock-none = مقفل نہیں
lock-none-detail = تالے کے بغیر یہ بلاک کسی بھی وقت بدلا یا بند کیا جا سکتا ہے۔ تالے کے ساتھ اسے تالا ختم ہونے تک صرف سخت کیا جا سکتا ہے۔
lock-active = مقفل
lock-inactive = تالا لگا ہے، ابھی نافذ نہیں
lock-pending = تالا { $time } کو کھلے گا۔
location-learn-detail = عِقال ابھی اس جگہ کو نہیں جانتا۔ جب یہ کمپیوٹر وہاں ہو تو بٹن دبائیں۔ تب تک بلاک پر تالا لگا رہتا ہے، آپ جہاں بھی ہوں۔
location-learned = جگہ یاد کر لی گئی
location-learned-detail = عِقال اب اس جگہ کو پہچانتا ہے۔
location-none-chosen = ابھی کوئی علاقہ منتخب نہیں ہوا۔

lock-kind-duration = کچھ دیر کے لیے
lock-kind-until = کسی تاریخ تک
lock-kind-password = پاس ورڈ کے ساتھ
lock-kind-random = بے ترتیب متن لکھ کر
lock-kind-restart = کمپیوٹر دوبارہ شروع ہونے تک
lock-kind-schedule = مخصوص اوقات میں
lock-kind-location = جب تک میں کسی جگہ پر ہوں
lock-dialog-title = اس بلاک کو مقفل کریں
unit-days = دن
unit-hours = گھنٹے
unit-minutes = منٹ
unit-metres = میٹر
password = پاس ورڈ
password-repeat = پاس ورڈ دوبارہ لکھیں
password-show = پاس ورڈ دکھائیں
password-generate = کسی اور کے پاس رکھنے کے لیے پاس ورڈ بنائیں
password-generate-detail = جب آپ «تالا لگائیں» دبائیں گے تو عِقال ایک پاس ورڈ بنائے گا اور اسے ایک بار دکھائے گا۔ اسے کسی قابلِ اعتماد شخص کو دے دیں۔
password-mismatch = پاس ورڈ خالی ہیں یا آپس میں نہیں ملتے۔
password-generated = تالے کا پاس ورڈ
password-generated-detail = { $password }

    اسے کسی قابلِ اعتماد شخص کو دیں۔ یہ دوبارہ نہیں دکھایا جائے گا۔
random-length = لکھنے کے لیے حروف
lock-restart-detail = بلاک اس کمپیوٹر کے دوبارہ شروع ہونے تک مقفل رہتا ہے۔
lock-location-detail = بلاک تب تک مقفل رہتا ہے جب تک یہ کمپیوٹر علاقے کے اندر ہو، اور جب بھی Iqal یہ نہ جان سکے کہ یہ کہاں ہے۔
unlock-delay = تالا کھولنے کے بعد انتظار
unlock-delay-detail = صحیح پاس ورڈ یا متن لکھنے کے بعد بلاک اتنے منٹ مزید مقفل رہتا ہے۔ اس سے اچانک خواہش کو گزرنے کا وقت ملتا ہے۔
lock-duration-zero = وقت کی مدت منتخب کریں۔
lock-until-past = یہ وقت گزر چکا ہے۔
lock-confirm = اس بلاک کو مقفل کریں؟
lock-confirm-detail = { $lock }۔ تب تک اس بلاک سے کچھ بھی ہٹایا، نرم یا بند نہیں کیا جا سکتا۔

unlock-not-possible = یہ تالا وقت سے پہلے نہیں کھولا جا سکتا
unlock-not-possible-detail = یہ اپنا وقت ختم ہونے پر، یا آپ کے اس جگہ سے جانے پر، خود کھل جاتا ہے۔
unlock-title = تالا کھولیں
unlock-type-text = یہ متن بالکل اسی طرح لکھیں (چسپاں کرنا ممکن نہیں):
unlock-enter-password = تالے کا پاس ورڈ درج کریں:
unlocked = تالا کھل گیا
unlocked-detail = بلاک اب بند ہے۔ آپ اسے بدل سکتے ہیں یا دوبارہ آن کر سکتے ہیں۔
unlock-pending = تالا کھلنا شروع ہو گیا

map-title = علاقہ منتخب کریں
map-help = علاقے کا مرکز رکھنے کے لیے نقشے پر کلک کریں، پھر اس کا سائز طے کریں۔ نقشہ ہلانے کے لیے گھسیٹیں اور زوم کے لیے پہیہ استعمال کریں۔
map-radius = رداس

new-block-name = بلاک کا نام
new-block-placeholder = مثلاً: سوشل میڈیا
export-done = برآمد ہو گیا

activity-empty = ابھی تک کچھ بلاک نہیں ہوا۔
event-app-start = ایپ روکی گئی
event-app-killed = ایپ بند کی گئی
event-window-closed = ونڈو بند کی گئی
event-site-blocked = ویب سائٹ بلاک ہوئی
event-page-blocked = صفحہ بلاک ہوا

settings-language = زبان
settings-language-system = سسٹم کے مطابق
settings-theme = ظاہری شکل
settings-theme-system = Windows کے مطابق
settings-theme-light = ہلکا
settings-theme-dark = گہرا
settings-language-restart = نئی زبان استعمال کرنے کے لیے Iqal دوبارہ شروع کریں۔
settings-hardened = سخت موڈ
settings-hardened-detail = جب تک کوئی بلاک مقفل ہے، جن لوگوں پر یہ لاگو ہے وہ sudo یا منتظم پاس ورڈ کی درخواستیں استعمال نہیں کر سکتے، اس لیے Iqal کو روک نہیں سکتے۔ کچھ مقفل ہونے کے دوران بند نہیں ہو سکتا۔
settings-status = حالت
status-service = Iqal سروس { $version }
status-time = وقت: { $source }
status-location = مقام: { $state }
status-extensions = براؤزر ایکسٹینشنز: { $list }
status-no-extensions = کوئی منسلک نہیں
status-not-admin = آپ بلاک دیکھ سکتے ہیں مگر بدل نہیں سکتے (صرف منتظمین)۔

event-allowance-low = اجازت کا وقت تقریباً ختم
event-allowance-used = اجازت کا وقت ختم

web-lists = تیار فہرستیں
web-lists-detail = ویب سائٹس کے تیار مجموعے۔ بالغ مواد، جوئے اور سوشل میڈیا کی فہرستیں ہر روز ایک بڑی کمیونٹی فہرست سے تازہ کی جاتی ہیں۔
list-social = سوشل میڈیا
list-adult = بالغ مواد
list-gambling = جوا
list-search = سرچ انجن
list-news = خبریں
list-video = ویڈیو اور اسٹریمنگ
list-games = گیمز
list-shopping = خریداری
list-count = { $count ->
    [one] { $count } ویب سائٹ
   *[other] { $count } ویب سائٹس
}
allowance-used-today = آج استعمال: { $total } میں سے { $used } منٹ

menu-stats = اعداد و شمار
stats-7 = پچھلے 7 دن
stats-30 = پچھلے 30 دن
stats-90 = پچھلے 90 دن
stats-private = اعداد و شمار اسی کمپیوٹر پر رہتے ہیں۔
stats-top = سب سے زیادہ بلاک
stats-by-block = بلاک کے لحاظ سے
stats-allowance = استعمال شدہ اجازت کا وقت
stats-minutes = { $minutes } منٹ
stats-empty = اس مدت میں کچھ بلاک نہیں ہوا۔

notify-app-blocked = ایپ بلاک ہوئی
notify-app-blocked-body = «{ $block }» نے { $app } کو روک دیا۔
notify-app-closed = ایپ بند ہوئی
notify-app-closed-body = «{ $block }» نے { $app } کو بند کر دیا۔
notify-window-closed = ونڈو بند ہوئی
notify-window-closed-body = «{ $block }» نے «{ $title }» کو بند کر دیا۔
notify-allowance-low = 5 منٹ باقی
notify-allowance-low-body = { $item } («{ $block }») کا روزانہ وقت تقریباً ختم ہو گیا ہے۔
notify-allowance-used = وقت ختم
notify-allowance-used-body = { $item } («{ $block }») کا روزانہ وقت ختم ہو گیا۔ کل تک بلاک رہے گا۔

# Saving
unsaved-changes = غیر محفوظ تبدیلیاں
action-save = محفوظ کریں
action-discard = رد کریں
confirm-unsaved = کیا «{ $name }» کی تبدیلیاں محفوظ کریں؟
confirm-unsaved-detail = اگر آپ محفوظ نہیں کریں گے تو آپ کی تبدیلیاں ضائع ہو جائیں گی۔
save-has-errors = محفوظ کرنے سے پہلے یہ سطریں درست کریں
line-invalid = سطر { $line }: «{ $rule }» استعمال نہیں ہو سکتا: { $problem }
lines-invalid-more = (مزید { $count } سطریں درست کرنی ہیں)
list-empty-hint = ہر سطر میں ایک، مثلاً: { $examples }
lock-saves-changes = اس بلاک کی آپ کی دوسری غیر محفوظ تبدیلیاں بھی محفوظ ہو جائیں گی۔

# Statistics window
stats-today = آج
stats-chart = روزانہ بلاک
stats-card-total = بلاک کی تعداد
stats-card-websites = روکی گئی ویب سائٹس
stats-card-apps = روکی گئی ایپس
stats-card-busiest = سب سے مصروف دن
stats-card-top = سب سے زیادہ بلاک
stats-before = { $days ->
    [1] کل: { $count }
   *[other] پچھلے { $days } دن: { $count }
}
stats-blocks = { $count } بلاک
stats-distinct-sites = { $count ->
    [one] { $count } سائٹ
   *[other] { $count } مختلف سائٹس
}
stats-distinct-apps = { $count ->
    [one] { $count } ایپ
   *[other] { $count } مختلف ایپس
}
stats-show-all = سب دکھائیں ({ $count })
stats-show-fewer = کم دکھائیں
stats-allowance-of = { $total } میں سے { $used } منٹ

# Rule lists
web-mode-block = بلاک لسٹ
web-mode-allow-only = اجازت لسٹ
apps-mode-block = بلاک لسٹ
apps-mode-allow-only = اجازت لسٹ
add-websites = ویب سائٹس شامل کریں
add-apps = ایپس شامل کریں
add-exceptions = استثنا شامل کریں
action-remove-selected = منتخب ہٹائیں
search = تلاش کریں
error-service-unreachable-detail = اسے انسٹال یا شروع کریں، پھر دوبارہ کوشش کریں۔
error-locked-title = بلاک مقفل ہے
error-wrong-password = غلط پاس ورڈ
error-wrong-text = متن مطابقت نہیں رکھتا
error-no-wifi = کوئی Wi-Fi نیٹ ورک نہیں ملا۔ Wi-Fi آن کریں اور دوبارہ کوشش کریں۔
location-learn-failed = جگہ یاد نہیں رکھی جا سکی
location-here-question = کیا یہ کمپیوٹر اس وقت اس جگہ پر ہے؟
location-here-yes = ہاں، اس وقت وہیں ہے
location-here-no = نہیں
location-here-detail = اگر ہاں، تو عِقال آس پاس کے Wi-Fi نیٹ ورک یاد رکھے گا اور پھر خود جان لے گا کہ کمپیوٹر کب آتا اور جاتا ہے۔ اگر نہیں، تو وہاں پہنچ کر تالا والے ٹیب میں «میں اس وقت اسی جگہ پر ہوں» دبائیں۔
location-here-unanswered = منتخب کریں کہ آیا یہ کمپیوٹر اس وقت اس جگہ پر ہے۔
lock-confirm-timer = وقت ختم ہونے پر بلاک بھی بند ہو جائے گا۔
schedule-explain = اوقات طے کرتے ہیں کہ یہ بلاک کب روکتا ہے۔ تالا طے کرتا ہے کہ بلاک کب تبدیل یا بند نہیں کیا جا سکتا۔ مثال: یہاں سوشل میڈیا کو کام کے دنوں میں 9:00 سے 17:00 تک روکیں، اور پاس ورڈ والا تالا لگائیں تاکہ اوقات نرم نہ کیے جا سکیں۔ «کچھ دیر کے لیے» یا «کسی تاریخ تک» والا تالا ختم ہونے پر بلاک کو بند کر دیتا ہے۔

# Windows: a place lock by name (no map)
location-name = جگہ کا نام
location-name-placeholder = مثال کے طور پر: گھر
area-named = «{ $name }» میں
action-close = بند کریں

lock-location-detail-named = جب تک یہ کمپیوٹر اس جگہ پر ہے، اور جب بھی عِقال یہ نہ جان سکے کہ وہ کہاں ہے، بلاک مقفل رہتا ہے۔
time-source-network = نیٹ ورک کا وقت
time-source-system = ہم وقت شدہ سسٹم کا وقت
time-source-counted = آخری قابلِ اعتماد وقت سے گنا گیا (آف لائن)
tray-open = عِقال کھولیں

setup-window-title = Iqal Blocker کی تنصیب
setup-install-title = کیا Iqal Blocker { $version } انسٹال کریں؟
setup-install-detail = عِقال { $folder } میں انسٹال ہوگا۔ یہ پس منظر میں ونڈوز سروس کے طور پر چلتا ہے اور آپ کی چنی ہوئی ویب سائٹس اور ایپس کو بلاک کرتا ہے۔
setup-update-title = کیا Iqal Blocker کو { $version } پر اپ ڈیٹ کریں؟
setup-update-detail = آپ کے بلاک محفوظ رہتے ہیں۔
action-install = انسٹال کریں
action-update = اپ ڈیٹ کریں
setup-installed = Iqal Blocker انسٹال ہو گیا
setup-installed-detail = اپنا پہلا بلاک بنانے کے لیے اسٹارٹ مینو سے Iqal Blocker کھولیں۔
setup-failed = عِقال انسٹال نہیں ہو سکا
setup-locked = عِقال ابھی اپ ڈیٹ نہیں ہو سکتا
setup-locked-detail = ایک بلاک مقفل ہے۔ اس کا تالا ختم ہونے پر عِقال اپ ڈیٹ ہو سکتا ہے۔
uninstall-title = کیا Iqal Blocker ہٹا دیں؟
uninstall-detail = سروس، اس کی براؤزر ترتیبات اور پروگرام ہٹا دیے جاتے ہیں۔ آپ کے بلاک { $folder } میں رہتے ہیں، اگر آپ عِقال دوبارہ انسٹال کریں۔
uninstall-action = ہٹائیں
uninstall-done = Iqal Blocker ہٹا دیا گیا
uninstall-failed = عِقال ہٹایا نہیں جا سکا
uninstall-locked = عِقال ابھی ہٹایا نہیں جا سکتا
uninstall-locked-detail = ایک بلاک مقفل ہے۔ سب تالے ختم ہونے پر عِقال ہٹایا جا سکتا ہے۔
setup-updated = عِقال اپ ڈیٹ ہو گیا
web-safesearch-detail = جب یہ آن ہو تو دوسرے سرچ انجن اور بالغ مواد کے لیے مشہور سائٹیں بلاک رہتی ہیں (Reddit، X، Instagram، Tumblr، Bluesky، Imgur…)۔
allowance-used-today-items = آج استعمال ہوا:
allowance-used-item = { $item } { $total } میں سے { $used } منٹ
allowance-unused = آج ابھی کچھ استعمال نہیں ہوا۔
unsaved-locked-weaker = یہ بلاک مقفل ہے، اور یہ تبدیلی { $reason }۔
schedule-old = پرانا اوقات کا شیڈول
schedule-old-detail = یہ بلاک صرف کچھ اوقات میں لاگو ہوتا ہے، جو Iqal کے پچھلے ورژن میں طے کیے گئے تھے۔ اب اوقات ایک تالے سے طے ہوتے ہیں («مخصوص اوقات میں»)۔
schedule-old-remove = ہمیشہ لاگو کریں
lock-schedule-detail = ان اوقات میں بلاک آن اور مقفل رہتا ہے۔ باقی وقت یہ بند رہتا ہے اور اسے بدلا جا سکتا ہے۔
settings-theme-desktop = ڈیسک ٹاپ کی طرح
action-learn-more = یہاں بھی تالا لگائیں
location-learn-more-detail = اسے دبائیں تاکہ جہاں اب کمپیوٹر ہے وہاں بھی بلاک پر تالا لگ جائے۔ اسی جگہ پر دوبارہ دبانے سے عِقال کو جگہ پہچاننے میں مدد ملتی ہے۔
lock-opens-until = یہ تالا اپنا وقت ختم ہونے پر خود کھل جاتا ہے۔
lock-opens-schedule = یہ تالا اپنے اوقات ختم ہونے پر خود کھل جاتا ہے۔
lock-opens-restart = یہ تالا کمپیوٹر دوبارہ شروع ہونے پر خود کھل جاتا ہے۔
lock-opens-location = یہ تالا کمپیوٹر کے اس جگہ سے جانے پر خود کھل جاتا ہے۔
