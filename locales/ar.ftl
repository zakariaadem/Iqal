# الدرع — العربية

app-name = الدرع
app-description = حاجب المواقع والتطبيقات
menu = القائمة
menu-import = استيراد كتل…
menu-export-all = تصدير كل الكتل…
menu-activity = النشاط الأخير
menu-settings = الإعدادات
menu-about = عن الدرع

action-new-block = كتلة جديدة
action-remove = إزالة
action-add = إضافة
action-cancel = إلغاء
action-delete = حذف
action-create = إنشاء
action-export-block = تصدير هذه الكتلة
action-delete-block = حذف هذه الكتلة
action-import-text = استيراد من ملف نصي…
action-add-installed-app = إضافة تطبيق مثبّت…
action-add-program-file = إضافة ملف برنامج…
action-add-times = إضافة هذه الأوقات
action-cancel-unlock = إلغاء فتح القفل
action-learn-location = أنا في هذا المكان الآن
action-unlock = فتح القفل…
action-remove-lock = إزالة القفل
action-change-lock = تغيير القفل…
action-lock = قفل
action-choose-on-map = اختيار على الخريطة…
action-use-area = استخدام هذه المنطقة

list-empty = لا يوجد شيء بعد.

state-off = متوقفة
state-active = نشطة
state-scheduled = مجدولة
state-locked = مقفلة

error-title = حدث خطأ
error-service-unreachable = خدمة الدرع لا تعمل. ثبّتها أو شغّلها ثم حاول مرة أخرى.
error-unexpected = أعطت خدمة الدرع إجابة غير متوقعة.
error-locked = هذه الكتلة مقفلة، وهذا التغيير { $reason }. لا يمكن إلا تشديد الكتل المقفلة.
error-wrong-answer = هذا غير صحيح.
error-cannot-unlock-early = لا يمكن فتح هذا القفل مبكرًا.
error-not-admin = لا يغيّر إعدادات الدرع إلا المسؤولون.

reason-disables = سيوقفها
reason-fewer-users = سيطبّقها على عدد أقل من الأشخاص
reason-web-mode = سيغيّر طريقة حجب المواقع
reason-removes-websites = سيزيل مواقع محجوبة
reason-adds-allowed-websites = سيسمح بمواقع أكثر
reason-safesearch-off = سيوقف البحث الآمن
reason-app-mode = سيغيّر طريقة حجب التطبيقات
reason-removes-apps = سيزيل تطبيقات محجوبة
reason-adds-allowed-apps = سيسمح بتطبيقات أكثر
reason-adds-schedule = سيقصرها على أوقات معيّنة
reason-shortens-schedule = سيقصّر أوقاتها
reason-adds-allowance = سيضيف وقتًا مسموحًا
reason-increases-allowance = سيزيد الوقت المسموح
reason-removes-lock = سيزيل القفل
reason-shortens-delay = سيقصّر مدة الانتظار قبل الفتح
reason-weakens-lock = سيضعف القفل
reason-pending-unlock = سيغيّر فتحًا قيد الانتظار
reason-deletes = سيحذفها
reason-hardened-off = سيوقف الوضع المشدّد

empty-title = لا توجد كتل بعد
empty-detail = الكتلة مجموعة من المواقع والتطبيقات المراد حجبها، لها أوقاتها وقفلها. أنشئ واحدة بزر «كتلة جديدة».

block-summary = قواعد المواقع: { $web } · قواعد التطبيقات: { $apps }
rules-count = { $count ->
    [zero] لا قواعد
    [one] قاعدة واحدة
    [two] قاعدتان
    [few] { $count } قواعد
    [many] { $count } قاعدة
   *[other] { $count } قاعدة
}

locked-banner = هذه الكتلة مقفلة. يمكنك إضافة قيود، لكن لا يمكنك إزالتها.
tab-websites = المواقع
tab-apps = التطبيقات
tab-users = الأشخاص
tab-schedule = الأوقات
tab-lock = القفل
block-enabled = تشغيل الكتلة أو إيقافها
confirm-delete = حذف «{ $name }»؟
confirm-delete-detail = ستُزال مواقعها وتطبيقاتها وأوقاتها وقفلها.
rule-invalid = لا يمكن استخدام «{ $rule }»: { $problem }

web-mode = طريقة حجب المواقع
web-mode-block = حجب المواقع المدرجة
web-mode-allow-only = السماح بالمواقع المدرجة فقط
web-blocked = المواقع المحجوبة
web-also-blocked = محجوبة حتى لو كانت مسموحة
web-placeholder = ⁦example.com⁩ Ø£Ù ⁦example.com/page⁩ Ø£Ù ⁦contains:ÙÙÙØ©⁩ Ø£Ù ⁦search:ÙÙÙØ§Øª⁩
web-syntax = ⁦example.com⁩ ÙØ­Ø¬Ø¨ Ø§ÙÙÙÙØ¹ ÙÙÙ ÙØ·Ø§ÙØ§ØªÙ Ø§ÙÙØ±Ø¹ÙØ©. ⁦example.com/videos⁩ ÙØ­Ø¬Ø¨ ØªÙÙ Ø§ÙØµÙØ­Ø§Øª ÙÙØ·. ⁦contains:casino⁩ ÙØ­Ø¬Ø¨ Ø£Ù Ø¹ÙÙØ§Ù ÙØ­ØªÙÙ Ø§ÙÙÙÙØ©. ⁦search:Ø£ÙÙØ§Ù ÙØ¬Ø§ÙÙØ©⁩ ÙØ­Ø¬Ø¨ Ø¹ÙÙÙØ§Øª Ø§ÙØ¨Ø­Ø« Ø§ÙØªÙ ØªØ­ØªÙÙ ÙØ°Ù Ø§ÙÙÙÙØ§Øª.
web-allowed = المواقع المسموحة
web-exceptions = الاستثناءات (مسموحة دائمًا)
web-safesearch = فرض البحث الآمن (Google وBing وDuckDuckGo وYouTube)
web-partial = الحجب الجزئي
web-partial-detail = احجب الجزء المشتّت من الموقع فقط واترك الباقي.
partial-youtube-shorts = مقاطع YouTube Shorts
partial-youtube-comments = تعليقات YouTube
partial-youtube-recommendations = اقتراحات YouTube والصفحة الرئيسية
partial-reddit-home = صفحة Reddit الرئيسية وr/all وr/popular
partial-facebook-feed = آخر أخبار Facebook وWatch وReels
partial-instagram-reels = مقاطع Instagram Reels
partial-instagram-explore = صفحة استكشاف Instagram
partial-x-trends = استكشاف X (تويتر) والمتداول

import-done = انتهى الاستيراد
import-result = أُضيفت { $added } من القواعد، وتُخطّي { $skipped } من الأسطر.
import-added = أُضيفت: { $name }
import-replaced = حُدّثت: { $name }
import-rejected = لم تُستورد: { $name } ({ $why })

apps-mode = طريقة حجب التطبيقات
apps-mode-block = حجب التطبيقات المدرجة
apps-mode-allow-only = السماح بالتطبيقات المدرجة فقط
apps-blocked = التطبيقات المحجوبة
apps-also-blocked = محجوبة حتى لو كانت مسموحة
apps-exceptions = الاستثناءات (مسموحة دائمًا)
apps-allowed = التطبيقات المسموحة
apps-placeholder = ⁦steam⁩ Ø£Ù ⁦/opt/games/⁩ Ø£Ù ⁦title:YouTube⁩ Ø£Ù ⁦cmd:minecraft⁩
apps-syntax = Ø§ÙØ§Ø³Ù ÙØ­Ø¬Ø¨ Ø§ÙØ¨Ø±ÙØ§ÙØ¬ Ø§ÙØ°Ù ÙØ­ÙÙ ÙØ°Ø§ Ø§ÙØ§Ø³Ù. Ø§ÙÙØ¬ÙØ¯ Ø§ÙÙÙØªÙÙ Ø¨Ù ⁦/⁩ ÙØ­Ø¬Ø¨ ÙÙ Ø§ÙØ¨Ø±Ø§ÙØ¬ Ø¯Ø§Ø®ÙÙ. ⁦title:ÙØµ⁩ ÙØºÙÙ Ø§ÙÙÙØ§ÙØ° Ø§ÙØªÙ ÙØ­ØªÙÙ Ø¹ÙÙØ§ÙÙØ§ Ø§ÙÙØµ. ⁦cmd:ÙØµ⁩ ÙØ­Ø¬Ø¨ Ø§ÙØ¨Ø±Ø§ÙØ¬ Ø§ÙØªÙ ÙØ­ØªÙÙ Ø³Ø·Ø± Ø£ÙØ§ÙØ±ÙØ§ Ø§ÙÙØµ. Ø§ÙÙØ³Ø® Ø§ÙÙØ¹Ø§Ø¯ ØªØ³ÙÙØªÙØ§ ØªÙÙØªØ´Ù Ø¨ÙÙØ§Ø¹Ø¯ ⁦hash:⁩.

users-applies-to = على من تُطبَّق هذه الكتلة
users-all = الجميع
users-all-except = الجميع ما عدا…
users-only = فقط…
users-except-these = ما عدا هؤلاء
users-only-these = هؤلاء فقط
users-note = مسؤولو هذا الحاسوب محجوبون أيضًا ما لم يُستثنوا هنا. النظام نفسه لا يُحجب أبدًا.

day-mon = الإثنين
day-tue = الثلاثاء
day-wed = الأربعاء
day-thu = الخميس
day-fri = الجمعة
day-sat = السبت
day-sun = الأحد
time-from = من
time-to = إلى
time-at = الساعة

schedule-limited = في أوقات معيّنة فقط
schedule-limited-detail = تعمل الكتلة فقط خلال الأوقات أدناه.
schedule-always = تعمل الكتلة دائمًا.
schedule-times = الأوقات
schedule-date-placeholder = ÙÙØ±Ø© ÙØ§Ø­Ø¯Ø©: ⁦2026-10-20 08:00 2026-10-22 18:00⁩
schedule-date-invalid = اكتب الفترة الواحدة هكذا: تاريخ البداية، وقت البداية، تاريخ النهاية، وقت النهاية (2026-10-20 08:00 2026-10-22 18:00).
schedule-add-weekly = إضافة أوقات أسبوعية
schedule-no-days = اختر يومًا واحدًا على الأقل.
allowance = وقت مسموح يوميًا
allowance-detail = يسمح بالمواقع والتطبيقات المحجوبة لوقت محدود كل يوم، ثم تُحجب.
allowance-minutes = الدقائق في اليوم

area-circle = ضمن { $radius } م من { $lat }، { $lon }
area-polygon = داخل منطقة لها { $count } من الزوايا
lock-desc-until = مقفلة حتى { $time }
lock-desc-schedule = مقفلة خلال { $times }
lock-desc-password = مقفلة بكلمة مرور
lock-desc-random = تُفتح بكتابة { $length } من الأحرف العشوائية
lock-desc-restart = مقفلة حتى إعادة تشغيل الحاسوب
lock-desc-location = مقفلة ما دام هذا الحاسوب { $area }
lock-desc-delay = تُفتح بعد { $minutes } دقيقة من الإجابة

lock-none = غير مقفلة
lock-none-detail = بدون قفل يمكن تغيير هذه الكتلة أو إيقافها في أي وقت. ومع القفل لا يمكن إلا تشديدها حتى ينتهي القفل.
lock-active = مقفلة
lock-inactive = القفل مضبوط، لكنه غير نافذ الآن
lock-pending = يُفتح القفل في { $time }.
location-learn-detail = إن كنت في هذا المكان الآن، فدع الدرع يتذكر شبكات Wi-Fi فيه. بهذا يعرف الدرع أنك هناك، حتى دون إنترنت.
location-learned = تم تذكّر المكان
location-learned-detail = عدد شبكات Wi-Fi المحفوظة لهذا المكان الآن: { $count }.
location-none-chosen = لم تُختر منطقة بعد.

lock-kind-duration = لمدة من الوقت
lock-kind-until = حتى تاريخ معيّن
lock-kind-password = بكلمة مرور
lock-kind-random = بكتابة نص عشوائي
lock-kind-restart = حتى إعادة تشغيل الحاسوب
lock-kind-schedule = في أوقات معيّنة
lock-kind-location = ما دمت في مكان معيّن
lock-dialog-title = قفل هذه الكتلة
unit-days = أيام
unit-hours = ساعات
unit-minutes = دقائق
unit-metres = متر
password = كلمة المرور
password-repeat = أعد كتابة كلمة المرور
password-generate = أنشئ كلمة مرور يحتفظ بها شخص آخر
password-generate-detail = ينشئ الدرع كلمة مرور ويعرضها مرة واحدة. أعطها لشخص تثق به.
password-mismatch = كلمتا المرور فارغتان أو غير متطابقتين.
password-generated = كلمة مرور القفل
password-generated-detail = { $password }

    أعطها لشخص تثق به. لن تُعرض مرة أخرى.
random-length = عدد الأحرف المطلوب كتابتها
lock-restart-detail = تبقى الكتلة مقفلة حتى يُعاد تشغيل هذا الحاسوب.
lock-location-detail = تبقى الكتلة مقفلة ما دام هذا الحاسوب داخل المنطقة، وكذلك كلما تعذّر على الدرع معرفة مكانه.
unlock-delay = الانتظار بعد الفتح
unlock-delay-detail = مع الانتظار، لا تفتح الإجابة الصحيحة القفل إلا بعد هذه المدة. هذا يساعد على تجاوز الرغبة المفاجئة.
lock-duration-zero = اختر مدة من الوقت.
lock-until-past = هذا الوقت قد مضى.
lock-confirm = قفل هذه الكتلة؟
lock-confirm-detail = { $lock }. وحتى ذلك الحين لا يمكن إزالة أي شيء من هذه الكتلة أو تخفيفه أو إيقافه.

unlock-not-possible = لا يمكن فتح هذا القفل مبكرًا
unlock-not-possible-detail = يُفتح من تلقاء نفسه عندما ينتهي وقته، أو عندما تغادر المكان.
unlock-title = فتح القفل
unlock-type-text = اكتب هذا النص كما هو (اللصق غير ممكن):
unlock-enter-password = أدخل كلمة مرور القفل:
unlocked = فُتح القفل
unlocked-detail = يمكن تغيير الكتلة من جديد.
unlock-pending = بدأ فتح القفل

map-title = اختر المنطقة
map-help = انقر على الخريطة لوضع مركز المنطقة، ثم حدّد حجمها. اسحب لتحريك الخريطة واستخدم العجلة للتكبير.
map-radius = نصف القطر

new-block-name = اسم الكتلة
new-block-placeholder = مثلًا: وسائل التواصل
export-done = تم التصدير

activity-empty = لم يُحجب شيء بعد.
event-app-start = أُوقف تطبيق
event-app-killed = أُغلق تطبيق
event-window-closed = أُغلقت نافذة
event-site-blocked = حُجب موقع
event-page-blocked = حُجبت صفحة

settings-language = اللغة
settings-language-system = مثل لغة النظام
settings-language-restart = أعد تشغيل الدرع لاستخدام اللغة الجديدة.
settings-hardened = الوضع المشدّد
settings-hardened-detail = ما دامت أي كتلة مقفلة، لا يستطيع الأشخاص المطبّقة عليهم استخدام sudo أو نوافذ كلمة مرور المسؤول، فلا يستطيعون إيقاف الدرع. للدخول الطارئ: أعد التشغيل في وضع الاسترداد. لا يمكن إيقافه ما دام شيء مقفلًا.
settings-status = الحالة
status-service = خدمة الدرع { $version }
status-time = الوقت: { $source }
status-location = الموقع: { $state }
status-extensions = إضافات المتصفح: { $list }
status-no-extensions = لا شيء متصل
status-not-admin = يمكنك عرض الكتل لكن لا يمكنك تغييرها (للمسؤولين فقط).

event-allowance-low = الوقت المسموح أوشك على الانتهاء
event-allowance-used = انتهى الوقت المسموح

web-lists = قوائم جاهزة
web-lists-detail = مجموعات جاهزة من المواقع. قوائم المحتوى الإباحي والقمار ووسائل التواصل تُحدَّث يوميًا من قائمة مجتمعية كبيرة.
list-social = وسائل التواصل
list-adult = محتوى إباحي
list-gambling = القمار
list-search = محركات البحث
list-news = الأخبار
list-video = الفيديو والبث
list-games = الألعاب
list-shopping = التسوق
list-count = { $count ->
    [zero] لا مواقع
    [one] موقع واحد
    [two] موقعان
    [few] { $count } مواقع
    [many] { $count } موقعًا
   *[other] { $count } موقع
}
allowance-used-today = المستخدم اليوم: { $used } من { $total } دقيقة

menu-stats = الإحصاءات
stats-period = المدة
stats-7 = آخر 7 أيام
stats-30 = آخر 30 يومًا
stats-90 = آخر 90 يومًا
stats-private = تبقى الإحصاءات على هذا الحاسوب.
stats-total = { $count ->
    [zero] لا حجب في هذه المدة
    [one] حجب واحد في هذه المدة
    [two] حجبان في هذه المدة
    [few] { $count } حالات حجب في هذه المدة
    [many] { $count } حالة حجب في هذه المدة
   *[other] { $count } حالة حجب في هذه المدة
}
stats-top = الأكثر حجبًا
stats-by-block = حسب الكتلة
stats-allowance = الوقت المسموح المستخدم
stats-minutes = { $minutes } د
stats-empty = لم يُحجب شيء في هذه المدة.

notify-app-blocked = حُجب تطبيق
notify-app-blocked-body = أوقفت «{ $block }» تطبيق { $app }.
notify-app-closed = أُغلق تطبيق
notify-app-closed-body = أغلقت «{ $block }» تطبيق { $app }.
notify-window-closed = أُغلقت نافذة
notify-window-closed-body = أغلقت «{ $block }» النافذة «{ $title }».
notify-allowance-low = بقيت 5 دقائق
notify-allowance-low-body = الوقت اليومي لـ«{ $block }» أوشك على الانتهاء.
notify-allowance-used = انتهى الوقت
notify-allowance-used-body = انتهى الوقت اليومي لـ«{ $block }». ستبقى محجوبة حتى الغد.

# Saving
unsaved-changes = تغييرات غير محفوظة
action-save = حفظ
action-discard = تجاهل
confirm-unsaved = هل تريد حفظ التغييرات على «{ $name }»؟
confirm-unsaved-detail = إذا لم تحفظها فستضيع تغييراتك.
save-has-errors = صحّح هذه الأسطر قبل الحفظ
line-invalid = السطر { $line }: لا يمكن استخدام «{ $rule }»: { $problem }
lines-invalid-more = (وأسطر أخرى تحتاج إلى تصحيح: { $count })
list-empty-hint = عنصر واحد في كل سطر، مثل: { $examples }
lock-saves-changes = ستُحفظ أيضًا تغييراتك الأخرى غير المحفوظة على هذه الكتلة.
