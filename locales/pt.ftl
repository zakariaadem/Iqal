# Al-Dir — Português

app-name = Al-Dir
app-description = Bloqueador de sites e aplicativos
menu = Menu
menu-import = Importar bloqueios…
menu-export-all = Exportar todos os bloqueios…
menu-activity = Atividade recente
menu-settings = Configurações
menu-about = Sobre o Al-Dir

action-new-block = Novo bloqueio
action-remove = Remover
action-add = Adicionar
action-cancel = Cancelar
action-delete = Excluir
action-create = Criar
action-export-block = Exportar este bloqueio
action-delete-block = Excluir este bloqueio
action-import-text = Importar de um arquivo de texto…
action-add-installed-app = Adicionar um aplicativo instalado…
action-add-program-file = Adicionar um arquivo de programa…
action-add-times = Adicionar estes horários
action-cancel-unlock = Cancelar o desbloqueio
action-learn-location = Estou neste lugar agora
action-unlock = Destrancar…
action-remove-lock = Remover a tranca
action-change-lock = Mudar a tranca…
action-lock = Trancar
action-choose-on-map = Escolher no mapa…
action-use-area = Usar esta área

list-empty = Nada aqui ainda.

state-off = Desligado
state-active = Ativo
state-scheduled = Agendado
state-locked = Trancado

error-title = Algo deu errado
error-service-unreachable = O serviço Al-Dir não está em execução. Instale-o ou inicie-o e tente de novo.
error-unexpected = O serviço Al-Dir deu uma resposta inesperada.
error-locked = Este bloqueio está trancado, e esta alteração { $reason }. Bloqueios trancados só podem ficar mais rígidos.
error-wrong-answer = Não está correto.
error-cannot-unlock-early = Esta tranca não pode ser aberta antes da hora.
error-not-admin = Só administradores podem alterar as configurações do Al-Dir.

reason-disables = o desligaria
reason-fewer-users = o aplicaria a menos pessoas
reason-web-mode = mudaria a forma de bloquear sites
reason-removes-websites = removeria sites bloqueados
reason-adds-allowed-websites = permitiria mais sites
reason-safesearch-off = desligaria o SafeSearch
reason-app-mode = mudaria a forma de bloquear aplicativos
reason-removes-apps = removeria aplicativos bloqueados
reason-adds-allowed-apps = permitiria mais aplicativos
reason-adds-schedule = o limitaria a certos horários
reason-shortens-schedule = encurtaria seus horários
reason-adds-allowance = adicionaria um tempo permitido
reason-increases-allowance = aumentaria o tempo permitido
reason-removes-lock = removeria a tranca
reason-shortens-delay = encurtaria a espera antes de destrancar
reason-weakens-lock = enfraqueceria a tranca
reason-pending-unlock = mudaria um desbloqueio pendente
reason-deletes = o excluiria
reason-hardened-off = desligaria o modo reforçado

empty-title = Nenhum bloqueio ainda
empty-detail = Um bloqueio é um grupo de sites e aplicativos a bloquear, com horários e tranca próprios. Crie um com “Novo bloqueio”.

block-summary = { $web } regras de sites · { $apps } regras de aplicativos
rules-count = { $count ->
    [one] { $count } regra
   *[other] { $count } regras
}

locked-banner = Este bloqueio está trancado. Você pode adicionar restrições, mas não removê-las.
tab-websites = Sites
tab-apps = Aplicativos
tab-users = Pessoas
tab-schedule = Horários
tab-lock = Tranca
block-enabled = Ligar ou desligar o bloqueio
confirm-delete = Excluir “{ $name }”?
confirm-delete-detail = Seus sites, aplicativos, horários e tranca serão removidos.
rule-invalid = “{ $rule }” não pode ser usado: { $problem }

web-mode = Como os sites são bloqueados
web-mode-block = Bloquear os sites da lista
web-mode-allow-only = Permitir apenas os sites da lista
web-blocked = Sites bloqueados
web-also-blocked = Bloqueados mesmo se permitidos
web-placeholder = example.com, example.com/page, contains:palavra, search:palavras
web-syntax = example.com bloqueia o site e todos os seus subdomínios. example.com/videos bloqueia só essas páginas. contains:casino bloqueia qualquer endereço que contenha a palavra. search:filmes grátis bloqueia pesquisas que contenham essas palavras.
web-allowed = Sites permitidos
web-exceptions = Exceções (sempre permitidas)
web-safesearch = Forçar SafeSearch (Google, Bing, DuckDuckGo, YouTube)
web-partial = Bloqueios parciais
web-partial-detail = Bloqueie só a parte que distrai de um site e mantenha o resto.
partial-youtube-shorts = YouTube Shorts
partial-youtube-comments = Comentários do YouTube
partial-youtube-recommendations = Recomendações e página inicial do YouTube
partial-reddit-home = Página inicial do Reddit, r/all e r/popular
partial-facebook-feed = Feed de notícias do Facebook, Watch e Reels
partial-instagram-reels = Reels do Instagram
partial-instagram-explore = Explorar do Instagram
partial-x-trends = Explorar e assuntos do momento do X (Twitter)

import-done = Importação concluída
import-result = { $added } regras adicionadas, { $skipped } linhas ignoradas.
import-added = Adicionado: { $name }
import-replaced = Atualizado: { $name }
import-rejected = Não importado: { $name } ({ $why })

apps-mode = Como os aplicativos são bloqueados
apps-mode-block = Bloquear os aplicativos da lista
apps-mode-allow-only = Permitir apenas os aplicativos da lista
apps-blocked = Aplicativos bloqueados
apps-also-blocked = Bloqueados mesmo se permitidos
apps-exceptions = Exceções (sempre permitidas)
apps-allowed = Aplicativos permitidos
apps-placeholder = steam, /opt/games/, title:YouTube, cmd:minecraft
apps-syntax = Um nome bloqueia o programa com esse nome. Uma pasta terminada em / bloqueia todos os programas dentro dela. title:Texto fecha janelas cujo título contém o texto. cmd:Texto bloqueia programas cuja linha de comando o contém. Cópias renomeadas são detectadas com regras hash:.

users-applies-to = A quem este bloqueio se aplica
users-all = Todos
users-all-except = Todos, exceto…
users-only = Apenas…
users-except-these = Exceto estas pessoas
users-only-these = Apenas estas pessoas
users-note = Os administradores deste computador também são bloqueados, a não ser que sejam excluídos aqui. O próprio sistema nunca é bloqueado.

day-mon = seg
day-tue = ter
day-wed = qua
day-thu = qui
day-fri = sex
day-sat = sáb
day-sun = dom
time-from = das
time-to = às
time-at = às

schedule-limited = Apenas em certos horários
schedule-limited-detail = O bloqueio vale apenas nos horários abaixo.
schedule-always = O bloqueio vale sempre.
schedule-times = Horários
schedule-date-placeholder = Pontual: 2026-10-20 08:00 2026-10-22 18:00
schedule-date-invalid = Escreva um período pontual assim: data de início, hora de início, data de fim, hora de fim (2026-10-20 08:00 2026-10-22 18:00).
schedule-add-weekly = Adicionar horários semanais
schedule-no-days = Escolha pelo menos um dia.
allowance = Tempo permitido por dia
allowance-detail = Permite os sites e aplicativos bloqueados por um tempo limitado a cada dia; depois disso, ficam bloqueados.
allowance-minutes = Minutos por dia

area-circle = a até { $radius } m de { $lat }, { $lon }
area-polygon = dentro de uma área com { $count } cantos
lock-desc-until = Trancado até { $time }
lock-desc-schedule = Trancado durante { $times }
lock-desc-password = Trancado com senha
lock-desc-random = Destranca ao digitar { $length } caracteres aleatórios
lock-desc-restart = Trancado até reiniciar o computador
lock-desc-location = Trancado enquanto este computador estiver { $area }
lock-desc-delay = abre { $minutes } minutos após a resposta

lock-none = Não trancado
lock-none-detail = Sem tranca, este bloqueio pode ser alterado ou desligado a qualquer momento. Com tranca, só pode ficar mais rígido até a tranca acabar.
lock-active = Trancado
lock-inactive = Tranca definida, não ativa agora
lock-pending = A tranca abre em { $time }.
location-learn-detail = Se você está neste lugar agora, deixe o Al-Dir lembrar as redes Wi-Fi daqui. É assim que o Al-Dir sabe que você está lá, mesmo sem internet.
location-learned = Lugar memorizado
location-learned-detail = Agora { $count } redes Wi-Fi estão memorizadas para este lugar.
location-none-chosen = Nenhuma área escolhida ainda.

lock-kind-duration = Por um tempo
lock-kind-until = Até uma data
lock-kind-password = Com uma senha
lock-kind-random = Digitando um texto aleatório
lock-kind-restart = Até reiniciar o computador
lock-kind-schedule = Em certos horários
lock-kind-location = Enquanto eu estiver em um lugar
lock-dialog-title = Trancar este bloqueio
unit-days = dias
unit-hours = horas
unit-minutes = minutos
unit-metres = metros
password = Senha
password-repeat = Repita a senha
password-generate = Criar uma senha para outra pessoa guardar
password-generate-detail = O Al-Dir cria uma senha e a mostra uma única vez. Entregue-a a alguém de confiança.
password-mismatch = As senhas estão vazias ou não coincidem.
password-generated = A senha da tranca
password-generated-detail = { $password }

    Entregue-a a alguém de confiança. Ela não será mostrada de novo.
random-length = Caracteres a digitar
lock-restart-detail = O bloqueio fica trancado até este computador ser reiniciado.
lock-location-detail = O bloqueio fica trancado enquanto este computador estiver dentro da área e também sempre que o Al-Dir não souber onde ele está.
unlock-delay = Espera após destrancar
unlock-delay-detail = Com uma espera, a resposta correta só abre a tranca depois desse tempo. Isso ajuda contra impulsos repentinos.
lock-duration-zero = Escolha uma duração.
lock-until-past = Esse momento já passou.
lock-confirm = Trancar este bloqueio?
lock-confirm-detail = { $lock }. Até lá, nada neste bloqueio poderá ser removido, afrouxado ou desligado.

unlock-not-possible = Esta tranca não pode ser aberta antes da hora
unlock-not-possible-detail = Ela abre sozinha quando o tempo acaba ou quando você sai do lugar.
unlock-title = Destrancar
unlock-type-text = Digite este texto exatamente (não é possível colar):
unlock-enter-password = Digite a senha da tranca:
unlocked = Destrancado
unlocked-detail = O bloqueio pode ser alterado de novo.
unlock-pending = Desbloqueio iniciado

map-title = Escolher a área
map-help = Clique no mapa para colocar o centro da área e depois ajuste o tamanho. Arraste para mover o mapa e use a roda para aproximar.
map-radius = Raio

new-block-name = Nome do bloqueio
new-block-placeholder = Por exemplo: Redes sociais
export-done = Exportado

activity-empty = Nada foi bloqueado ainda.
event-app-start = Aplicativo interrompido
event-app-killed = Aplicativo fechado
event-window-closed = Janela fechada
event-site-blocked = Site bloqueado
event-page-blocked = Página bloqueada

settings-language = Idioma
settings-language-system = Igual ao sistema
settings-language-restart = Reinicie o Al-Dir para usar o novo idioma.
settings-hardened = Modo reforçado
settings-hardened-detail = Enquanto houver um bloqueio trancado, as pessoas afetadas não podem usar sudo nem pedidos de senha de administrador, portanto não podem parar o Al-Dir. Acesso de emergência: reiniciar no modo de recuperação. Não pode ser desligado enquanto algo estiver trancado.
settings-status = Estado
status-service = Serviço Al-Dir { $version }
status-time = Hora: { $source }
status-location = Localização: { $state }
status-extensions = Extensões do navegador: { $list }
status-no-extensions = nenhuma conectada
status-not-admin = Você pode ver os bloqueios, mas não alterá-los (apenas administradores).

event-allowance-low = Tempo permitido quase esgotado
event-allowance-used = Tempo permitido esgotado

web-lists = Listas prontas
web-lists-detail = Coleções de sites já prontas. As de conteúdo adulto, apostas e redes sociais são atualizadas diariamente a partir de uma grande lista comunitária.
list-social = Redes sociais
list-adult = Conteúdo adulto
list-gambling = Apostas
list-search = Mecanismos de busca
list-news = Notícias
list-video = Vídeo e streaming
list-games = Jogos
list-shopping = Compras
list-count = { $count ->
    [one] { $count } site
   *[other] { $count } sites
}
allowance-used-today = Usado hoje: { $used } de { $total } minutos

menu-stats = Estatísticas
stats-7 = Últimos 7 dias
stats-30 = Últimos 30 dias
stats-90 = Últimos 90 dias
stats-private = As estatísticas ficam neste computador.
stats-top = Mais bloqueados
stats-by-block = Por bloqueio
stats-allowance = Tempo permitido usado
stats-minutes = { $minutes } min
stats-empty = Nada foi bloqueado neste período.

notify-app-blocked = Aplicativo bloqueado
notify-app-blocked-body = { $app } foi interrompido por “{ $block }”.
notify-app-closed = Aplicativo fechado
notify-app-closed-body = { $app } foi fechado por “{ $block }”.
notify-window-closed = Janela fechada
notify-window-closed-body = “{ $title }” foi fechada por “{ $block }”.
notify-allowance-low = Faltam 5 minutos
notify-allowance-low-body = O tempo diário de “{ $block }” está quase esgotado.
notify-allowance-used = O tempo acabou
notify-allowance-used-body = O tempo diário de “{ $block }” acabou. Fica bloqueado até amanhã.

# Saving
unsaved-changes = Alterações não salvas
action-save = Salvar
action-discard = Descartar
confirm-unsaved = Salvar as alterações em “{ $name }”?
confirm-unsaved-detail = Se você não salvar, suas alterações serão perdidas.
save-has-errors = Corrija estas linhas antes de salvar
line-invalid = Linha { $line }: “{ $rule }” não pode ser usado: { $problem }
lines-invalid-more = (mais { $count } linhas precisam de correção)
list-empty-hint = Um por linha, por exemplo: { $examples }
lock-saves-changes = Suas outras alterações não salvas neste bloqueio também serão salvas.

# Statistics window
stats-today = Hoje
stats-chart = Bloqueios por dia
stats-card-total = Vezes bloqueado
stats-card-websites = Sites interrompidos
stats-card-apps = Apps interrompidos
stats-card-busiest = Dia com mais bloqueios
stats-card-top = Mais bloqueado
stats-before = { $days ->
    [1] Ontem: { $count }
   *[other] { $days } dias anteriores: { $count }
}
stats-blocks = { $count ->
    [one] { $count } bloqueio
   *[other] { $count } bloqueios
}
stats-distinct-sites = { $count ->
    [one] { $count } site
   *[other] { $count } sites diferentes
}
stats-distinct-apps = { $count ->
    [one] { $count } app
   *[other] { $count } apps diferentes
}
stats-show-all = Mostrar todos ({ $count })
stats-show-fewer = Mostrar menos
stats-allowance-of = { $used } de { $total } min
