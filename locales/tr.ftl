# Iqal — Türkçe

app-name = Iqal
app-description = Web sitesi ve uygulama engelleyici
menu = Menü
menu-import = Engelleri içe aktar…
menu-export-all = Tüm engelleri dışa aktar…
menu-activity = Son etkinlik
menu-settings = Ayarlar
menu-about = Iqal hakkında

action-new-block = Yeni engel
action-remove = Kaldır
action-add = Ekle
action-cancel = İptal
action-delete = Sil
action-create = Oluştur
action-export-block = Bu engeli dışa aktar
action-delete-block = Bu engeli sil
action-import-text = Metin dosyasından içe aktar…
action-add-installed-app = Yüklü bir uygulama ekle…
action-add-program-file = Bir program dosyası ekle…
action-add-times = Bu saatleri ekle
action-cancel-unlock = Kilit açmayı iptal et
action-learn-location = Şu anda bu yerdeyim
action-unlock = Kilidi aç…
action-remove-lock = Kilidi kaldır
action-change-lock = Kilidi değiştir…
action-lock = Kilitle
action-choose-on-map = Haritada seç…
action-use-area = Bu alanı kullan

list-empty = Henüz bir şey yok.

state-off = Kapalı
state-active = Etkin
state-scheduled = Zamanlanmış
state-locked = Kilitli

error-service-unreachable = Iqal hizmeti çalışmıyor
error-unexpected = Iqal hizmeti beklenmeyen bir yanıt verdi.
error-locked = Bu değişiklik { $reason }. Kilitli bir engel yalnızca sıkılaştırılabilir.
error-wrong-answer = Yanlış cevap
error-cannot-unlock-early = Bu kilit erken açılamaz.
error-not-admin = Iqal ayarlarını yalnızca yöneticiler değiştirebilir.

reason-disables = engeli kapatır
reason-fewer-users = engeli daha az kişiye uygular
reason-web-mode = sitelerin engellenme biçimini değiştirir
reason-removes-websites = engellenen siteleri kaldırır
reason-adds-allowed-websites = daha fazla siteye izin verir
reason-safesearch-off = SafeSearch’ü kapatır
reason-app-mode = uygulamaların engellenme biçimini değiştirir
reason-removes-apps = engellenen uygulamaları kaldırır
reason-adds-allowed-apps = daha fazla uygulamaya izin verir
reason-adds-schedule = engeli belirli saatlerle sınırlar
reason-shortens-schedule = saatlerini kısaltır
reason-adds-allowance = süre hakkı ekler
reason-increases-allowance = süre hakkını artırır
reason-removes-lock = kilidi kaldırır
reason-shortens-delay = kilit açma beklemesini kısaltır
reason-weakens-lock = kilidi zayıflatır
reason-pending-unlock = bekleyen bir kilit açmayı değiştirir
reason-deletes = engeli siler
reason-hardened-off = sıkı modu kapatır

empty-title = Henüz engel yok
empty-detail = Engel, kendi saatleri ve kilidi olan, engellenecek web siteleri ve uygulamalardan oluşan bir gruptur. “Yeni engel” ile bir tane oluşturun.

block-summary = { $web } site kuralı · { $apps } uygulama kuralı
rules-count = { $count ->
    [one] { $count } kural
   *[other] { $count } kural
}

locked-banner = Bu engel kilitli. Kısıtlama ekleyebilirsiniz ama kaldıramazsınız.
tab-websites = Web siteleri
tab-apps = Uygulamalar
tab-users = Kişiler
tab-schedule = Saatler
tab-lock = Kilit
block-enabled = Engeli aç veya kapat
confirm-delete = “{ $name }” silinsin mi?
confirm-delete-detail = Siteleri, uygulamaları, saatleri ve kilidi kaldırılacak.
rule-invalid = “{ $rule }” kullanılamaz: { $problem }

web-blocked = Engellenen siteler
web-placeholder = example.com, example.com/page, contains:kelime, search:kelimeler
web-syntax = example.com siteyi ve tüm alt alan adlarını engeller. example.com/videos yalnızca o sayfaları engeller. contains:casino kelimeyi içeren her adresi engeller. search:ücretsiz film bu kelimeleri içeren aramaları engeller.
web-allowed = İzin verilen siteler
web-exceptions = İstisnalar (her zaman izinli)
web-safesearch = SafeSearch’ü zorunlu kıl (Google, Bing, DuckDuckGo, YouTube)

import-done = İçe aktarma tamamlandı
import-result = { $added } kural eklendi, { $skipped } satır atlandı.
import-added = Eklendi: { $name }
import-replaced = Güncellendi: { $name }
import-rejected = İçe aktarılmadı: { $name } ({ $why })

apps-blocked = Engellenen uygulamalar
apps-exceptions = İstisnalar (her zaman izinli)
apps-allowed = İzin verilen uygulamalar
apps-placeholder = steam, /opt/games/, title:YouTube, cmd:minecraft
apps-placeholder-windows = steam, C:\Games\, title:YouTube, cmd:minecraft
apps-syntax = Bir ad, o adı taşıyan programı engeller. / ile biten bir klasör içindeki tüm programları engeller. title:Metin, başlığı metni içeren pencereleri kapatır. cmd:Metin, komut satırı onu içeren programları engeller. Yeniden adlandırılmış kopyalar hash: kurallarıyla yakalanır.
apps-syntax-windows = Bir ad, o adı taşıyan programı engeller. \ ile biten bir klasör içindeki tüm programları engeller. title:Metin, başlığı metni içeren pencereleri kapatır. cmd:Metin, komut satırı onu içeren programları engeller. Yeniden adlandırılmış kopyalar hash: kurallarıyla yakalanır.

users-applies-to = Bu engel kimlere uygulanır
users-all = Herkes
users-all-except = Herkes, şunlar hariç…
users-only = Yalnızca…
users-except-these = Bu kişiler hariç
users-only-these = Yalnızca bu kişiler
users-note = Bu bilgisayarın yöneticileri de burada hariç tutulmadıkça engellenir. Sistemin kendisi asla engellenmez.

day-mon = Pzt
day-tue = Sal
day-wed = Çar
day-thu = Per
day-fri = Cum
day-sat = Cmt
day-sun = Paz
time-from = başlangıç
time-to = bitiş
time-at = saat

schedule-limited = Yalnızca belirli saatlerde
schedule-limited-detail = Engel yalnızca aşağıdaki saatlerde geçerlidir.
schedule-always = Engel her zaman geçerlidir.
schedule-times = Saatler
schedule-date-placeholder = Tek seferlik: 2026-10-20 08:00 2026-10-22 18:00
schedule-date-invalid = Tek seferlik bir dönemi şöyle yazın: başlangıç tarihi, başlangıç saati, bitiş tarihi, bitiş saati (2026-10-20 08:00 2026-10-22 18:00).
schedule-add-weekly = Haftalık saat ekle
schedule-no-days = En az bir gün seçin.
allowance = Günlük süre hakkı
allowance-detail = Engellenen site ve uygulamalara her gün sınırlı bir süre izin verir; sonrasında engellenirler.
allowance-minutes = Günlük dakika

area-circle = { $lat }, { $lon } noktasının { $radius } m yakınında
area-polygon = { $count } köşeli bir alanın içinde
lock-desc-until = { $time } tarihine kadar kilitli
lock-desc-schedule = { $times } boyunca kilitli
lock-desc-password = Parolayla kilitli
lock-desc-random = { $length } rastgele karakter yazılarak açılır
lock-desc-restart = Bilgisayar yeniden başlatılana kadar kilitli
lock-desc-location = Bu bilgisayar { $area } olduğu sürece kilitli
lock-desc-delay = yanıttan { $minutes } dakika sonra açılır

lock-none = Kilitli değil
lock-none-detail = Kilit olmadan bu engel her zaman değiştirilebilir veya kapatılabilir. Kilitle, kilit bitene kadar yalnızca sıkılaştırılabilir.
lock-active = Kilitli
lock-inactive = Kilit ayarlı, şu anda geçerli değil
lock-pending = Kilit { $time } tarihinde açılır.
location-learn-detail = Iqal bu yeri henüz tanımıyor. Bu bilgisayar oradayken düğmeye basın; Iqal çevredeki Wi-Fi ağlarını hatırlasın. O zamana kadar engel nerede olursanız olun kilitli kalır.
location-learned = Yer hatırlandı
location-learned-detail = Bu yer için artık { $count } Wi-Fi ağı hatırlanıyor.
location-none-chosen = Henüz alan seçilmedi.

lock-kind-duration = Bir süreliğine
lock-kind-until = Bir tarihe kadar
lock-kind-password = Parolayla
lock-kind-random = Rastgele metin yazarak
lock-kind-restart = Bilgisayar yeniden başlatılana kadar
lock-kind-schedule = Belirli saatlerde
lock-kind-location = Bir yerdeyken
lock-dialog-title = Bu engeli kilitle
unit-days = gün
unit-hours = saat
unit-minutes = dakika
unit-metres = metre
password = Parola
password-repeat = Parolayı tekrarlayın
password-generate = Başka birinin saklayacağı bir parola oluştur
password-generate-detail = Kilitle'ye bastığınızda Iqal bir parola oluşturur ve bir kez gösterir. Güvendiğiniz birine verin.
password-mismatch = Parolalar boş ya da eşleşmiyor.
password-generated = Kilit parolası
password-generated-detail = { $password }

    Güvendiğiniz birine verin. Bir daha gösterilmeyecek.
random-length = Yazılacak karakter sayısı
lock-restart-detail = Engel, bu bilgisayar yeniden başlatılana kadar kilitli kalır.
lock-location-detail = Engel, bu bilgisayar alanın içindeyken ve Iqal nerede olduğunu anlayamadığı her durumda kilitli kalır.
unlock-delay = Kilit açtıktan sonra bekleme
unlock-delay-detail = Doğru parola veya metin girildikten sonra engel bu kadar dakika daha kilitli kalır. Ani bir isteğin geçmesi için zaman tanır.
lock-duration-zero = Bir süre seçin.
lock-until-past = Bu zaman geçmişte kaldı.
lock-confirm = Bu engel kilitlensin mi?
lock-confirm-detail = { $lock }. O zamana kadar bu engelden hiçbir şey kaldırılamaz, gevşetilemez veya kapatılamaz.

unlock-not-possible = Bu kilit erken açılamaz
unlock-not-possible-detail = Süresi dolduğunda ya da yerden ayrıldığınızda kendiliğinden açılır.
unlock-title = Kilidi aç
unlock-type-text = Bu metni aynen yazın (yapıştırma mümkün değil):
unlock-enter-password = Kilit parolasını girin:
unlocked = Kilit açıldı
unlocked-detail = Engel yeniden değiştirilebilir.
unlock-pending = Kilit açma başladı

map-title = Alanı seçin
map-help = Alanın merkezini yerleştirmek için haritaya tıklayın, sonra boyutunu ayarlayın. Haritayı kaydırmak için sürükleyin, yakınlaştırmak için tekerleği kullanın.
map-radius = Yarıçap

new-block-name = Engelin adı
new-block-placeholder = Örneğin: Sosyal medya
export-done = Dışa aktarıldı

activity-empty = Henüz hiçbir şey engellenmedi.
event-app-start = Uygulama durduruldu
event-app-killed = Uygulama kapatıldı
event-window-closed = Pencere kapatıldı
event-site-blocked = Site engellendi
event-page-blocked = Sayfa engellendi

settings-language = Dil
settings-language-system = Sistemle aynı
settings-language-restart = Yeni dili kullanmak için Iqal’i yeniden başlatın.
settings-hardened = Sıkı mod
settings-hardened-detail = Bir engel kilitli olduğu sürece, engelin uygulandığı kişiler sudo’yu veya yönetici parola isteklerini kullanamaz; bu yüzden Iqal’i durduramazlar. Acil erişim: kurtarma kipinde yeniden başlatın. Bir şey kilitliyken kapatılamaz.
settings-status = Durum
status-service = Iqal hizmeti { $version }
status-time = Saat: { $source }
status-location = Konum: { $state }
status-extensions = Tarayıcı uzantıları: { $list }
status-no-extensions = bağlı uzantı yok
status-not-admin = Engelleri görebilirsiniz ama değiştiremezsiniz (yalnızca yöneticiler).

event-allowance-low = Süre hakkı neredeyse doldu
event-allowance-used = Süre hakkı doldu

web-lists = Hazır listeler
web-lists-detail = Hazır site koleksiyonları. Yetişkin içerik, kumar ve sosyal medya listeleri her gün büyük bir topluluk listesinden güncellenir.
list-social = Sosyal medya
list-adult = Yetişkin içerik
list-gambling = Kumar
list-search = Arama motorları
list-news = Haberler
list-video = Video ve yayın
list-games = Oyunlar
list-shopping = Alışveriş
list-count = { $count ->
    [one] { $count } site
   *[other] { $count } site
}
allowance-used-today = Bugün kullanılan: { $total } dakikanın { $used } dakikası

menu-stats = İstatistikler
stats-7 = Son 7 gün
stats-30 = Son 30 gün
stats-90 = Son 90 gün
stats-private = İstatistikler bu bilgisayarda kalır.
stats-top = En çok engellenenler
stats-by-block = Engele göre
stats-allowance = Kullanılan süre hakkı
stats-minutes = { $minutes } dk
stats-empty = Bu dönemde hiçbir şey engellenmedi.

notify-app-blocked = Uygulama engellendi
notify-app-blocked-body = { $app }, “{ $block }” tarafından durduruldu.
notify-app-closed = Uygulama kapatıldı
notify-app-closed-body = { $app }, “{ $block }” tarafından kapatıldı.
notify-window-closed = Pencere kapatıldı
notify-window-closed-body = “{ $title }”, “{ $block }” tarafından kapatıldı.
notify-allowance-low = 5 dakika kaldı
notify-allowance-low-body = “{ $block }” için günlük süre neredeyse doldu.
notify-allowance-used = Süre doldu
notify-allowance-used-body = “{ $block }” için günlük süre doldu. Yarına kadar engelli.

# Saving
unsaved-changes = Kaydedilmemiş değişiklikler
action-save = Kaydet
action-discard = Vazgeç
confirm-unsaved = “{ $name }” üzerindeki değişiklikler kaydedilsin mi?
confirm-unsaved-detail = Kaydetmezseniz değişiklikleriniz kaybolur.
save-has-errors = Kaydetmeden önce bu satırları düzeltin
line-invalid = Satır { $line }: “{ $rule }” kullanılamaz: { $problem }
lines-invalid-more = (düzeltilmesi gereken { $count } satır daha)
list-empty-hint = Her satıra bir tane, örneğin: { $examples }
lock-saves-changes = Bu engeldeki kaydedilmemiş diğer değişiklikleriniz de kaydedilecek.

# Statistics window
stats-today = Bugün
stats-chart = Günlük engellemeler
stats-card-total = Engelleme sayısı
stats-card-websites = Durdurulan siteler
stats-card-apps = Durdurulan uygulamalar
stats-card-busiest = En yoğun gün
stats-card-top = En çok engellenen
stats-before = { $days ->
    [1] Dün: { $count }
   *[other] Önceki { $days } gün: { $count }
}
stats-blocks = { $count } engelleme
stats-distinct-sites = { $count } farklı site
stats-distinct-apps = { $count } farklı uygulama
stats-show-all = Tümünü göster ({ $count })
stats-show-fewer = Daha az göster
stats-allowance-of = { $total } dakikanın { $used } dakikası

# Rule lists
web-mode-block = Engelleme listesi
web-mode-allow-only = İzin listesi
apps-mode-block = Engelleme listesi
apps-mode-allow-only = İzin listesi
add-websites = Site ekle
add-apps = Uygulama ekle
add-exceptions = İstisna ekle
action-remove-selected = Seçilenleri kaldır
search = Ara
error-service-unreachable-detail = Kurun veya başlatın, sonra yeniden deneyin.
error-locked-title = Engel kilitli
error-wrong-password = Yanlış parola
error-wrong-text = Metin eşleşmiyor
error-no-wifi = Hiç Wi-Fi ağı bulunamadı. Wi-Fi'yi açıp yeniden deneyin.
location-learn-failed = Yer hatırlanamadı
location-known = Iqal bu yeri çevresindeki Wi-Fi ağlarından tanır ve bilgisayarın ne zaman orada olduğunu kendisi bilir.
location-here-question = Bu bilgisayar şu anda bu yerde mi?
location-here-yes = Evet, şu anda orada
location-here-no = Hayır
location-here-detail = Evet ise Iqal çevredeki Wi-Fi ağlarını hatırlar ve bundan sonra bilgisayarın ne zaman gelip gittiğini kendisi bilir. Hayır ise, oraya vardığınızda Kilit sekmesinde "Şu anda bu yerdeyim"e basın.
location-here-unanswered = Bu bilgisayarın şu anda o yerde olup olmadığını seçin.
lock-confirm-timer = Süre dolduğunda engel de kapanır.
schedule-explain = Saatler bu engelin ne zaman engellediğini belirler. Kilit ise engelin ne zaman değiştirilemeyeceğini veya kapatılamayacağını belirler. Örnek: sosyal medyayı burada hafta içi 9:00–17:00 arası engelleyin ve saatler gevşetilemesin diye bir parola kilidi ekleyin. "Bir süreliğine" veya "Bir tarihe kadar" kilidi bittiğinde engeli kapatır.

# Windows: a place lock by name (no map)
location-name = Yerin adı
location-name-placeholder = Örneğin: Ev
area-named = “{ $name }” konumunda
action-close = Kapat

lock-location-detail-named = Bu bilgisayar bu yerde olduğu sürece ve Iqal nerede olduğunu anlayamadığı her durumda engel kilitli kalır.
time-source-network = ağ saati
time-source-system = eşitlenmiş sistem saati
time-source-counted = son güvenilir saatten sayılan (çevrimdışı)
tray-open = Iqal’ı aç
