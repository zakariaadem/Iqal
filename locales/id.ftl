# Al-Dir — Bahasa Indonesia

app-name = Al-Dir
app-description = Pemblokir situs web dan aplikasi
menu = Menu
menu-import = Impor blok…
menu-export-all = Ekspor semua blok…
menu-activity = Aktivitas terbaru
menu-settings = Pengaturan
menu-about = Tentang Al-Dir

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

error-title = Terjadi kesalahan
error-service-unreachable = Layanan Al-Dir tidak berjalan. Pasang atau jalankan, lalu coba lagi.
error-unexpected = Layanan Al-Dir memberikan jawaban yang tidak terduga.
error-locked = Blok ini terkunci, dan perubahan ini { $reason }. Blok yang terkunci hanya bisa dibuat lebih ketat.
error-wrong-answer = Itu tidak benar.
error-cannot-unlock-early = Kunci ini tidak bisa dibuka lebih awal.
error-not-admin = Hanya administrator yang dapat mengubah pengaturan Al-Dir.

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
confirm-delete = Hapus “{ $name }”?
confirm-delete-detail = Situs, aplikasi, waktu, dan kuncinya akan dihapus.
rule-invalid = “{ $rule }” tidak dapat digunakan: { $problem }

web-mode = Cara situs diblokir
web-mode-block = Blokir situs dalam daftar
web-mode-allow-only = Hanya izinkan situs dalam daftar
web-blocked = Situs yang diblokir
web-also-blocked = Diblokir meskipun diizinkan
web-placeholder = example.com, example.com/page, contains:kata, search:kata-kata
web-syntax = example.com memblokir situs beserta semua subdomainnya. example.com/videos hanya memblokir halaman tersebut. contains:casino memblokir alamat apa pun yang memuat kata itu. search:film gratis memblokir pencarian yang memuat kata-kata itu.
web-allowed = Situs yang diizinkan
web-exceptions = Pengecualian (selalu diizinkan)
web-safesearch = Wajibkan SafeSearch (Google, Bing, DuckDuckGo, YouTube)
web-partial = Blokir sebagian
web-partial-detail = Blokir hanya bagian situs yang mengganggu dan biarkan sisanya.
partial-youtube-shorts = YouTube Shorts
partial-youtube-comments = Komentar YouTube
partial-youtube-recommendations = Rekomendasi dan beranda YouTube
partial-reddit-home = Halaman depan Reddit, r/all dan r/popular
partial-facebook-feed = Kabar berita Facebook, Watch dan Reels
partial-instagram-reels = Reels Instagram
partial-instagram-explore = Jelajah Instagram
partial-x-trends = Jelajah dan tren X (Twitter)

import-done = Impor selesai
import-result = { $added } aturan ditambahkan, { $skipped } baris dilewati.
import-added = Ditambahkan: { $name }
import-replaced = Diperbarui: { $name }
import-rejected = Tidak diimpor: { $name } ({ $why })

apps-mode = Cara aplikasi diblokir
apps-mode-block = Blokir aplikasi dalam daftar
apps-mode-allow-only = Hanya izinkan aplikasi dalam daftar
apps-blocked = Aplikasi yang diblokir
apps-also-blocked = Diblokir meskipun diizinkan
apps-exceptions = Pengecualian (selalu diizinkan)
apps-allowed = Aplikasi yang diizinkan
apps-placeholder = steam, /opt/games/, title:YouTube, cmd:minecraft
apps-syntax = Nama memblokir program dengan nama itu. Folder yang diakhiri / memblokir semua program di dalamnya. title:Teks menutup jendela yang judulnya memuat teks itu. cmd:Teks memblokir program yang baris perintahnya memuatnya. Salinan yang diganti namanya dikenali dengan aturan hash:.

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
allowance-detail = Mengizinkan situs dan aplikasi yang diblokir selama waktu terbatas setiap hari; setelah itu diblokir.
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
location-learn-detail = Jika Anda sedang berada di tempat ini, biarkan Al-Dir mengingat jaringan Wi-Fi-nya. Dengan begitu Al-Dir tahu Anda ada di sana, bahkan tanpa internet.
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
password-generate = Buat kata sandi untuk disimpan orang lain
password-generate-detail = Al-Dir membuat kata sandi dan menampilkannya sekali. Berikan kepada orang yang Anda percaya.
password-mismatch = Kata sandi kosong atau tidak cocok.
password-generated = Kata sandi kunci
password-generated-detail = { $password }

    Berikan kepada orang yang Anda percaya. Ini tidak akan ditampilkan lagi.
random-length = Jumlah karakter yang diketik
lock-restart-detail = Blok tetap terkunci sampai komputer ini dinyalakan ulang.
lock-location-detail = Blok tetap terkunci selama komputer ini berada di dalam area, dan juga setiap kali Al-Dir tidak bisa mengetahui lokasinya.
unlock-delay = Jeda setelah membuka kunci
unlock-delay-detail = Dengan jeda, jawaban yang benar baru membuka kunci setelah waktu ini. Ini membantu menahan dorongan sesaat.
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
unlocked-detail = Blok bisa diubah lagi.
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
settings-language-restart = Mulai ulang Al-Dir untuk memakai bahasa baru.
settings-hardened = Mode ketat
settings-hardened-detail = Selama ada blok yang terkunci, orang yang terkena blok tidak dapat memakai sudo atau permintaan kata sandi administrator, sehingga tidak dapat menghentikan Al-Dir. Akses darurat: mulai ulang ke mode pemulihan. Tidak dapat dimatikan selama ada yang terkunci.
settings-status = Status
status-service = Layanan Al-Dir { $version }
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
stats-period = Periode
stats-7 = 7 hari terakhir
stats-30 = 30 hari terakhir
stats-90 = 90 hari terakhir
stats-private = Statistik tetap berada di komputer ini.
stats-total = { $count } pemblokiran dalam periode ini
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
notify-allowance-low-body = Waktu harian untuk “{ $block }” hampir habis.
notify-allowance-used = Waktu habis
notify-allowance-used-body = Waktu harian untuk “{ $block }” sudah habis. Diblokir sampai besok.
