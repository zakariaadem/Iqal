# Al-Dir — اردو

app-name = Al-Dir
app-description = ویب سائٹ اور ایپ بلاکر
menu = مینو
menu-import = بلاک درآمد کریں…
menu-export-all = تمام بلاک برآمد کریں…
menu-activity = حالیہ سرگرمی
menu-settings = ترتیبات
menu-about = Al-Dir کے بارے میں

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

error-title = کچھ غلط ہو گیا
error-service-unreachable = Al-Dir سروس نہیں چل رہی۔ اسے انسٹال کریں یا شروع کریں، پھر دوبارہ کوشش کریں۔
error-unexpected = Al-Dir سروس نے غیر متوقع جواب دیا۔
error-locked = یہ بلاک مقفل ہے، اور یہ تبدیلی { $reason }۔ مقفل بلاک کو صرف مزید سخت کیا جا سکتا ہے۔
error-wrong-answer = یہ درست نہیں ہے۔
error-cannot-unlock-early = یہ تالا وقت سے پہلے نہیں کھولا جا سکتا۔
error-not-admin = صرف منتظمین Al-Dir کی ترتیبات بدل سکتے ہیں۔

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
confirm-delete = «{ $name }» حذف کریں؟
confirm-delete-detail = اس کی ویب سائٹس، ایپس، اوقات اور تالا ہٹا دیے جائیں گے۔
rule-invalid = «{ $rule }» استعمال نہیں ہو سکتا: { $problem }

web-mode = ویب سائٹس بلاک کرنے کا طریقہ
web-mode-block = فہرست کی ویب سائٹس بلاک کریں
web-mode-allow-only = صرف فہرست کی ویب سائٹس کی اجازت دیں
web-blocked = بلاک شدہ ویب سائٹس
web-also-blocked = اجازت ہونے پر بھی بلاک
web-placeholder = ⁦example.com⁩ یا ⁦example.com/page⁩ یا ⁦contains:لفظ⁩ یا ⁦search:الفاظ⁩
web-syntax = ⁦example.com⁩ سائٹ اور اس کے تمام ذیلی ڈومین بلاک کرتا ہے۔ ⁦example.com/videos⁩ صرف وہ صفحات بلاک کرتا ہے۔ ⁦contains:casino⁩ ہر وہ پتا بلاک کرتا ہے جس میں یہ لفظ ہو۔ ⁦search:مفت فلمیں⁩ ان الفاظ والی تلاش بلاک کرتا ہے۔
web-allowed = اجازت یافتہ ویب سائٹس
web-exceptions = استثنا (ہمیشہ اجازت)
web-safesearch = SafeSearch لازمی کریں (Google، Bing، DuckDuckGo، YouTube)
web-partial = جزوی بلاک
web-partial-detail = سائٹ کا صرف توجہ ہٹانے والا حصہ بلاک کریں اور باقی رہنے دیں۔
partial-youtube-shorts = YouTube Shorts
partial-youtube-comments = YouTube تبصرے
partial-youtube-recommendations = YouTube تجاویز اور ہوم فیڈ
partial-reddit-home = Reddit کا پہلا صفحہ، r/all اور r/popular
partial-facebook-feed = Facebook نیوز فیڈ، Watch اور Reels
partial-instagram-reels = Instagram Reels
partial-instagram-explore = Instagram Explore
partial-x-trends = X (Twitter) Explore اور ٹرینڈز

import-done = درآمد مکمل
import-result = { $added } اصول شامل ہوئے، { $skipped } سطریں چھوڑ دی گئیں۔
import-added = شامل: { $name }
import-replaced = تازہ کیا گیا: { $name }
import-rejected = درآمد نہیں ہوا: { $name } ({ $why })

apps-mode = ایپس بلاک کرنے کا طریقہ
apps-mode-block = فہرست کی ایپس بلاک کریں
apps-mode-allow-only = صرف فہرست کی ایپس کی اجازت دیں
apps-blocked = بلاک شدہ ایپس
apps-also-blocked = اجازت ہونے پر بھی بلاک
apps-exceptions = استثنا (ہمیشہ اجازت)
apps-allowed = اجازت یافتہ ایپس
apps-placeholder = ⁦steam⁩ یا ⁦/opt/games/⁩ یا ⁦title:YouTube⁩ یا ⁦cmd:minecraft⁩
apps-syntax = نام اسی نام والے پروگرام کو بلاک کرتا ہے۔ ⁦/⁩ پر ختم ہونے والا فولڈر اس کے اندر ہر پروگرام بلاک کرتا ہے۔ ⁦title:متن⁩ وہ ونڈوز بند کرتا ہے جن کے عنوان میں یہ متن ہو۔ ⁦cmd:متن⁩ وہ پروگرام بلاک کرتا ہے جن کی کمانڈ لائن میں یہ ہو۔ نام بدلی ہوئی نقلیں ⁦hash:⁩ اصولوں سے پکڑی جاتی ہیں۔

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
allowance-detail = بلاک شدہ ویب سائٹس اور ایپس کو ہر روز محدود وقت کے لیے اجازت دیتا ہے؛ اس کے بعد وہ بلاک ہو جاتی ہیں۔
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
location-learn-detail = اگر آپ اس وقت اسی جگہ پر ہیں تو Al-Dir کو یہاں کے Wi-Fi نیٹ ورک یاد رکھنے دیں۔ اسی طرح Al-Dir جانتا ہے کہ آپ وہاں ہیں، انٹرنیٹ کے بغیر بھی۔
location-learned = جگہ یاد کر لی گئی
location-learned-detail = اس جگہ کے لیے اب { $count } Wi-Fi نیٹ ورک یاد ہیں۔
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
password-generate = کسی اور کے پاس رکھنے کے لیے پاس ورڈ بنائیں
password-generate-detail = Al-Dir ایک پاس ورڈ بنا کر ایک بار دکھاتا ہے۔ اسے کسی قابلِ اعتماد شخص کو دیں۔
password-mismatch = پاس ورڈ خالی ہیں یا آپس میں نہیں ملتے۔
password-generated = تالے کا پاس ورڈ
password-generated-detail = { $password }

    اسے کسی قابلِ اعتماد شخص کو دیں۔ یہ دوبارہ نہیں دکھایا جائے گا۔
random-length = لکھنے کے لیے حروف
lock-restart-detail = بلاک اس کمپیوٹر کے دوبارہ شروع ہونے تک مقفل رہتا ہے۔
lock-location-detail = بلاک تب تک مقفل رہتا ہے جب تک یہ کمپیوٹر علاقے کے اندر ہو، اور جب بھی Al-Dir یہ نہ جان سکے کہ یہ کہاں ہے۔
unlock-delay = تالا کھولنے کے بعد انتظار
unlock-delay-detail = انتظار کے ساتھ، درست جواب تالا اسی وقت کے بعد کھولتا ہے۔ یہ اچانک خواہش میں مدد دیتا ہے۔
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
unlocked-detail = بلاک کو دوبارہ بدلا جا سکتا ہے۔
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
settings-language-restart = نئی زبان استعمال کرنے کے لیے Al-Dir دوبارہ شروع کریں۔
settings-hardened = سخت موڈ
settings-hardened-detail = جب تک کوئی بلاک مقفل ہے، جن لوگوں پر یہ لاگو ہے وہ sudo یا منتظم پاس ورڈ کی درخواستیں استعمال نہیں کر سکتے، اس لیے Al-Dir کو روک نہیں سکتے۔ ہنگامی رسائی: ریکوری موڈ میں دوبارہ شروع کریں۔ کچھ مقفل ہونے کے دوران بند نہیں ہو سکتا۔
settings-status = حالت
status-service = Al-Dir سروس { $version }
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
stats-period = مدت
stats-7 = پچھلے 7 دن
stats-30 = پچھلے 30 دن
stats-90 = پچھلے 90 دن
stats-private = اعداد و شمار اسی کمپیوٹر پر رہتے ہیں۔
stats-total = { $count ->
    [one] اس مدت میں { $count } بلاک
   *[other] اس مدت میں { $count } بلاک
}
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
notify-allowance-low-body = «{ $block }» کا روزانہ وقت تقریباً ختم ہو گیا ہے۔
notify-allowance-used = وقت ختم
notify-allowance-used-body = «{ $block }» کا روزانہ وقت ختم ہو گیا۔ یہ کل تک بلاک رہے گا۔
