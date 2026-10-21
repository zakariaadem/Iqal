# Iqal — Deutsch

app-name = Iqal
app-description = Sperre für Websites und Apps
menu = Menü
menu-import = Sperren importieren…
menu-export-all = Alle Sperren exportieren…
menu-activity = Letzte Aktivität
menu-settings = Einstellungen
menu-about = Über Iqal

action-new-block = Neue Sperre
action-remove = Entfernen
action-add = Hinzufügen
action-cancel = Abbrechen
action-delete = Löschen
action-create = Erstellen
action-export-block = Diese Sperre exportieren
action-delete-block = Diese Sperre löschen
action-import-text = Aus einer Textdatei importieren…
action-add-installed-app = Installierte App hinzufügen…
action-add-program-file = Programmdatei hinzufügen…
action-add-times = Diese Zeiten hinzufügen
action-cancel-unlock = Entsperren abbrechen
action-learn-location = Ich bin jetzt an diesem Ort
action-unlock = Entsperren…
action-remove-lock = Schloss entfernen
action-change-lock = Schloss ändern…
action-lock = Abschließen
action-choose-on-map = Auf der Karte wählen…
action-use-area = Diesen Bereich verwenden

list-empty = Noch nichts vorhanden.

state-off = Aus
state-active = Aktiv
state-scheduled = Geplant
state-locked = Abgeschlossen

error-service-unreachable = Der Iqal-Dienst läuft nicht
error-unexpected = Der Iqal-Dienst hat unerwartet geantwortet.
error-locked = Diese Änderung { $reason }. Eine abgeschlossene Sperre kann nur strenger gemacht werden.
error-wrong-answer = Falsche Antwort
error-cannot-unlock-early = Dieses Schloss kann nicht vorzeitig geöffnet werden.
error-not-admin = Nur Administratoren können die Einstellungen von Iqal ändern.

reason-disables = würde sie ausschalten
reason-fewer-users = würde sie auf weniger Personen anwenden
reason-web-mode = würde ändern, wie Websites gesperrt werden
reason-removes-websites = würde gesperrte Websites entfernen
reason-adds-allowed-websites = würde mehr Websites erlauben
reason-safesearch-off = würde SafeSearch ausschalten
reason-app-mode = würde ändern, wie Apps gesperrt werden
reason-removes-apps = würde gesperrte Apps entfernen
reason-adds-allowed-apps = würde mehr Apps erlauben
reason-adds-schedule = würde sie auf bestimmte Zeiten beschränken
reason-shortens-schedule = würde ihre Zeiten verkürzen
reason-adds-allowance = würde ein Zeitkontingent hinzufügen
reason-increases-allowance = würde das Zeitkontingent erhöhen
reason-removes-lock = würde das Schloss entfernen
reason-shortens-delay = würde die Wartezeit vor dem Entsperren verkürzen
reason-weakens-lock = würde das Schloss schwächen
reason-pending-unlock = würde ein laufendes Entsperren ändern
reason-deletes = würde sie löschen
reason-hardened-off = würde den verschärften Modus ausschalten

empty-title = Noch keine Sperren
empty-detail = Eine Sperre ist eine Gruppe von Websites und Apps, die gesperrt werden, mit eigenen Zeiten und eigenem Schloss. Erstellen Sie eine mit „Neue Sperre“.

block-summary = Website-Regeln: { $web } · App-Regeln: { $apps }
rules-count = { $count ->
    [one] { $count } Regel
   *[other] { $count } Regeln
}

locked-banner = Diese Sperre ist abgeschlossen. Sie können Einschränkungen hinzufügen, aber nicht entfernen.
tab-websites = Websites
tab-apps = Apps
tab-users = Personen
tab-schedule = Zeiten
tab-lock = Schloss
block-enabled = Sperre ein oder aus
block-switch-on = Sperre ist ein
block-switch-off = Sperre ist aus
confirm-delete = „{ $name }“ löschen?
confirm-delete-detail = Ihre Websites, Apps, Zeiten und ihr Schloss werden entfernt.
rule-invalid = „{ $rule }“ kann nicht verwendet werden: { $problem }

web-blocked = Gesperrte Websites
web-placeholder = example.com, example.com/page, contains:Wort, search:Wörter
web-syntax = example.com sperrt die Website mit allen Subdomains. example.com/videos sperrt nur diese Seiten. contains:casino sperrt jede Adresse, die das Wort enthält. search:kostenlose Filme sperrt Suchen, die diese Wörter enthalten.
web-allowed = Erlaubte Websites
web-exceptions = Ausnahmen (immer erlaubt)
web-safesearch = SafeSearch erzwingen (Google, Bing, DuckDuckGo, YouTube)

import-done = Import abgeschlossen
import-result = { $added } Regeln hinzugefügt, { $skipped } Zeilen übersprungen.
import-added = Hinzugefügt: { $name }
import-replaced = Aktualisiert: { $name }
import-rejected = Nicht importiert: { $name } ({ $why })

apps-blocked = Gesperrte Apps
apps-exceptions = Ausnahmen (immer erlaubt)
apps-allowed = Erlaubte Apps
apps-placeholder = steam, /opt/games/, title:YouTube, cmd:minecraft
apps-placeholder-windows = steam, C:\Games\, title:YouTube, cmd:minecraft
apps-syntax = Ein Name sperrt das Programm mit diesem Namen. Ein Ordner, der auf / endet, sperrt alle Programme darin. title:Text schließt Fenster, deren Titel den Text enthält. cmd:Text sperrt Programme, deren Befehlszeile ihn enthält. Umbenannte Kopien werden mit hash:-Regeln erkannt.
apps-syntax-windows = Ein Name sperrt das Programm mit diesem Namen. Ein Ordner, der auf \ endet, sperrt alle Programme darin. title:Text schließt Fenster, deren Titel den Text enthält. cmd:Text sperrt Programme, deren Befehlszeile ihn enthält. Umbenannte Kopien werden mit hash:-Regeln erkannt.

users-applies-to = Für wen diese Sperre gilt
users-all = Alle
users-all-except = Alle außer…
users-only = Nur…
users-except-these = Außer diesen Personen
users-only-these = Nur diese Personen
users-note = Administratoren dieses Computers werden ebenfalls gesperrt, sofern sie hier nicht ausgenommen sind. Das System selbst wird nie gesperrt.

day-mon = Mo
day-tue = Di
day-wed = Mi
day-thu = Do
day-fri = Fr
day-sat = Sa
day-sun = So
time-from = von
time-to = bis
time-at = um

schedule-limited = Nur zu bestimmten Zeiten
schedule-limited-detail = Die Sperre gilt nur während der Zeiten unten.
schedule-always = Die Sperre gilt immer.
schedule-times = Zeiten
schedule-date-placeholder = Einmalig: 2026-10-20 08:00 2026-10-22 18:00
schedule-date-invalid = Schreiben Sie einen einmaligen Zeitraum so: Startdatum, Startzeit, Enddatum, Endzeit (2026-10-20 08:00 2026-10-22 18:00).
schedule-add-weekly = Wöchentliche Zeiten hinzufügen
schedule-no-days = Wählen Sie mindestens einen Tag.
allowance = Tägliches Zeitkontingent
allowance-detail = Erlaubt die gesperrten Websites und Apps jeden Tag für eine begrenzte Zeit; danach sind sie gesperrt.
allowance-minutes = Minuten pro Tag

area-circle = im Umkreis von { $radius } m um { $lat }, { $lon }
area-polygon = in einem Bereich mit { $count } Ecken
lock-desc-until = Abgeschlossen bis { $time }
lock-desc-schedule = Abgeschlossen während { $times }
lock-desc-password = Mit Passwort abgeschlossen
lock-desc-random = Wird durch Eintippen von { $length } zufälligen Zeichen geöffnet
lock-desc-restart = Abgeschlossen bis zum Neustart des Computers
lock-desc-location = Abgeschlossen, solange dieser Computer { $area } ist
lock-desc-delay = öffnet { $minutes } Minuten nach der Antwort

lock-none = Nicht abgeschlossen
lock-none-detail = Ohne Schloss kann diese Sperre jederzeit geändert oder ausgeschaltet werden. Mit Schloss kann sie bis zum Ende nur verschärft werden.
lock-active = Abgeschlossen
lock-inactive = Schloss eingerichtet, derzeit nicht aktiv
lock-pending = Das Schloss öffnet sich am { $time }.
location-learn-detail = Iqal kennt diesen Ort noch nicht. Wenn dieser Computer dort ist, drücken Sie die Schaltfläche, damit Iqal sich die WLAN-Netze dort merkt. Bis dahin bleibt die Sperre überall abgeschlossen.
location-learned = Ort gemerkt
location-learned-detail = Für diesen Ort sind jetzt { $count } WLAN-Netze gespeichert.
location-none-chosen = Noch kein Bereich gewählt.

lock-kind-duration = Für eine Weile
lock-kind-until = Bis zu einem Datum
lock-kind-password = Mit einem Passwort
lock-kind-random = Durch Eintippen von Zufallstext
lock-kind-restart = Bis zum Neustart des Computers
lock-kind-schedule = Zu bestimmten Zeiten
lock-kind-location = Solange ich an einem Ort bin
lock-dialog-title = Diese Sperre abschließen
unit-days = Tage
unit-hours = Stunden
unit-minutes = Minuten
unit-metres = Meter
password = Passwort
password-repeat = Passwort wiederholen
password-show = Passwort anzeigen
password-generate = Ein Passwort für jemand anderen erstellen
password-generate-detail = Wenn Sie auf „Abschließen“ drücken, erstellt Iqal ein Passwort und zeigt es einmal an. Geben Sie es jemandem, dem Sie vertrauen.
password-mismatch = Die Passwörter sind leer oder stimmen nicht überein.
password-generated = Das Passwort des Schlosses
password-generated-detail = { $password }

    Geben Sie es einer Person Ihres Vertrauens. Es wird nicht noch einmal angezeigt.
random-length = Einzutippende Zeichen
lock-restart-detail = Die Sperre bleibt abgeschlossen, bis dieser Computer neu gestartet wird.
lock-location-detail = Die Sperre bleibt abgeschlossen, solange dieser Computer im Bereich ist, und auch immer dann, wenn Iqal nicht erkennen kann, wo er ist.
unlock-delay = Wartezeit nach dem Entsperren
unlock-delay-detail = Nach dem richtigen Passwort oder Text bleibt die Sperre noch so viele Minuten abgeschlossen. So hat ein plötzlicher Drang Zeit, vorüberzugehen.
lock-duration-zero = Wählen Sie eine Dauer.
lock-until-past = Dieser Zeitpunkt liegt in der Vergangenheit.
lock-confirm = Diese Sperre abschließen?
lock-confirm-detail = { $lock }. Bis dahin kann in dieser Sperre nichts entfernt, gelockert oder ausgeschaltet werden.

unlock-not-possible = Dieses Schloss kann nicht vorzeitig geöffnet werden
unlock-not-possible-detail = Es öffnet sich von selbst, wenn seine Zeit abgelaufen ist oder wenn Sie den Ort verlassen.
unlock-title = Entsperren
unlock-type-text = Tippen Sie diesen Text genau ab (Einfügen ist nicht möglich):
unlock-enter-password = Passwort des Schlosses eingeben:
unlocked = Entsperrt
unlocked-detail = Die Sperre ist jetzt aus. Sie können sie ändern oder wieder einschalten.
unlock-pending = Entsperren gestartet

map-title = Bereich wählen
map-help = Klicken Sie auf die Karte, um die Mitte des Bereichs festzulegen, und stellen Sie dann seine Größe ein. Ziehen verschiebt die Karte, das Mausrad zoomt.
map-radius = Radius

new-block-name = Name der Sperre
new-block-placeholder = Zum Beispiel: Soziale Medien
export-done = Exportiert

activity-empty = Bisher wurde nichts gesperrt.
event-app-start = App gestoppt
event-app-killed = App geschlossen
event-window-closed = Fenster geschlossen
event-site-blocked = Website gesperrt
event-page-blocked = Seite gesperrt

settings-language = Sprache
settings-language-system = Wie das System
settings-theme = Darstellung
settings-theme-system = Wie Windows
settings-theme-light = Hell
settings-theme-dark = Dunkel
settings-language-restart = Starten Sie Iqal neu, um die neue Sprache zu verwenden.
settings-hardened = Verschärfter Modus
settings-hardened-detail = Solange eine Sperre abgeschlossen ist, können die betroffenen Personen weder sudo noch Administrator-Passwortabfragen nutzen und Iqal daher nicht stoppen. Notzugang: im Wiederherstellungsmodus neu starten. Kann nicht ausgeschaltet werden, solange etwas abgeschlossen ist.
settings-status = Status
status-service = Iqal-Dienst { $version }
status-time = Zeit: { $source }
status-location = Ort: { $state }
status-extensions = Browser-Erweiterungen: { $list }
status-no-extensions = keine verbunden
status-not-admin = Sie können die Sperren ansehen, aber nicht ändern (nur Administratoren).

event-allowance-low = Zeitkontingent fast aufgebraucht
event-allowance-used = Zeitkontingent aufgebraucht

web-lists = Fertige Listen
web-lists-detail = Fertige Sammlungen von Websites. Erwachseneninhalte, Glücksspiel und soziale Medien werden täglich aus einer großen Community-Liste aktualisiert.
list-social = Soziale Medien
list-adult = Inhalte für Erwachsene
list-gambling = Glücksspiel
list-search = Suchmaschinen
list-news = Nachrichten
list-video = Video und Streaming
list-games = Spiele
list-shopping = Einkaufen
list-count = { $count ->
    [one] { $count } Website
   *[other] { $count } Websites
}
allowance-used-today = Heute genutzt: { $used } von { $total } Minuten

menu-stats = Statistik
stats-7 = Letzte 7 Tage
stats-30 = Letzte 30 Tage
stats-90 = Letzte 90 Tage
stats-private = Die Statistik bleibt auf diesem Computer.
stats-top = Am häufigsten gesperrt
stats-by-block = Nach Sperre
stats-allowance = Genutztes Zeitkontingent
stats-minutes = { $minutes } Min.
stats-empty = In diesem Zeitraum wurde nichts gesperrt.

notify-app-blocked = App gesperrt
notify-app-blocked-body = { $app } wurde von „{ $block }“ gestoppt.
notify-app-closed = App geschlossen
notify-app-closed-body = { $app } wurde von „{ $block }“ geschlossen.
notify-window-closed = Fenster geschlossen
notify-window-closed-body = „{ $title }“ wurde von „{ $block }“ geschlossen.
notify-allowance-low = Noch 5 Minuten
notify-allowance-low-body = Die tägliche Zeit für „{ $block }“ ist fast aufgebraucht.
notify-allowance-used = Die Zeit ist um
notify-allowance-used-body = Die tägliche Zeit für „{ $block }“ ist aufgebraucht. Bis morgen ist sie gesperrt.

# Saving
unsaved-changes = Ungespeicherte Änderungen
action-save = Speichern
action-discard = Verwerfen
confirm-unsaved = Änderungen an „{ $name }“ speichern?
confirm-unsaved-detail = Wenn du nicht speicherst, gehen deine Änderungen verloren.
save-has-errors = Korrigiere diese Zeilen vor dem Speichern
line-invalid = Zeile { $line }: „{ $rule }“ kann nicht verwendet werden: { $problem }
lines-invalid-more = ({ $count } weitere Zeilen müssen korrigiert werden)
list-empty-hint = Ein Eintrag pro Zeile, zum Beispiel: { $examples }
lock-saves-changes = Deine anderen ungespeicherten Änderungen an dieser Sperre werden ebenfalls gespeichert.

# Statistics window
stats-today = Heute
stats-chart = Sperrungen pro Tag
stats-card-total = Gesperrt
stats-card-websites = Gestoppte Websites
stats-card-apps = Gestoppte Apps
stats-card-busiest = Stärkster Tag
stats-card-top = Am häufigsten gesperrt
stats-before = { $days ->
    [1] Gestern: { $count }
   *[other] Vorherige { $days } Tage: { $count }
}
stats-blocks = { $count ->
    [one] { $count } Sperrung
   *[other] { $count } Sperrungen
}
stats-distinct-sites = { $count ->
    [one] { $count } Website
   *[other] { $count } verschiedene Websites
}
stats-distinct-apps = { $count ->
    [one] { $count } App
   *[other] { $count } verschiedene Apps
}
stats-show-all = Alle anzeigen ({ $count })
stats-show-fewer = Weniger anzeigen
stats-allowance-of = { $used } von { $total } Min.

# Rule lists
web-mode-block = Sperrliste
web-mode-allow-only = Erlaubnisliste
apps-mode-block = Sperrliste
apps-mode-allow-only = Erlaubnisliste
add-websites = Websites hinzufügen
add-apps = Apps hinzufügen
add-exceptions = Ausnahmen hinzufügen
action-remove-selected = Auswahl entfernen
search = Suchen
error-service-unreachable-detail = Installieren oder starten Sie ihn und versuchen Sie es erneut.
error-locked-title = Die Sperre ist abgeschlossen
error-wrong-password = Falsches Passwort
error-wrong-text = Der Text stimmt nicht überein
error-no-wifi = Keine WLAN-Netze gefunden. Schalten Sie WLAN ein und versuchen Sie es erneut.
location-learn-failed = Der Ort konnte nicht gespeichert werden
location-known = Iqal erkennt diesen Ort an den WLAN-Netzen in der Nähe und weiß selbst, wann dieser Computer dort ist.
location-here-question = Ist dieser Computer jetzt an diesem Ort?
location-here-yes = Ja, er ist jetzt dort
location-here-no = Nein
location-here-detail = Bei Ja merkt sich Iqal die WLAN-Netze in der Nähe und weiß von da an selbst, wann der Computer ankommt und geht. Bei Nein drücken Sie „Ich bin jetzt an diesem Ort“ im Tab „Schloss“, sobald Sie dort sind.
location-here-unanswered = Wählen Sie, ob dieser Computer jetzt an dem Ort ist.
lock-confirm-timer = Wenn die Zeit abgelaufen ist, wird auch die Sperre ausgeschaltet.
schedule-explain = Zeiten legen fest, wann diese Sperre blockiert. Ein Schloss legt fest, wann die Sperre nicht geändert oder ausgeschaltet werden kann. Beispiel: Sperren Sie hier soziale Medien werktags von 9:00 bis 17:00 und fügen Sie ein Passwort-Schloss hinzu, damit die Zeiten nicht gelockert werden können. Ein Schloss „Für eine Weile“ oder „Bis zu einem Datum“ schaltet die Sperre aus, wenn es endet.

# Windows: a place lock by name (no map)
location-name = Name des Ortes
location-name-placeholder = Zum Beispiel: Zuhause
area-named = bei „{ $name }“
action-close = Schließen

lock-location-detail-named = Die Sperre bleibt gesperrt, solange dieser Computer an diesem Ort ist, und auch immer dann, wenn Iqal nicht erkennen kann, wo er ist.
time-source-network = Netzwerkzeit
time-source-system = synchronisierte Systemzeit
time-source-counted = gezählt ab der letzten vertrauenswürdigen Zeit (offline)
tray-open = Iqal öffnen

setup-window-title = Iqal-Installation
setup-install-title = Iqal { $version } installieren?
setup-install-detail = Iqal wird in { $folder } installiert. Es läuft im Hintergrund als Windows-Dienst und sperrt die Websites und Apps, die du auswählst.
setup-update-title = Iqal auf { $version } aktualisieren?
setup-update-detail = Deine Sperren bleiben erhalten.
action-install = Installieren
action-update = Aktualisieren
setup-installed = Iqal ist installiert
setup-installed-detail = Öffne Iqal über das Startmenü, um deine erste Sperre anzulegen.
setup-failed = Iqal konnte nicht installiert werden
setup-locked = Iqal kann jetzt nicht aktualisiert werden
setup-locked-detail = Eine Sperre ist abgeschlossen. Iqal kann aktualisiert werden, wenn ihr Schloss endet.
uninstall-title = Iqal entfernen?
uninstall-detail = Der Dienst, seine Browser-Einstellungen und die Programme werden entfernt. Deine Sperren bleiben in { $folder }, falls du Iqal wieder installierst.
uninstall-action = Entfernen
uninstall-done = Iqal wurde entfernt
uninstall-failed = Iqal konnte nicht entfernt werden
uninstall-locked = Iqal kann jetzt nicht entfernt werden
uninstall-locked-detail = Eine Sperre ist abgeschlossen. Iqal kann entfernt werden, wenn alle Schlösser geendet haben.
setup-updated = Iqal ist aktualisiert
