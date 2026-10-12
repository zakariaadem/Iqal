# Al-Dir — Русский

app-name = Al-Dir
app-description = Блокировщик сайтов и приложений
menu = Меню
menu-import = Импортировать блоки…
menu-export-all = Экспортировать все блоки…
menu-activity = Недавние события
menu-settings = Настройки
menu-about = О программе Al-Dir

action-new-block = Новый блок
action-remove = Убрать
action-add = Добавить
action-cancel = Отмена
action-delete = Удалить
action-create = Создать
action-export-block = Экспортировать этот блок
action-delete-block = Удалить этот блок
action-import-text = Импортировать из текстового файла…
action-add-installed-app = Добавить установленное приложение…
action-add-program-file = Добавить файл программы…
action-add-times = Добавить это время
action-cancel-unlock = Отменить разблокировку
action-learn-location = Я сейчас в этом месте
action-unlock = Разблокировать…
action-remove-lock = Снять замок
action-change-lock = Изменить замок…
action-lock = Запереть
action-choose-on-map = Выбрать на карте…
action-use-area = Использовать эту область

list-empty = Пока ничего нет.

state-off = Выключен
state-active = Активен
state-scheduled = По расписанию
state-locked = Заперт

error-title = Что-то пошло не так
error-service-unreachable = Служба Al-Dir не запущена. Установите или запустите её и повторите попытку.
error-unexpected = Служба Al-Dir дала неожиданный ответ.
error-locked = Этот блок заперт, а это изменение { $reason }. Запертый блок можно только ужесточить.
error-wrong-answer = Неверно.
error-cannot-unlock-early = Этот замок нельзя открыть досрочно.
error-not-admin = Изменять настройки Al-Dir могут только администраторы.

reason-disables = выключило бы его
reason-fewer-users = применило бы его к меньшему числу людей
reason-web-mode = изменило бы способ блокировки сайтов
reason-removes-websites = убрало бы заблокированные сайты
reason-adds-allowed-websites = разрешило бы больше сайтов
reason-safesearch-off = отключило бы SafeSearch
reason-app-mode = изменило бы способ блокировки приложений
reason-removes-apps = убрало бы заблокированные приложения
reason-adds-allowed-apps = разрешило бы больше приложений
reason-adds-schedule = ограничило бы его определённым временем
reason-shortens-schedule = сократило бы его время
reason-adds-allowance = добавило бы лимит времени
reason-increases-allowance = увеличило бы лимит времени
reason-removes-lock = сняло бы замок
reason-shortens-delay = сократило бы ожидание перед разблокировкой
reason-weakens-lock = ослабило бы замок
reason-pending-unlock = изменило бы ожидающую разблокировку
reason-deletes = удалило бы его
reason-hardened-off = отключило бы усиленный режим

empty-title = Блоков пока нет
empty-detail = Блок — это группа сайтов и приложений для блокировки со своим расписанием и замком. Создайте его кнопкой «Новый блок».

block-summary = Правил для сайтов: { $web } · для приложений: { $apps }
rules-count = { $count ->
    [one] { $count } правило
    [few] { $count } правила
    [many] { $count } правил
   *[other] { $count } правила
}

locked-banner = Этот блок заперт. Ограничения можно добавлять, но нельзя убирать.
tab-websites = Сайты
tab-apps = Приложения
tab-users = Люди
tab-schedule = Время
tab-lock = Замок
block-enabled = Включить или выключить блок
confirm-delete = Удалить «{ $name }»?
confirm-delete-detail = Его сайты, приложения, расписание и замок будут удалены.
rule-invalid = «{ $rule }» нельзя использовать: { $problem }

web-mode = Как блокируются сайты
web-mode-block = Блокировать сайты из списка
web-mode-allow-only = Разрешать только сайты из списка
web-blocked = Заблокированные сайты
web-also-blocked = Блокируются, даже если разрешены
web-placeholder = example.com, example.com/page, contains:слово, search:слова
web-syntax = example.com блокирует сайт и все его поддомены. example.com/videos блокирует только эти страницы. contains:casino блокирует любой адрес, содержащий слово. search:бесплатные фильмы блокирует поиск с этими словами.
web-allowed = Разрешённые сайты
web-exceptions = Исключения (всегда разрешены)
web-safesearch = Включать SafeSearch принудительно (Google, Bing, DuckDuckGo, YouTube)
web-partial = Частичная блокировка
web-partial-detail = Блокировать только отвлекающую часть сайта, оставляя остальное.
partial-youtube-shorts = YouTube Shorts
partial-youtube-comments = Комментарии YouTube
partial-youtube-recommendations = Рекомендации и главная страница YouTube
partial-reddit-home = Главная страница Reddit, r/all и r/popular
partial-facebook-feed = Лента новостей Facebook, Watch и Reels
partial-instagram-reels = Reels в Instagram
partial-instagram-explore = Интересное в Instagram
partial-x-trends = Обзор и тренды X (Twitter)

import-done = Импорт завершён
import-result = Добавлено правил: { $added }, пропущено строк: { $skipped }.
import-added = Добавлен: { $name }
import-replaced = Обновлён: { $name }
import-rejected = Не импортирован: { $name } ({ $why })

apps-mode = Как блокируются приложения
apps-mode-block = Блокировать приложения из списка
apps-mode-allow-only = Разрешать только приложения из списка
apps-blocked = Заблокированные приложения
apps-also-blocked = Блокируются, даже если разрешены
apps-exceptions = Исключения (всегда разрешены)
apps-allowed = Разрешённые приложения
apps-placeholder = steam, /opt/games/, title:YouTube, cmd:minecraft
apps-syntax = Имя блокирует программу с таким именем. Папка, оканчивающаяся на /, блокирует все программы в ней. title:Текст закрывает окна, в заголовке которых есть этот текст. cmd:Текст блокирует программы, в командной строке которых он есть. Переименованные копии распознаются правилами hash:.

users-applies-to = К кому применяется этот блок
users-all = Ко всем
users-all-except = Ко всем, кроме…
users-only = Только к…
users-except-these = Кроме этих людей
users-only-these = Только к этим людям
users-note = Администраторы этого компьютера тоже блокируются, если не исключены здесь. Сама система никогда не блокируется.

day-mon = Пн
day-tue = Вт
day-wed = Ср
day-thu = Чт
day-fri = Пт
day-sat = Сб
day-sun = Вс
time-from = с
time-to = до
time-at = в

schedule-limited = Только в определённое время
schedule-limited-detail = Блок действует только в указанное ниже время.
schedule-always = Блок действует всегда.
schedule-times = Время
schedule-date-placeholder = Разово: 2026-10-20 08:00 2026-10-22 18:00
schedule-date-invalid = Запишите разовый период так: дата начала, время начала, дата окончания, время окончания (2026-10-20 08:00 2026-10-22 18:00).
schedule-add-weekly = Добавить еженедельное время
schedule-no-days = Выберите хотя бы один день.
allowance = Дневной лимит времени
allowance-detail = Разрешает заблокированные сайты и приложения на ограниченное время каждый день; после этого они блокируются.
allowance-minutes = Минут в день

area-circle = в пределах { $radius } м от { $lat }, { $lon }
area-polygon = внутри области с углами: { $count }
lock-desc-until = Заперт до { $time }
lock-desc-schedule = Заперт в течение: { $times }
lock-desc-password = Заперт паролем
lock-desc-random = Открывается вводом случайных символов: { $length }
lock-desc-restart = Заперт до перезагрузки компьютера
lock-desc-location = Заперт, пока этот компьютер { $area }
lock-desc-delay = открывается через { $minutes } мин после ответа

lock-none = Не заперт
lock-none-detail = Без замка этот блок можно изменить или выключить в любой момент. С замком его можно только ужесточать, пока замок действует.
lock-active = Заперт
lock-inactive = Замок задан, сейчас не действует
lock-pending = Замок откроется { $time }.
location-learn-detail = Если вы сейчас в этом месте, позвольте Al-Dir запомнить здешние сети Wi-Fi. Так Al-Dir узнаёт, что вы здесь, даже без интернета.
location-learned = Место запомнено
location-learned-detail = Для этого места запомнено сетей Wi-Fi: { $count }.
location-none-chosen = Область ещё не выбрана.

lock-kind-duration = На время
lock-kind-until = До даты
lock-kind-password = Паролем
lock-kind-random = Вводом случайного текста
lock-kind-restart = До перезагрузки компьютера
lock-kind-schedule = В определённое время
lock-kind-location = Пока я в определённом месте
lock-dialog-title = Запереть этот блок
unit-days = дней
unit-hours = часов
unit-minutes = минут
unit-metres = метров
password = Пароль
password-repeat = Повторите пароль
password-generate = Создать пароль, который будет хранить другой человек
password-generate-detail = Al-Dir создаёт пароль и показывает его один раз. Отдайте его человеку, которому доверяете.
password-mismatch = Пароли пусты или не совпадают.
password-generated = Пароль замка
password-generated-detail = { $password }

    Отдайте его человеку, которому доверяете. Больше он показан не будет.
random-length = Количество символов для ввода
lock-restart-detail = Блок остаётся запертым до перезагрузки этого компьютера.
lock-location-detail = Блок остаётся запертым, пока этот компьютер находится в области, а также всякий раз, когда Al-Dir не может определить, где он.
unlock-delay = Ожидание после разблокировки
unlock-delay-detail = С ожиданием правильный ответ открывает замок только по истечении этого времени. Это помогает против внезапных порывов.
lock-duration-zero = Выберите продолжительность.
lock-until-past = Это время уже прошло.
lock-confirm = Запереть этот блок?
lock-confirm-detail = { $lock }. До этого момента ничего в блоке нельзя будет убрать, ослабить или выключить.

unlock-not-possible = Этот замок нельзя открыть досрочно
unlock-not-possible-detail = Он откроется сам, когда истечёт время или когда вы покинете место.
unlock-title = Разблокировать
unlock-type-text = Введите этот текст точно (вставка невозможна):
unlock-enter-password = Введите пароль замка:
unlocked = Разблокировано
unlocked-detail = Блок снова можно изменять.
unlock-pending = Разблокировка начата

map-title = Выбор области
map-help = Нажмите на карту, чтобы поставить центр области, затем задайте её размер. Перетаскивайте карту и меняйте масштаб колёсиком.
map-radius = Радиус

new-block-name = Название блока
new-block-placeholder = Например: Соцсети
export-done = Экспортировано

activity-empty = Пока ничего не заблокировано.
event-app-start = Приложение остановлено
event-app-killed = Приложение закрыто
event-window-closed = Окно закрыто
event-site-blocked = Сайт заблокирован
event-page-blocked = Страница заблокирована

settings-language = Язык
settings-language-system = Как в системе
settings-language-restart = Перезапустите Al-Dir, чтобы применить новый язык.
settings-hardened = Усиленный режим
settings-hardened-detail = Пока какой-либо блок заперт, люди, к которым он применяется, не могут пользоваться sudo и запросами пароля администратора и поэтому не могут остановить Al-Dir. Экстренный доступ: перезагрузка в режим восстановления. Нельзя выключить, пока что-то заперто.
settings-status = Состояние
status-service = Служба Al-Dir { $version }
status-time = Время: { $source }
status-location = Местоположение: { $state }
status-extensions = Расширения браузера: { $list }
status-no-extensions = не подключены
status-not-admin = Вы можете просматривать блоки, но не изменять их (только администраторы).

event-allowance-low = Лимит времени почти исчерпан
event-allowance-used = Лимит времени исчерпан

web-lists = Готовые списки
web-lists-detail = Готовые подборки сайтов. Списки контента для взрослых, азартных игр и соцсетей ежедневно обновляются из большого общественного списка.
list-social = Соцсети
list-adult = Контент для взрослых
list-gambling = Азартные игры
list-search = Поисковые системы
list-news = Новости
list-video = Видео и стриминг
list-games = Игры
list-shopping = Покупки
list-count = { $count ->
    [one] { $count } сайт
    [few] { $count } сайта
    [many] { $count } сайтов
   *[other] { $count } сайта
}
allowance-used-today = Использовано сегодня: { $used } из { $total } мин

menu-stats = Статистика
stats-7 = Последние 7 дней
stats-30 = Последние 30 дней
stats-90 = Последние 90 дней
stats-private = Статистика хранится только на этом компьютере.
stats-top = Чаще всего блокировались
stats-by-block = По блокам
stats-allowance = Использованный лимит времени
stats-minutes = { $minutes } мин
stats-empty = За этот период ничего не заблокировано.

notify-app-blocked = Приложение заблокировано
notify-app-blocked-body = Блок «{ $block }» остановил { $app }.
notify-app-closed = Приложение закрыто
notify-app-closed-body = Блок «{ $block }» закрыл { $app }.
notify-window-closed = Окно закрыто
notify-window-closed-body = Блок «{ $block }» закрыл окно «{ $title }».
notify-allowance-low = Осталось 5 минут
notify-allowance-low-body = Дневное время для «{ $block }» почти закончилось.
notify-allowance-used = Время вышло
notify-allowance-used-body = Дневное время для «{ $block }» закончилось. Блокировка до завтра.

# Saving
unsaved-changes = Несохранённые изменения
action-save = Сохранить
action-discard = Отменить изменения
confirm-unsaved = Сохранить изменения в «{ $name }»?
confirm-unsaved-detail = Если не сохранить, изменения будут потеряны.
save-has-errors = Исправьте эти строки перед сохранением
line-invalid = Строка { $line }: «{ $rule }» нельзя использовать: { $problem }
lines-invalid-more = (ещё строк для исправления: { $count })
list-empty-hint = По одному в строке, например: { $examples }
lock-saves-changes = Другие несохранённые изменения этого блока тоже будут сохранены.

# Statistics window
stats-today = Сегодня
stats-chart = Блокировки по дням
stats-card-total = Блокировок
stats-card-websites = Остановлено сайтов
stats-card-apps = Остановлено приложений
stats-card-busiest = Самый активный день
stats-card-top = Чаще всего блокировалось
stats-before = { $days ->
    [1] Вчера: { $count }
   *[other] Предыдущие { $days } дн.: { $count }
}
stats-blocks = { $count ->
    [one] { $count } блокировка
    [few] { $count } блокировки
    [many] { $count } блокировок
   *[other] { $count } блокировки
}
stats-distinct-sites = { $count ->
    [one] { $count } сайт
    [few] { $count } разных сайта
    [many] { $count } разных сайтов
   *[other] { $count } разных сайта
}
stats-distinct-apps = { $count ->
    [one] { $count } приложение
    [few] { $count } разных приложения
    [many] { $count } разных приложений
   *[other] { $count } разных приложения
}
stats-show-all = Показать все ({ $count })
stats-show-fewer = Показать меньше
stats-allowance-of = { $used } из { $total } мин
