# Iqal — Español

app-name = Iqal
app-description = Bloqueador de sitios web y aplicaciones
menu = Menú
menu-import = Importar bloques…
menu-export-all = Exportar todos los bloques…
menu-activity = Actividad reciente
menu-settings = Ajustes
menu-about = Acerca de Iqal

action-new-block = Nuevo bloque
action-remove = Quitar
action-add = Añadir
action-cancel = Cancelar
action-delete = Eliminar
action-create = Crear
action-export-block = Exportar este bloque
action-delete-block = Eliminar este bloque
action-import-text = Importar desde un archivo de texto…
action-add-installed-app = Añadir una aplicación instalada…
action-add-program-file = Añadir un archivo de programa…
action-add-times = Añadir estos horarios
action-cancel-unlock = Cancelar el desbloqueo
action-learn-location = Estoy en este lugar ahora
action-unlock = Desbloquear…
action-remove-lock = Quitar el bloqueo
action-change-lock = Cambiar el bloqueo…
action-lock = Bloquear
action-choose-on-map = Elegir en el mapa…
action-use-area = Usar esta zona

list-empty = Todavía no hay nada.

state-off = Desactivado
state-active = Activo
state-scheduled = Programado
state-locked = Bloqueado

error-service-unreachable = El servicio de Iqal no está funcionando
error-unexpected = El servicio Iqal dio una respuesta inesperada.
error-locked = Este cambio { $reason }. Un bloque bloqueado solo puede hacerse más estricto.
error-wrong-answer = Respuesta incorrecta
error-cannot-unlock-early = Este bloqueo no se puede abrir antes de tiempo.
error-not-admin = Solo los administradores pueden cambiar los ajustes de Iqal.

reason-disables = lo desactivaría
reason-fewer-users = lo aplicaría a menos personas
reason-web-mode = cambiaría la forma de bloquear sitios
reason-removes-websites = quitaría sitios bloqueados
reason-adds-allowed-websites = permitiría más sitios
reason-safesearch-off = desactivaría SafeSearch
reason-app-mode = cambiaría la forma de bloquear aplicaciones
reason-removes-apps = quitaría aplicaciones bloqueadas
reason-adds-allowed-apps = permitiría más aplicaciones
reason-adds-schedule = lo limitaría a ciertos horarios
reason-shortens-schedule = acortaría sus horarios
reason-adds-allowance = añadiría un tiempo permitido
reason-increases-allowance = aumentaría el tiempo permitido
reason-removes-lock = quitaría el bloqueo
reason-shortens-delay = acortaría la espera antes del desbloqueo
reason-weakens-lock = debilitaría el bloqueo
reason-pending-unlock = cambiaría un desbloqueo pendiente
reason-deletes = lo eliminaría
reason-hardened-off = desactivaría el modo reforzado

empty-title = Todavía no hay bloques
empty-detail = Un bloque es un grupo de sitios web y aplicaciones que bloquear, con sus propios horarios y su bloqueo. Crea uno con «Nuevo bloque».

block-summary = Reglas de sitios: { $web } · Reglas de aplicaciones: { $apps }
rules-count = { $count ->
    [one] { $count } regla
   *[other] { $count } reglas
}

locked-banner = Este bloque está bloqueado. Puedes añadir restricciones, pero no quitarlas.
tab-websites = Sitios web
tab-apps = Aplicaciones
tab-users = Personas
tab-schedule = Horarios
tab-lock = Bloqueo
block-enabled = Activar o desactivar el bloque
confirm-delete = ¿Eliminar «{ $name }»?
confirm-delete-detail = Se quitarán sus sitios, aplicaciones, horarios y bloqueo.
rule-invalid = «{ $rule }» no se puede usar: { $problem }

web-blocked = Sitios bloqueados
web-placeholder = example.com, example.com/page, contains:palabra, search:palabras
web-syntax = example.com bloquea el sitio y todos sus subdominios. example.com/videos bloquea solo esas páginas. contains:casino bloquea cualquier dirección que contenga la palabra. search:películas gratis bloquea las búsquedas que contengan esas palabras.
web-allowed = Sitios permitidos
web-exceptions = Excepciones (siempre permitidas)
web-safesearch = Forzar SafeSearch (Google, Bing, DuckDuckGo, YouTube)

import-done = Importación terminada
import-result = { $added } reglas añadidas, { $skipped } líneas omitidas.
import-added = Añadido: { $name }
import-replaced = Actualizado: { $name }
import-rejected = No importado: { $name } ({ $why })

apps-blocked = Aplicaciones bloqueadas
apps-exceptions = Excepciones (siempre permitidas)
apps-allowed = Aplicaciones permitidas
apps-placeholder = steam, /opt/games/, title:YouTube, cmd:minecraft
apps-syntax = Un nombre bloquea el programa con ese nombre. Una carpeta terminada en / bloquea todos los programas que contiene. title:Texto cierra las ventanas cuyo título contiene el texto. cmd:Texto bloquea los programas cuya línea de órdenes lo contiene. Las copias renombradas se detectan con reglas hash:.

users-applies-to = A quién se aplica este bloque
users-all = A todos
users-all-except = A todos excepto…
users-only = Solo a…
users-except-these = Excepto a estas personas
users-only-these = Solo a estas personas
users-note = Los administradores de este equipo también quedan bloqueados, salvo que se excluyan aquí. El sistema nunca se bloquea.

day-mon = lun
day-tue = mar
day-wed = mié
day-thu = jue
day-fri = vie
day-sat = sáb
day-sun = dom
time-from = de
time-to = a
time-at = a las

schedule-limited = Solo en ciertos horarios
schedule-limited-detail = El bloque solo se aplica durante los horarios de abajo.
schedule-always = El bloque se aplica siempre.
schedule-times = Horarios
schedule-date-placeholder = Puntual: 2026-10-20 08:00 2026-10-22 18:00
schedule-date-invalid = Escribe un periodo puntual así: fecha de inicio, hora de inicio, fecha de fin, hora de fin (2026-10-20 08:00 2026-10-22 18:00).
schedule-add-weekly = Añadir horarios semanales
schedule-no-days = Elige al menos un día.
allowance = Tiempo permitido al día
allowance-detail = Permite los sitios y aplicaciones bloqueados durante un tiempo limitado cada día; después se bloquean.
allowance-minutes = Minutos al día

area-circle = a menos de { $radius } m de { $lat }, { $lon }
area-polygon = dentro de una zona de { $count } vértices
lock-desc-until = Bloqueado hasta el { $time }
lock-desc-schedule = Bloqueado durante { $times }
lock-desc-password = Bloqueado con contraseña
lock-desc-random = Se desbloquea escribiendo { $length } caracteres aleatorios
lock-desc-restart = Bloqueado hasta que se reinicie el equipo
lock-desc-location = Bloqueado mientras este equipo esté { $area }
lock-desc-delay = se abre { $minutes } minutos después de la respuesta

lock-none = Sin bloquear
lock-none-detail = Sin bloqueo, este bloque se puede cambiar o desactivar en cualquier momento. Con un bloqueo, solo puede hacerse más estricto hasta que termine.
lock-active = Bloqueado
lock-inactive = Bloqueo definido, no activo ahora
lock-pending = El bloqueo se abre el { $time }.
location-learn-detail = Iqal aún no conoce este lugar. Cuando este ordenador esté allí, pulsa el botón para que Iqal recuerde las redes Wi-Fi de alrededor. Hasta entonces, el bloque sigue bloqueado estés donde estés.
location-learned = Lugar recordado
location-learned-detail = Ahora se recuerdan { $count } redes Wi-Fi para este lugar.
location-none-chosen = Todavía no se ha elegido ninguna zona.

lock-kind-duration = Durante un tiempo
lock-kind-until = Hasta una fecha
lock-kind-password = Con una contraseña
lock-kind-random = Escribiendo un texto aleatorio
lock-kind-restart = Hasta que se reinicie el equipo
lock-kind-schedule = En ciertos horarios
lock-kind-location = Mientras esté en un lugar
lock-dialog-title = Bloquear este bloque
unit-days = días
unit-hours = horas
unit-minutes = minutos
unit-metres = metros
password = Contraseña
password-repeat = Repite la contraseña
password-generate = Crear una contraseña para que la guarde otra persona
password-generate-detail = Al pulsar «Bloquear», Iqal crea una contraseña y la muestra una sola vez. Dásela a alguien de confianza.
password-mismatch = Las contraseñas están vacías o no coinciden.
password-generated = La contraseña del bloqueo
password-generated-detail = { $password }

    Dásela a alguien de confianza. No se volverá a mostrar.
random-length = Caracteres que escribir
lock-restart-detail = El bloque sigue bloqueado hasta que se reinicie este equipo.
lock-location-detail = El bloque sigue bloqueado mientras este equipo esté dentro de la zona, y también siempre que Iqal no pueda saber dónde está.
unlock-delay = Espera tras desbloquear
unlock-delay-detail = Tras escribir la contraseña o el texto correctos, el bloque sigue bloqueado estos minutos más. Así un impulso repentino tiene tiempo de pasar.
lock-duration-zero = Elige una duración.
lock-until-past = Ese momento ya ha pasado.
lock-confirm = ¿Bloquear este bloque?
lock-confirm-detail = { $lock }. Hasta entonces, nada de este bloque se podrá quitar, relajar ni desactivar.

unlock-not-possible = Este bloqueo no se puede abrir antes de tiempo
unlock-not-possible-detail = Se abre solo cuando termina su tiempo o cuando te vas del lugar.
unlock-title = Desbloquear
unlock-type-text = Escribe este texto exactamente (no se puede pegar):
unlock-enter-password = Introduce la contraseña del bloqueo:
unlocked = Desbloqueado
unlocked-detail = El bloque se puede cambiar de nuevo.
unlock-pending = Desbloqueo iniciado

map-title = Elegir la zona
map-help = Haz clic en el mapa para colocar el centro de la zona y luego ajusta su tamaño. Arrastra para mover el mapa y usa la rueda para acercar.
map-radius = Radio

new-block-name = Nombre del bloque
new-block-placeholder = Por ejemplo: Redes sociales
export-done = Exportado

activity-empty = Todavía no se ha bloqueado nada.
event-app-start = Aplicación detenida
event-app-killed = Aplicación cerrada
event-window-closed = Ventana cerrada
event-site-blocked = Sitio bloqueado
event-page-blocked = Página bloqueada

settings-language = Idioma
settings-language-system = Igual que el sistema
settings-language-restart = Reinicia Iqal para usar el nuevo idioma.
settings-hardened = Modo reforzado
settings-hardened-detail = Mientras haya un bloque bloqueado, las personas afectadas no pueden usar sudo ni las solicitudes de contraseña de administrador, así que no pueden detener Iqal. Acceso de emergencia: reiniciar en modo de recuperación. No se puede desactivar mientras algo esté bloqueado.
settings-status = Estado
status-service = Servicio Iqal { $version }
status-time = Hora: { $source }
status-location = Ubicación: { $state }
status-extensions = Extensiones del navegador: { $list }
status-no-extensions = ninguna conectada
status-not-admin = Puedes ver los bloques pero no cambiarlos (solo administradores).

event-allowance-low = Tiempo permitido casi agotado
event-allowance-used = Tiempo permitido agotado

web-lists = Listas preparadas
web-lists-detail = Colecciones de sitios ya preparadas. Las de contenido para adultos, apuestas y redes sociales se actualizan cada día a partir de una gran lista comunitaria.
list-social = Redes sociales
list-adult = Contenido para adultos
list-gambling = Apuestas
list-search = Buscadores
list-news = Noticias
list-video = Vídeo y streaming
list-games = Juegos
list-shopping = Compras
list-count = { $count ->
    [one] { $count } sitio
   *[other] { $count } sitios
}
allowance-used-today = Usado hoy: { $used } de { $total } minutos

menu-stats = Estadísticas
stats-7 = Últimos 7 días
stats-30 = Últimos 30 días
stats-90 = Últimos 90 días
stats-private = Las estadísticas se quedan en este equipo.
stats-top = Lo más bloqueado
stats-by-block = Por bloque
stats-allowance = Tiempo permitido usado
stats-minutes = { $minutes } min
stats-empty = No se ha bloqueado nada en este periodo.

notify-app-blocked = Aplicación bloqueada
notify-app-blocked-body = «{ $block }» detuvo { $app }.
notify-app-closed = Aplicación cerrada
notify-app-closed-body = «{ $block }» cerró { $app }.
notify-window-closed = Ventana cerrada
notify-window-closed-body = «{ $block }» cerró «{ $title }».
notify-allowance-low = Quedan 5 minutos
notify-allowance-low-body = El tiempo diario de «{ $block }» casi se ha agotado.
notify-allowance-used = Se acabó el tiempo
notify-allowance-used-body = El tiempo diario de «{ $block }» se ha agotado. Queda bloqueado hasta mañana.

# Saving
unsaved-changes = Cambios sin guardar
action-save = Guardar
action-discard = Descartar
confirm-unsaved = ¿Guardar los cambios de «{ $name }»?
confirm-unsaved-detail = Si no los guardas, perderás los cambios.
save-has-errors = Corrige estas líneas antes de guardar
line-invalid = Línea { $line }: «{ $rule }» no se puede usar: { $problem }
lines-invalid-more = ({ $count } líneas más por corregir)
list-empty-hint = Una por línea, por ejemplo: { $examples }
lock-saves-changes = También se guardarán tus otros cambios sin guardar de este bloque.

# Statistics window
stats-today = Hoy
stats-chart = Bloqueos por día
stats-card-total = Veces bloqueado
stats-card-websites = Sitios detenidos
stats-card-apps = Aplicaciones detenidas
stats-card-busiest = Día con más bloqueos
stats-card-top = Lo más bloqueado
stats-before = { $days ->
    [1] Ayer: { $count }
   *[other] { $days } días anteriores: { $count }
}
stats-blocks = { $count ->
    [one] { $count } bloqueo
   *[other] { $count } bloqueos
}
stats-distinct-sites = { $count ->
    [one] { $count } sitio
   *[other] { $count } sitios distintos
}
stats-distinct-apps = { $count ->
    [one] { $count } aplicación
   *[other] { $count } aplicaciones distintas
}
stats-show-all = Mostrar todo ({ $count })
stats-show-fewer = Mostrar menos
stats-allowance-of = { $used } de { $total } min

# Rule lists
web-mode-block = Lista de bloqueo
web-mode-allow-only = Lista de permitidos
apps-mode-block = Lista de bloqueo
apps-mode-allow-only = Lista de permitidos
add-websites = Añadir sitios
add-apps = Añadir aplicaciones
add-exceptions = Añadir excepciones
action-remove-selected = Quitar lo seleccionado
search = Buscar
error-service-unreachable-detail = Instálalo o inícialo y vuelve a intentarlo.
error-locked-title = El bloque está bloqueado
error-wrong-password = Contraseña incorrecta
error-wrong-text = El texto no coincide
error-no-wifi = No se encontraron redes Wi-Fi. Activa el Wi-Fi y vuelve a intentarlo.
location-learn-failed = No se pudo recordar el lugar
location-known = Iqal reconoce este lugar por las redes Wi-Fi de alrededor y sabe por sí mismo cuándo está allí este ordenador.
location-here-question = ¿Está este ordenador en este lugar ahora?
location-here-yes = Sí, está allí ahora
location-here-no = No
location-here-detail = Si respondes sí, Iqal recuerda las redes Wi-Fi de alrededor y desde entonces sabe por sí mismo cuándo llega y se va el ordenador. Si respondes no, pulsa «Estoy en este lugar ahora» en la pestaña Bloqueo cuando estés allí.
location-here-unanswered = Indica si este ordenador está en el lugar ahora.
lock-confirm-timer = Cuando termine el tiempo, el bloque también se desactivará.
schedule-explain = Los horarios deciden cuándo bloquea este bloque. Un bloqueo decide cuándo el bloque no se puede cambiar ni desactivar. Por ejemplo: bloquea aquí las redes sociales los días laborables de 9:00 a 17:00 y añade un bloqueo con contraseña para que los horarios no se puedan relajar. Un bloqueo «Durante un tiempo» o «Hasta una fecha» desactiva el bloque cuando termina.

# Windows: a place lock by name (no map)
location-name = Nombre del lugar
location-name-placeholder = Por ejemplo: Casa
area-named = en «{ $name }»
