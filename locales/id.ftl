# Iqal — Bahasa Indonesia

app-name = Iqal Blocker
app-description = Pemblokir situs web dan aplikasi
menu = Menu
menu-import = Impor blok…
menu-export-all = Ekspor semua blok…
menu-activity = Aktivitas terbaru
menu-settings = Pengaturan
menu-about = Tentang Iqal

action-new-block = Blok baru
action-remove = Hapus dari daftar
action-add = Tambah
action-cancel = Batal
action-delete = Hapus
action-create = Buat
action-export-block = Ekspor blok ini
action-delete-block = Hapus blok ini
action-import-text = Impor dari file teks…
action-add-installed-app = Tambah aplikasi terpasang…
action-add-program-file = Tambah file program…
action-add-window-title = Tambah judul jendela…
action-add-times = Tambah waktu ini
action-cancel-unlock = Batalkan pembukaan kunci
action-learn-location = Saya sedang berada di tempat ini
action-unlock = Buka kunci…
action-remove-lock = Hapus kunci
action-change-lock = Ganti kunci…
action-lock = Kunci
action-choose-on-map = Pilih di peta…
action-use-area = Gunakan area ini

list-empty = Belum ada apa-apa.

state-off = Mati
state-active = Aktif
state-scheduled = Terjadwal
state-locked = Terkunci

error-service-unreachable = Layanan Iqal tidak berjalan
error-unexpected = Layanan Iqal memberikan jawaban yang tidak terduga.
error-locked = Perubahan ini { $reason }. Blok yang terkunci hanya bisa dibuat lebih ketat.
error-wrong-answer = Jawaban salah
error-cannot-unlock-early = Kunci ini tidak bisa dibuka lebih awal.
error-not-admin = Hanya administrator yang dapat mengubah pengaturan Iqal.

reason-disables = akan mematikannya
reason-fewer-users = akan menerapkannya ke lebih sedikit orang
reason-web-mode = akan mengubah cara situs diblokir
reason-removes-websites = akan menghapus situs yang diblokir
reason-adds-allowed-websites = akan mengizinkan lebih banyak situs
reason-safesearch-off = akan mematikan SafeSearch
reason-app-mode = akan mengubah cara aplikasi diblokir
reason-removes-apps = akan menghapus aplikasi yang diblokir
reason-adds-allowed-apps = akan mengizinkan lebih banyak aplikasi
reason-adds-schedule = akan membatasinya ke waktu tertentu
reason-shortens-schedule = akan memperpendek waktunya
reason-adds-allowance = akan menambah jatah waktu
reason-increases-allowance = akan menambah jatah waktu
reason-removes-lock = akan menghapus kunci
reason-shortens-delay = akan memperpendek jeda sebelum kunci terbuka
reason-weakens-lock = akan melemahkan kunci
reason-pending-unlock = akan mengubah pembukaan kunci yang tertunda
reason-deletes = akan menghapusnya
reason-hardened-off = akan mematikan mode ketat

empty-title = Belum ada blok
empty-detail = Blok adalah kumpulan situs web dan aplikasi yang diblokir, dengan waktu dan kuncinya sendiri. Buat dengan “Blok baru”.

block-summary = { $web } aturan situs · { $apps } aturan aplikasi
rules-count = { $count } aturan

locked-banner = Blok ini terkunci. Anda bisa menambah batasan, tetapi tidak bisa menghapusnya.
tab-websites = Situs web
tab-apps = Aplikasi
tab-users = Orang
tab-schedule = Waktu
tab-lock = Kunci
block-enabled = Nyalakan atau matikan blok
block-switch-on = Blok menyala
block-switch-off = Blok mati
confirm-delete = Hapus “{ $name }”?
confirm-delete-detail = Situs, aplikasi, waktu, dan kuncinya akan dihapus.
rule-invalid = “{ $rule }” tidak dapat digunakan: { $problem }

web-blocked = Situs yang diblokir
web-placeholder = example.com, example.com/page, contains:kata, search:kata-kata
web-syntax = example.com memblokir situs beserta semua subdomainnya. example.com/videos hanya memblokir halaman tersebut. contains:casino memblokir alamat apa pun yang memuat kata itu. search:film gratis memblokir pencarian yang memuat kata-kata itu.
web-allowed = Situs yang diizinkan
web-exceptions = Pengecualian (selalu diizinkan)
web-safesearch = Wajibkan SafeSearch (Google, Bing, DuckDuckGo, Yandex)

import-done = Impor selesai
import-result = { $added } aturan ditambahkan, { $skipped } baris dilewati.
import-added = Ditambahkan: { $name }
import-replaced = Diperbarui: { $name }
import-rejected = Tidak diimpor: { $name } ({ $why })

apps-blocked = Aplikasi yang diblokir
apps-exceptions = Pengecualian (selalu diizinkan)
apps-allowed = Aplikasi yang diizinkan
apps-placeholder = steam, /opt/games/, title:YouTube, cmd:minecraft
apps-placeholder-windows = steam, C:\Games\, title:YouTube, cmd:minecraft
apps-syntax = Nama memblokir program dengan nama itu. Folder yang diakhiri / memblokir semua program di dalamnya. title:Teks menutup jendela yang judulnya memuat teks itu. cmd:Teks memblokir program yang baris perintahnya memuatnya. Salinan yang diganti namanya dikenali dengan aturan hash:.
apps-syntax-windows = Nama memblokir program dengan nama itu. Folder yang diakhiri \ memblokir semua program di dalamnya. title:Teks menutup jendela yang judulnya memuat teks itu. cmd:Teks memblokir program yang baris perintahnya memuatnya. Salinan yang diganti namanya dikenali dengan aturan hash:.
window-title-intro = Iqal menutup jendela apa pun yang judulnya berisi kata-kata ini, dari program apa pun. Cocok untuk hal yang tidak punya program sendiri, misalnya game di dalam launcher. Untuk situs web, gunakan tab Situs web: jendela browser akan tertutup bersama semua tabnya.
window-title-words = Kata dalam judul jendela
window-title-open = Atau pilih jendela yang sedang terbuka
window-title-empty = Ketik beberapa kata dari judul, atau pilih jendela.

users-applies-to = Blok ini berlaku untuk siapa
users-all = Semua orang
users-all-except = Semua orang kecuali…
users-only = Hanya…
users-except-these = Kecuali orang-orang ini
users-only-these = Hanya orang-orang ini
users-note = Administrator komputer ini juga diblokir, kecuali dikecualikan di sini. Sistem sendiri tidak pernah diblokir.

day-mon = Sen
day-tue = Sel
day-wed = Rab
day-thu = Kam
day-fri = Jum
day-sat = Sab
day-sun = Min
time-from = dari
time-to = sampai
time-at = pukul

schedule-limited = Hanya pada waktu tertentu
schedule-limited-detail = Blok hanya berlaku pada waktu di bawah ini.
schedule-always = Blok selalu berlaku.
schedule-times = Waktu
schedule-date-placeholder = Sekali saja: 2026-10-20 08:00 2026-10-22 18:00
schedule-date-invalid = Tulis periode sekali saja seperti ini: tanggal mulai, jam mulai, tanggal selesai, jam selesai (2026-10-20 08:00 2026-10-22 18:00).
schedule-add-weekly = Tambah waktu mingguan
schedule-no-days = Pilih setidaknya satu hari.
allowance = Jatah waktu harian
allowance-detail = Setiap situs dan aplikasi yang diblokir boleh dipakai selama sekian menit sehari; setelah itu diblokir sampai besok.
allowance-minutes = Menit per hari

area-circle = dalam radius { $radius } m dari { $lat }, { $lon }
area-polygon = di dalam area dengan { $count } sudut
lock-desc-until = Terkunci sampai { $time }
lock-desc-schedule = Terkunci selama { $times }
lock-desc-password = Terkunci dengan kata sandi
lock-desc-random = Terbuka dengan mengetik { $length } karakter acak
lock-desc-restart = Terkunci sampai komputer dinyalakan ulang
lock-desc-location = Terkunci selama komputer ini { $area }
lock-desc-delay = terbuka { $minutes } menit setelah jawaban

lock-none = Tidak terkunci
lock-none-detail = Tanpa kunci, blok ini bisa diubah atau dimatikan kapan saja. Dengan kunci, blok hanya bisa dibuat lebih ketat sampai kunci berakhir.
lock-active = Terkunci
lock-inactive = Kunci terpasang, belum berlaku sekarang
lock-pending = Kunci terbuka pada { $time }.
location-learn-detail = Iqal belum mengenal tempat ini. Saat komputer ini berada di sana, tekan tombolnya agar Iqal mengingat jaringan Wi-Fi di sekitarnya. Sampai saat itu, blok tetap terkunci di mana pun Anda berada.
location-learned = Tempat diingat
location-learned-detail = Sekarang { $count } jaringan Wi-Fi diingat untuk tempat ini.
location-none-chosen = Belum ada area yang dipilih.

lock-kind-duration = Untuk sementara
lock-kind-until = Sampai tanggal tertentu
lock-kind-password = Dengan kata sandi
lock-kind-random = Dengan mengetik teks acak
lock-kind-restart = Sampai komputer dinyalakan ulang
lock-kind-schedule = Pada waktu tertentu
lock-kind-location = Selama saya di suatu tempat
lock-dialog-title = Kunci blok ini
unit-days = hari
unit-hours = jam
unit-minutes = menit
unit-metres = meter
password = Kata sandi
password-repeat = Ulangi kata sandi
password-show = Tampilkan kata sandi
password-generate = Buat kata sandi untuk disimpan orang lain
password-generate-detail = Saat Anda menekan Kunci, Iqal membuat kata sandi dan menampilkannya sekali. Berikan kepada orang yang Anda percayai.
password-mismatch = Kata sandi kosong atau tidak cocok.
password-generated = Kata sandi kunci
password-generated-detail = { $password }

    Berikan kepada orang yang Anda percaya. Ini tidak akan ditampilkan lagi.
random-length = Jumlah karakter yang diketik
lock-restart-detail = Blok tetap terkunci sampai komputer ini dinyalakan ulang.
lock-location-detail = Blok tetap terkunci selama komputer ini berada di dalam area, dan juga setiap kali Iqal tidak bisa mengetahui lokasinya.
unlock-delay = Jeda setelah membuka kunci
unlock-delay-detail = Setelah kata sandi atau teks yang benar dimasukkan, blok tetap terkunci selama sekian menit lagi. Ini memberi waktu agar dorongan tiba-tiba mereda.
lock-duration-zero = Pilih lamanya waktu.
lock-until-past = Waktu itu sudah lewat.
lock-confirm = Kunci blok ini?
lock-confirm-detail = { $lock }. Sampai saat itu, tidak ada isi blok ini yang dapat dihapus, dilonggarkan, atau dimatikan.

unlock-not-possible = Kunci ini tidak bisa dibuka lebih awal
unlock-not-possible-detail = Kunci terbuka sendiri saat waktunya habis, atau saat Anda meninggalkan tempat itu.
unlock-title = Buka kunci
unlock-type-text = Ketik teks ini persis (tidak bisa ditempel):
unlock-enter-password = Masukkan kata sandi kunci:
unlocked = Kunci terbuka
unlocked-detail = Blok sekarang mati. Anda bisa mengubahnya atau menyalakannya lagi.
unlock-pending = Pembukaan kunci dimulai

map-title = Pilih area
map-help = Klik peta untuk menaruh titik tengah area, lalu atur ukurannya. Seret untuk menggeser peta dan gunakan roda untuk memperbesar.
map-radius = Radius

new-block-name = Nama blok
new-block-placeholder = Misalnya: Media sosial
export-done = Diekspor

activity-empty = Belum ada yang diblokir.
event-app-start = Aplikasi dihentikan
event-app-killed = Aplikasi ditutup
event-window-closed = Jendela ditutup
event-site-blocked = Situs diblokir
event-page-blocked = Halaman diblokir

settings-language = Bahasa
settings-language-system = Sama dengan sistem
settings-theme = Tampilan
settings-theme-system = Sama dengan Windows
settings-theme-light = Terang
settings-theme-dark = Gelap
settings-language-restart = Mulai ulang Iqal untuk memakai bahasa baru.
settings-hardened = Mode ketat
settings-hardened-detail = Selama ada blok yang terkunci, orang yang terkena blok tidak dapat memakai sudo atau permintaan kata sandi administrator, sehingga tidak dapat menghentikan Iqal. Akses darurat: mulai ulang ke mode pemulihan. Tidak dapat dimatikan selama ada yang terkunci.
settings-status = Status
status-service = Layanan Iqal { $version }
status-time = Waktu: { $source }
status-location = Lokasi: { $state }
status-extensions = Ekstensi peramban: { $list }
status-no-extensions = tidak ada yang terhubung
status-not-admin = Anda dapat melihat blok tetapi tidak dapat mengubahnya (khusus administrator).

event-allowance-low = Jatah waktu hampir habis
event-allowance-used = Jatah waktu habis

web-lists = Daftar siap pakai
web-lists-detail = Kumpulan situs yang sudah siap. Daftar konten dewasa, judi, dan media sosial diperbarui setiap hari dari daftar komunitas yang besar.
list-social = Media sosial
list-adult = Konten dewasa
list-gambling = Judi
list-search = Mesin pencari
list-news = Berita
list-video = Video dan streaming
list-games = Gim
list-shopping = Belanja
list-count = { $count } situs
allowance-used-today = Terpakai hari ini: { $used } dari { $total } menit

menu-stats = Statistik
stats-7 = 7 hari terakhir
stats-30 = 30 hari terakhir
stats-90 = 90 hari terakhir
stats-private = Statistik tetap berada di komputer ini.
stats-top = Paling sering diblokir
stats-by-block = Per blok
stats-allowance = Jatah waktu terpakai
stats-minutes = { $minutes } mnt
stats-empty = Tidak ada yang diblokir dalam periode ini.

notify-app-blocked = Aplikasi diblokir
notify-app-blocked-body = { $app } dihentikan oleh “{ $block }”.
notify-app-closed = Aplikasi ditutup
notify-app-closed-body = { $app } ditutup oleh “{ $block }”.
notify-window-closed = Jendela ditutup
notify-window-closed-body = “{ $title }” ditutup oleh “{ $block }”.
notify-allowance-low = Tersisa 5 menit
notify-allowance-low-body = Waktu harian untuk { $item } (“{ $block }”) hampir habis.
notify-allowance-used = Waktu habis
notify-allowance-used-body = Waktu harian untuk { $item } (“{ $block }”) sudah habis. Diblokir sampai besok.

# Saving
unsaved-changes = Perubahan belum disimpan
action-save = Simpan
action-discard = Buang
confirm-unsaved = Simpan perubahan pada “{ $name }”?
confirm-unsaved-detail = Jika tidak disimpan, perubahan Anda akan hilang.
save-has-errors = Perbaiki baris-baris ini sebelum menyimpan
line-invalid = Baris { $line }: “{ $rule }” tidak dapat digunakan: { $problem }
lines-invalid-more = ({ $count } baris lain perlu diperbaiki)
list-empty-hint = Satu per baris, misalnya: { $examples }
lock-saves-changes = Perubahan lain yang belum disimpan pada blok ini juga akan disimpan.

# Statistics window
stats-today = Hari ini
stats-chart = Pemblokiran per hari
stats-card-total = Jumlah pemblokiran
stats-card-websites = Situs dihentikan
stats-card-apps = Aplikasi dihentikan
stats-card-busiest = Hari tersibuk
stats-card-top = Paling sering diblokir
stats-before = { $days ->
    [1] Kemarin: { $count }
   *[other] { $days } hari sebelumnya: { $count }
}
stats-blocks = { $count } pemblokiran
stats-distinct-sites = { $count } situs berbeda
stats-distinct-apps = { $count } aplikasi berbeda
stats-show-all = Tampilkan semua ({ $count })
stats-show-fewer = Tampilkan lebih sedikit
stats-allowance-of = { $used } dari { $total } menit

# Rule lists
web-mode-block = Daftar blokir
web-mode-allow-only = Daftar izin
apps-mode-block = Daftar blokir
apps-mode-allow-only = Daftar izin
add-websites = Tambah situs
add-apps = Tambah aplikasi
add-exceptions = Tambah pengecualian
action-remove-selected = Hapus yang dipilih
search = Cari
error-service-unreachable-detail = Pasang atau jalankan layanan itu, lalu coba lagi.
error-locked-title = Blok ini terkunci
error-wrong-password = Kata sandi salah
error-wrong-text = Teksnya tidak cocok
error-no-wifi = Tidak ada jaringan Wi-Fi yang ditemukan. Nyalakan Wi-Fi lalu coba lagi.
location-learn-failed = Tidak bisa mengingat tempat ini
location-known = Iqal mengenali tempat ini dari jaringan Wi-Fi di sekitarnya, dan tahu sendiri kapan komputer ini berada di sana.
location-here-question = Apakah komputer ini sedang berada di tempat ini?
location-here-yes = Ya, sedang di sana
location-here-no = Tidak
location-here-detail = Jika ya, Iqal mengingat jaringan Wi-Fi di sekitarnya dan sejak itu tahu sendiri kapan komputer datang dan pergi. Jika tidak, tekan "Saya sedang berada di tempat ini" di tab Kunci saat Anda sudah di sana.
location-here-unanswered = Pilih apakah komputer ini sedang berada di tempat itu.
lock-confirm-timer = Saat waktunya habis, blok juga dimatikan.
schedule-explain = Waktu menentukan kapan blok ini memblokir. Kunci menentukan kapan blok tidak bisa diubah atau dimatikan. Contoh: blokir media sosial di sini pada hari kerja pukul 9:00–17:00, lalu tambahkan kunci kata sandi agar waktunya tidak bisa dilonggarkan. Kunci "Untuk sementara" atau "Sampai tanggal tertentu" mematikan blok saat berakhir.

# Windows: a place lock by name (no map)
location-name = Nama tempat
location-name-placeholder = Misalnya: Rumah
area-named = di “{ $name }”
action-close = Tutup

lock-location-detail-named = Blokir tetap terkunci selama komputer ini berada di tempat ini, dan juga setiap kali Iqal tidak dapat mengetahui lokasinya.
time-source-network = waktu jaringan
time-source-system = waktu sistem yang tersinkron
time-source-counted = dihitung dari waktu tepercaya terakhir (luring)
tray-open = Buka Iqal

setup-window-title = Pemasangan Iqal Blocker
setup-install-title = Pasang Iqal Blocker { $version }?
setup-install-detail = Iqal akan dipasang di { $folder }. Iqal berjalan di latar belakang sebagai layanan Windows dan memblokir situs web dan aplikasi yang Anda pilih.
setup-update-title = Perbarui Iqal Blocker ke { $version }?
setup-update-detail = Blok Anda tetap disimpan.
action-install = Pasang
action-update = Perbarui
setup-installed = Iqal Blocker sudah terpasang
setup-installed-detail = Buka Iqal Blocker dari menu Start untuk membuat blok pertama Anda.
setup-failed = Iqal tidak dapat dipasang
setup-locked = Iqal tidak dapat diperbarui sekarang
setup-locked-detail = Ada blok yang terkunci. Iqal dapat diperbarui setelah kuncinya berakhir.
uninstall-title = Hapus Iqal Blocker?
uninstall-detail = Layanan, pengaturan peramban, dan programnya dihapus. Blok Anda tetap ada di { $folder }, jika Anda memasang Iqal lagi.
uninstall-action = Hapus
uninstall-done = Iqal Blocker telah dihapus
uninstall-failed = Iqal tidak dapat dihapus
uninstall-locked = Iqal tidak dapat dihapus sekarang
uninstall-locked-detail = Ada blok yang terkunci. Iqal dapat dihapus setelah semua kunci berakhir.
setup-updated = Iqal sudah diperbarui
web-safesearch-detail = Selama ini aktif, mesin pencari lain dan situs yang dikenal berisi konten dewasa diblokir (Reddit, X, Instagram, Tumblr, Bluesky, Imgur…).
allowance-used-today-items = Terpakai hari ini:
allowance-used-item = { $item } { $used } dari { $total } mnt
allowance-unused = Belum ada yang terpakai hari ini.
unsaved-locked-weaker = Blok ini terkunci, dan perubahan ini { $reason }.
schedule-old = Jadwal waktu lama
schedule-old-detail = Blok ini hanya berlaku pada waktu tertentu, yang diatur di versi Iqal sebelumnya. Waktu sekarang diatur dengan kunci (“Pada waktu tertentu”).
schedule-old-remove = Jadikan selalu berlaku
lock-schedule-detail = Pada waktu-waktu ini blok menyala dan terkunci. Di luar itu blok mati dan bisa diubah.
