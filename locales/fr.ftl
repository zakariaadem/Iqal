# Iqal — Français

app-name = Iqal Blocker
app-description = Bloqueur de sites web et d’applications
menu = Menu
menu-import = Importer des blocs…
menu-export-all = Exporter tous les blocs…
menu-activity = Activité récente
menu-settings = Paramètres
menu-about = À propos d’Iqal

action-new-block = Nouveau bloc
action-remove = Retirer
action-add = Ajouter
action-cancel = Annuler
action-delete = Supprimer
action-create = Créer
action-export-block = Exporter ce bloc
action-delete-block = Supprimer ce bloc
action-import-text = Importer depuis un fichier texte…
action-add-installed-app = Ajouter une application installée…
action-add-program-file = Ajouter un fichier de programme…
action-add-window-title = Ajouter un titre de fenêtre…
action-add-times = Ajouter ces horaires
action-cancel-unlock = Annuler le déverrouillage
action-learn-location = Je suis à cet endroit maintenant
action-unlock = Déverrouiller…
action-remove-lock = Retirer le verrou
action-change-lock = Changer le verrou…
action-lock = Verrouiller
action-choose-on-map = Choisir sur la carte…
action-use-area = Utiliser cette zone

list-empty = Rien pour l’instant.

state-off = Désactivé
state-active = Actif
state-scheduled = Programmé
state-locked = Verrouillé

error-service-unreachable = Le service Iqal ne fonctionne pas
error-unexpected = Le service Iqal a donné une réponse inattendue.
error-locked = Cette modification { $reason }. Un bloc verrouillé ne peut être que renforcé.
error-wrong-answer = Mauvaise réponse
error-cannot-unlock-early = Ce verrou ne peut pas être ouvert plus tôt.
error-not-admin = Seuls les administrateurs peuvent modifier les réglages d’Iqal.

reason-disables = le désactiverait
reason-fewer-users = l’appliquerait à moins de personnes
reason-web-mode = changerait la façon de bloquer les sites
reason-removes-websites = retirerait des sites bloqués
reason-adds-allowed-websites = autoriserait davantage de sites
reason-safesearch-off = désactiverait SafeSearch
reason-app-mode = changerait la façon de bloquer les applications
reason-removes-apps = retirerait des applications bloquées
reason-adds-allowed-apps = autoriserait davantage d’applications
reason-adds-schedule = le limiterait à certains horaires
reason-shortens-schedule = raccourcirait ses horaires
reason-adds-allowance = ajouterait un temps autorisé
reason-increases-allowance = augmenterait le temps autorisé
reason-removes-lock = retirerait le verrou
reason-shortens-delay = raccourcirait l’attente avant déverrouillage
reason-weakens-lock = affaiblirait le verrou
reason-pending-unlock = modifierait un déverrouillage en attente
reason-deletes = le supprimerait
reason-hardened-off = désactiverait le mode renforcé

empty-title = Aucun bloc pour l’instant
empty-detail = Un bloc est un ensemble de sites et d’applications à bloquer, avec ses propres horaires et son verrou. Créez-en un avec « Nouveau bloc ».

block-summary = Règles de sites : { $web } · Règles d’applications : { $apps }
rules-count = { $count ->
    [one] { $count } règle
   *[other] { $count } règles
}

locked-banner = Ce bloc est verrouillé. Vous pouvez ajouter des restrictions, mais pas en retirer.
tab-websites = Sites web
tab-apps = Applications
tab-users = Personnes
tab-schedule = Horaires
tab-lock = Verrou
block-enabled = Activer ou désactiver le bloc
block-switch-on = Le bloc est activé
block-switch-off = Le bloc est désactivé
confirm-delete = Supprimer « { $name } » ?
confirm-delete-detail = Ses sites, applications, horaires et son verrou seront supprimés.
rule-invalid = « { $rule } » ne peut pas être utilisé : { $problem }

web-blocked = Sites bloqués
web-placeholder = example.com, example.com/page, contains:mot, search:mots
web-syntax = example.com bloque le site et tous ses sous-domaines. example.com/videos bloque seulement ces pages. contains:casino bloque toute adresse contenant le mot. search:films gratuits bloque les recherches contenant ces mots.
web-allowed = Sites autorisés
web-exceptions = Exceptions (toujours autorisées)
web-safesearch = Imposer SafeSearch (Google, Bing, DuckDuckGo, YouTube)

import-done = Importation terminée
import-result = { $added } règles ajoutées, { $skipped } lignes ignorées.
import-added = Ajouté : { $name }
import-replaced = Mis à jour : { $name }
import-rejected = Non importé : { $name } ({ $why })

apps-blocked = Applications bloquées
apps-exceptions = Exceptions (toujours autorisées)
apps-allowed = Applications autorisées
apps-placeholder = steam, /opt/games/, title:YouTube, cmd:minecraft
apps-placeholder-windows = steam, C:\Games\, title:YouTube, cmd:minecraft
apps-syntax = Un nom bloque le programme portant ce nom. Un dossier terminé par / bloque tous les programmes qu’il contient. title:Texte ferme les fenêtres dont le titre contient le texte. cmd:Texte bloque les programmes dont la ligne de commande le contient. Les copies renommées sont repérées avec les règles hash:.
apps-syntax-windows = Un nom bloque le programme portant ce nom. Un dossier terminé par \ bloque tous les programmes qu’il contient. title:Texte ferme les fenêtres dont le titre contient le texte. cmd:Texte bloque les programmes dont la ligne de commande le contient. Les copies renommées sont repérées avec les règles hash:.
window-title-intro = Iqal ferme toute fenêtre dont le titre contient ces mots, quel que soit son programme. Cela convient à ce qui n’a pas de programme propre, comme un jeu dans un lanceur. Pour les sites web, utilisez plutôt l’onglet Sites web : une fenêtre de navigateur serait fermée avec tous ses onglets.
window-title-words = Mots du titre de la fenêtre
window-title-open = Ou choisissez une fenêtre ouverte maintenant
window-title-empty = Saisissez des mots du titre ou choisissez une fenêtre.

users-applies-to = À qui s’applique ce bloc
users-all = Tout le monde
users-all-except = Tout le monde sauf…
users-only = Seulement…
users-except-these = Sauf ces personnes
users-only-these = Seulement ces personnes
users-note = Les administrateurs de cet ordinateur sont aussi bloqués, sauf s’ils sont exclus ici. Le système lui-même n’est jamais bloqué.

day-mon = lun.
day-tue = mar.
day-wed = mer.
day-thu = jeu.
day-fri = ven.
day-sat = sam.
day-sun = dim.
time-from = de
time-to = à
time-at = à

schedule-limited = Seulement à certains horaires
schedule-limited-detail = Le bloc ne s’applique que pendant les horaires ci-dessous.
schedule-always = Le bloc s’applique en permanence.
schedule-times = Horaires
schedule-date-placeholder = Ponctuel : 2026-10-20 08:00 2026-10-22 18:00
schedule-date-invalid = Écrivez une période ponctuelle ainsi : date de début, heure de début, date de fin, heure de fin (2026-10-20 08:00 2026-10-22 18:00).
schedule-add-weekly = Ajouter des horaires hebdomadaires
schedule-no-days = Choisissez au moins un jour.
allowance = Temps autorisé par jour
allowance-detail = Autorise les sites et applications bloqués pendant un temps limité chaque jour ; ensuite, ils sont bloqués.
allowance-minutes = Minutes par jour

area-circle = à moins de { $radius } m de { $lat }, { $lon }
area-polygon = dans une zone à { $count } sommets
lock-desc-until = Verrouillé jusqu’au { $time }
lock-desc-schedule = Verrouillé pendant { $times }
lock-desc-password = Verrouillé par un mot de passe
lock-desc-random = Se déverrouille en tapant { $length } caractères aléatoires
lock-desc-restart = Verrouillé jusqu’au redémarrage de l’ordinateur
lock-desc-location = Verrouillé tant que cet ordinateur est { $area }
lock-desc-delay = s’ouvre { $minutes } minutes après la réponse

lock-none = Non verrouillé
lock-none-detail = Sans verrou, ce bloc peut être modifié ou désactivé à tout moment. Avec un verrou, il ne peut être que renforcé jusqu’à la fin du verrou.
lock-active = Verrouillé
lock-inactive = Verrou défini, inactif pour le moment
lock-pending = Le verrou s’ouvre le { $time }.
location-learn-detail = Iqal ne connaît pas encore cet endroit. Quand cet ordinateur y est, appuyez sur le bouton pour qu’Iqal retienne les réseaux Wi-Fi autour. D’ici là, le bloc reste verrouillé où que vous soyez.
location-learned = Endroit mémorisé
location-learned-detail = { $count } réseaux Wi-Fi sont maintenant mémorisés pour cet endroit.
location-none-chosen = Aucune zone choisie.

lock-kind-duration = Pendant un moment
lock-kind-until = Jusqu’à une date
lock-kind-password = Avec un mot de passe
lock-kind-random = En tapant un texte aléatoire
lock-kind-restart = Jusqu’au redémarrage de l’ordinateur
lock-kind-schedule = À certains horaires
lock-kind-location = Tant que je suis à un endroit
lock-dialog-title = Verrouiller ce bloc
unit-days = jours
unit-hours = heures
unit-minutes = minutes
unit-metres = mètres
password = Mot de passe
password-repeat = Répétez le mot de passe
password-show = Afficher le mot de passe
password-generate = Créer un mot de passe que quelqu’un d’autre gardera
password-generate-detail = Quand vous appuyez sur « Verrouiller », Iqal crée un mot de passe et l’affiche une seule fois. Donnez-le à quelqu’un de confiance.
password-mismatch = Les mots de passe sont vides ou ne correspondent pas.
password-generated = Le mot de passe du verrou
password-generated-detail = { $password }

    Confiez-le à une personne de confiance. Il ne sera plus affiché.
random-length = Caractères à taper
lock-restart-detail = Le bloc reste verrouillé jusqu’au redémarrage de cet ordinateur.
lock-location-detail = Le bloc reste verrouillé tant que cet ordinateur est dans la zone, et aussi chaque fois qu’Iqal ne peut pas savoir où il se trouve.
unlock-delay = Attente après déverrouillage
unlock-delay-detail = Après le bon mot de passe ou le bon texte, le bloc reste verrouillé encore ce nombre de minutes. Cela laisse le temps à une envie soudaine de passer.
lock-duration-zero = Choisissez une durée.
lock-until-past = Ce moment est déjà passé.
lock-confirm = Verrouiller ce bloc ?
lock-confirm-detail = { $lock }. D’ici là, rien dans ce bloc ne peut être retiré, assoupli ou désactivé.

unlock-not-possible = Ce verrou ne peut pas être ouvert plus tôt
unlock-not-possible-detail = Il s’ouvre tout seul quand son temps est écoulé, ou quand vous quittez l’endroit.
unlock-title = Déverrouiller
unlock-type-text = Tapez ce texte exactement (le collage est impossible) :
unlock-enter-password = Saisissez le mot de passe du verrou :
unlocked = Déverrouillé
unlocked-detail = Le bloc est désactivé. Vous pouvez le modifier ou le réactiver.
unlock-pending = Déverrouillage lancé

map-title = Choisir la zone
map-help = Cliquez sur la carte pour placer le centre de la zone, puis réglez sa taille. Faites glisser pour déplacer la carte et utilisez la molette pour zoomer.
map-radius = Rayon

new-block-name = Nom du bloc
new-block-placeholder = Par exemple : Réseaux sociaux
export-done = Exporté

activity-empty = Rien n’a encore été bloqué.
event-app-start = Application arrêtée
event-app-killed = Application fermée
event-window-closed = Fenêtre fermée
event-site-blocked = Site bloqué
event-page-blocked = Page bloquée

settings-language = Langue
settings-language-system = Comme le système
settings-theme = Apparence
settings-theme-system = Comme Windows
settings-theme-light = Clair
settings-theme-dark = Sombre
settings-language-restart = Redémarrez Iqal pour utiliser la nouvelle langue.
settings-hardened = Mode renforcé
settings-hardened-detail = Tant qu’un bloc est verrouillé, les personnes concernées ne peuvent pas utiliser sudo ni les demandes de mot de passe administrateur, et ne peuvent donc pas arrêter Iqal. Accès d’urgence : redémarrer en mode de récupération. Impossible à désactiver tant que quelque chose est verrouillé.
settings-status = État
status-service = Service Iqal { $version }
status-time = Heure : { $source }
status-location = Position : { $state }
status-extensions = Extensions de navigateur : { $list }
status-no-extensions = aucune connectée
status-not-admin = Vous pouvez voir les blocs mais pas les modifier (réservé aux administrateurs).

event-allowance-low = Temps autorisé presque écoulé
event-allowance-used = Temps autorisé écoulé

web-lists = Listes prêtes à l’emploi
web-lists-detail = Des ensembles de sites tout prêts. Les listes de contenu adulte, de jeux d’argent et de réseaux sociaux sont mises à jour chaque jour à partir d’une grande liste communautaire.
list-social = Réseaux sociaux
list-adult = Contenu pour adultes
list-gambling = Jeux d’argent
list-search = Moteurs de recherche
list-news = Actualités
list-video = Vidéo et streaming
list-games = Jeux
list-shopping = Achats
list-count = { $count ->
    [one] { $count } site
   *[other] { $count } sites
}
allowance-used-today = Utilisé aujourd’hui : { $used } sur { $total } minutes

menu-stats = Statistiques
stats-7 = 7 derniers jours
stats-30 = 30 derniers jours
stats-90 = 90 derniers jours
stats-private = Les statistiques restent sur cet ordinateur.
stats-top = Les plus bloqués
stats-by-block = Par bloc
stats-allowance = Temps autorisé utilisé
stats-minutes = { $minutes } min
stats-empty = Rien n’a été bloqué sur cette période.

notify-app-blocked = Application bloquée
notify-app-blocked-body = { $app } a été arrêtée par « { $block } ».
notify-app-closed = Application fermée
notify-app-closed-body = { $app } a été fermée par « { $block } ».
notify-window-closed = Fenêtre fermée
notify-window-closed-body = « { $title } » a été fermée par « { $block } ».
notify-allowance-low = Plus que 5 minutes
notify-allowance-low-body = Le temps quotidien de « { $block } » est presque écoulé.
notify-allowance-used = Le temps est écoulé
notify-allowance-used-body = Le temps quotidien de « { $block } » est écoulé. Il est bloqué jusqu’à demain.

# Saving
unsaved-changes = Modifications non enregistrées
action-save = Enregistrer
action-discard = Annuler les modifications
confirm-unsaved = Enregistrer les modifications de « { $name } » ?
confirm-unsaved-detail = Si vous n’enregistrez pas, vos modifications seront perdues.
save-has-errors = Corrigez ces lignes avant d’enregistrer
line-invalid = Ligne { $line } : « { $rule } » ne peut pas être utilisé : { $problem }
lines-invalid-more = ({ $count } autres lignes à corriger)
list-empty-hint = Une entrée par ligne, par exemple : { $examples }
lock-saves-changes = Vos autres modifications non enregistrées de ce bloc seront aussi enregistrées.

# Statistics window
stats-today = Aujourd’hui
stats-chart = Blocages par jour
stats-card-total = Blocages
stats-card-websites = Sites arrêtés
stats-card-apps = Applications arrêtées
stats-card-busiest = Jour le plus chargé
stats-card-top = Le plus bloqué
stats-before = { $days ->
    [1] Hier : { $count }
   *[other] { $days } jours précédents : { $count }
}
stats-blocks = { $count ->
    [one] { $count } blocage
   *[other] { $count } blocages
}
stats-distinct-sites = { $count ->
    [one] { $count } site
   *[other] { $count } sites différents
}
stats-distinct-apps = { $count ->
    [one] { $count } application
   *[other] { $count } applications différentes
}
stats-show-all = Tout afficher ({ $count })
stats-show-fewer = Afficher moins
stats-allowance-of = { $used } sur { $total } min

# Rule lists
web-mode-block = Liste de blocage
web-mode-allow-only = Liste d’autorisation
apps-mode-block = Liste de blocage
apps-mode-allow-only = Liste d’autorisation
add-websites = Ajouter des sites
add-apps = Ajouter des applications
add-exceptions = Ajouter des exceptions
action-remove-selected = Retirer la sélection
search = Rechercher
error-service-unreachable-detail = Installez-le ou démarrez-le, puis réessayez.
error-locked-title = Le bloc est verrouillé
error-wrong-password = Mot de passe incorrect
error-wrong-text = Le texte ne correspond pas
error-no-wifi = Aucun réseau Wi-Fi trouvé. Activez le Wi-Fi et réessayez.
location-learn-failed = Impossible de retenir l’endroit
location-known = Iqal reconnaît cet endroit grâce aux réseaux Wi-Fi autour et sait de lui-même quand cet ordinateur s’y trouve.
location-here-question = Cet ordinateur est-il à cet endroit maintenant ?
location-here-yes = Oui, il y est maintenant
location-here-no = Non
location-here-detail = Si oui, Iqal retient les réseaux Wi-Fi autour et sait ensuite de lui-même quand l’ordinateur arrive et repart. Si non, appuyez sur « Je suis à cet endroit maintenant » dans l’onglet Verrou une fois sur place.
location-here-unanswered = Indiquez si cet ordinateur est à cet endroit maintenant.
lock-confirm-timer = À la fin du temps, le bloc s’arrête aussi.
schedule-explain = Les horaires décident quand ce bloc bloque. Un verrou décide quand le bloc ne peut pas être modifié ni arrêté. Par exemple : bloquez ici les réseaux sociaux en semaine de 9 h à 17 h, et ajoutez un verrou par mot de passe pour que les horaires ne puissent pas être assouplis. Un verrou « Pendant un moment » ou « Jusqu’à une date » arrête le bloc quand il se termine.

# Windows: a place lock by name (no map)
location-name = Nom du lieu
location-name-placeholder = Par exemple : Maison
area-named = à « { $name } »
action-close = Fermer

lock-location-detail-named = Le blocage reste verrouillé tant que cet ordinateur est à cet endroit, et aussi chaque fois qu’Iqal ne peut pas savoir où il se trouve.
time-source-network = heure du réseau
time-source-system = heure système synchronisée
time-source-counted = comptée depuis la dernière heure fiable (hors ligne)
tray-open = Ouvrir Iqal

setup-window-title = Installation d’Iqal Blocker
setup-install-title = Installer Iqal Blocker { $version } ?
setup-install-detail = Iqal sera installé dans { $folder }. Il fonctionne en arrière-plan comme service Windows et bloque les sites et les applications que vous choisissez.
setup-update-title = Mettre Iqal Blocker à jour vers { $version } ?
setup-update-detail = Vos blocs sont conservés.
action-install = Installer
action-update = Mettre à jour
setup-installed = Iqal Blocker est installé
setup-installed-detail = Ouvrez Iqal Blocker depuis le menu Démarrer pour créer votre premier bloc.
setup-failed = Iqal n’a pas pu être installé
setup-locked = Iqal ne peut pas être mis à jour maintenant
setup-locked-detail = Un bloc est verrouillé. Iqal pourra être mis à jour quand son verrou aura pris fin.
uninstall-title = Supprimer Iqal Blocker ?
uninstall-detail = Le service, ses réglages des navigateurs et les programmes sont supprimés. Vos blocs restent dans { $folder }, au cas où vous réinstalleriez Iqal.
uninstall-action = Supprimer
uninstall-done = Iqal Blocker a été supprimé
uninstall-failed = Iqal n’a pas pu être supprimé
uninstall-locked = Iqal ne peut pas être supprimé maintenant
uninstall-locked-detail = Un bloc est verrouillé. Iqal pourra être supprimé quand tous les verrous auront pris fin.
setup-updated = Iqal est à jour
