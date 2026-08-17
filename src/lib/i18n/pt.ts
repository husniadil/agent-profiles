import type { Strings } from "./index";

export const pt: Strings = {
  // Tabs
  "tab.profiles": "Agent Profiles",
  "tab.keepAwake": "Manter Ativo",
  "tab.general": "Geral",

  // Status strip and its tooltip
  "status.profile": "perfil",
  "status.profiles": "perfis",
  "status.running": "em execução",
  "status.onDisk": "no disco",
  "status.sizing": "Calculando",
  "status.summaryProfile": "{{count}} perfil",
  "status.summaryProfiles": "{{count}} perfis",
  "status.summaryRunning": "{{count}} em execução",
  "status.summaryOnDisk": "{{size}} no disco",
  "status.revealFolder": "Mostrar a pasta de perfis no gerenciador de arquivos: {{path}}",

  // Start-at-login row, in the General tab
  "autostart.label": "Iniciar ao fazer login",
  "autostart.offered": "abre apenas a bandeja — nenhum perfil é iniciado",
  "autostart.unavailable": "disponível quando o Agent Profiles estiver instalado",
  "autostart.aria": "Iniciar o Agent Profiles ao fazer login",

  // Nothing installed
  "empty.title": "Nada para abrir ainda",
  "empty.body":
    "O Agent Profiles executa os agentes de código já instalados n{{machine}}{{names}}. Instale um e reabra esta janela.",
  "empty.appsSupported": "{{count}} apps compatíveis",

  // Add a profile
  "compose.heading": "Novo perfil",
  "compose.namePlaceholder": "Nomeie este perfil",
  "compose.nameAria": "Nome do perfil",
  "compose.appAria": "App",
  "compose.add": "Adicionar",
  "compose.adding": "Adicionando",
  "compose.added": "Adicionado",
  "compose.retry": "Tentar novamente",
  "compose.needName": "Digite um nome para este perfil.",
  "compose.noApp": "Nenhum app compatível foi encontrado para adicionar um perfil.",
  "compose.thisApp": "Este app",

  // A profile row
  "profiles.empty": "Nenhum perfil ainda.",
  "row.running": "Em execução",
  "row.sharedSignIn": "Login compartilhado",
  "row.open": "Abrir {{name}}",
  "row.rename": "Renomear {{name}}",
  "row.deleteTrigger": "Excluir {{name}}",
  "row.delete": "Excluir {{name}} permanentemente. Pressione e segure para confirmar.",
  "row.deleteUnavailable":
    "{{name}} é a instalação original do app e não pode ser excluído",
  "row.renameNameAria": "Novo nome para {{name}}",
  "row.saveName": "Salvar nome",
  "row.cancel": "Cancelar",
  "row.holdToDelete": "Segure para excluir",
  "row.holdingLabel": "Continue segurando…",
  "row.completeLabel": "Excluindo…",
  "row.keepIt": "Manter",
  "row.deleteBody":
    "Excluir {{label}} e os {{bytes}} em sua pasta. Isso não pode ser desfeito.",

  // Socket path budget
  "budget.aria": "Limite do caminho do socket",
  "budget.over": "{{bytes}} bytes acima do limite",
  "budget.under": "limite do caminho do socket · {{system}} para em {{limit}}",
  "budget.ofLimit": " / {{limit}} bytes",
  "budget.tooDeep":
    "Esta pasta é profunda demais para os {{bytes}} bytes do caminho do socket que um perfil precisa. Nenhum perfil pode ser adicionado aqui.",
  "budget.cannotCreate":
    "{{app}} não conseguiria criar seu socket aqui. Mova a raiz de dados para um caminho mais curto para liberar espaço.",

  // Keep Awake — status card
  "awake.off.title": "Desligado",
  "awake.off.detail": "{{machine}} entra em suspensão ao fechar a tampa, como de costume.",
  "awake.idle.title": "Observando",
  "awake.idle.detail": "Nada está em execução agora, então nada está sendo mantido ativo.",
  "awake.holding.title": "Mantendo {{machine}} ativo",
  "awake.holding.detail":
    "Você pode fechar a tampa — a suspensão volta quando o trabalho parar.",
  "awake.lowBattery.title": "Pausado — bateria fraca",
  "awake.lowBattery.detail": "Interrompido para proteger a bateria. Conecte o carregador para retomar.",
  "awake.tooHot.title": "Pausado — {{machine}} está muito quente",
  "awake.tooHot.detail":
    "Mantê-lo ativo pioraria isso. Ele retoma assim que esfriar.",
  "awake.stranded":
    "O Agent Profiles foi encerrado inesperadamente enquanto mantinha o estado de tampa fechada, e essa configuração sobrevive a uma reinicialização.",
  "awake.restoreSleep": "Restaurar suspensão",
  "awake.needsPassword":
    "Precisa da senha de administrador uma vez por execução. Um auxiliar ativa a configuração enquanto um agente trabalha, desativa quando ele para, e encerra junto com o Agent Profiles.",

  // Keep Awake — status card bands (unsupported, stranded, unauthorized, failed hold)
  "awake.band.unavailable": "Não disponível aqui",
  "awake.band.stranded": "Seu Mac pode não conseguir suspender",
  "awake.band.notAuthorized": "Ainda não autorizado",
  "awake.band.holdFailed": "Não ativo — a solicitação falhou",
  "awake.band.holdFailedDetail": "{{machine}} vai suspender normalmente: {{error}}",
  "awake.unsupported.linux":
    "O systemd-inhibit não foi encontrado, então nada aqui pode obter um bloqueio de tampa. Manter a tampa fechada exige um desktop rodando systemd-logind.",
  "awake.unsupported.generic":
    "{{system}} n{{machine}} informa que não é possível manter a tampa fechada.",
  "awake.authorize": "Autorizar…",

  // Keep Awake — status card's assembled status line
  "awake.status.noBattery": "Sem bateria",
  "awake.status.battery": "Bateria {{percent}}%",
  "awake.status.pluggedIn": ", na tomada",
  "awake.status.held": " · ativo há {{duration}}",

  // Keep Awake — section legends
  "awake.section.hold": "Manter a máquina ativa",
  "awake.section.limits": "Limites",
  "awake.section.watching": "Observando",

  // Keep Awake — low-battery control
  "awake.battery.name": "Pausar com bateria fraca",
  "awake.battery.aria": "Pausar com bateria fraca",
  "awake.battery.below": "abaixo de {{percent}}%",

  // Keep Awake — thermal guard
  "awake.thermal.name": "Proteção térmica",
  "awake.thermal.aria": "Proteção térmica",

  // Keep Awake — hint paragraphs under each Limits setting
  "awake.hint.noBattery": "{{machine}} não tem bateria, então isso nunca se aplica.",
  "awake.hint.lowBattery":
    "Caiu abaixo desta carga, mesmo no meio de uma tarefa. Ignorado enquanto conectado à energia.",
  "awake.hint.idleWindow":
    "Um agente que terminou sua vez libera {{machine}} imediatamente. Isso só limita um que parou no meio do caminho: após ficar tanto tempo sem escrever nada, ele é tratado como encerrado, não como em execução.",
  "awake.hint.thermal":
    "Libera a máquina quando ela relata superaquecimento.",

  // Keep Awake — triggers and limits
  "awake.trigger.off": "Desligado",
  "awake.trigger.agentActive": "Quando um agente está trabalhando",
  "awake.trigger.agentActiveDetail":
    "Uma sessão do Claude Code ou Codex recebendo escrita.",
  "awake.trigger.always": "Sempre que o Agent Profiles estiver em execução",
  "awake.trigger.alwaysDetail":
    "Para agentes dentro de um app desktop, onde não há nada a detectar.",
  "awake.limit.idleWindow": "Desistir de um agente silencioso após",
  "awake.limit.minutes": "min",
  "awake.limit.aria": "{{label}} ({{unit}})",

  // Keep Awake — watch list
  "awake.watch.empty":
    "Nada para observar ainda. Claude Code e Codex são encontrados automaticamente assim que gravam uma sessão.",
  "awake.watch.working": "Trabalhando",
  "awake.watch.never": "nunca",
  "awake.watch.ago": "há {{duration}}",
  "awake.watch.stalled": "parado há {{duration}}",
  "awake.watch.idle": "ocioso há {{duration}}",

  // Machine and system names — "this Mac" reads differently in six languages
  "system.macos": "macOS",
  "system.windows": "Windows",
  "system.linux": "Linux",
  "system.unknown": "este sistema",
  "machine.mac": "este Mac",
  "machine.pc": "este PC",
  "machine.computer": "este computador",

  // General tab — language
  "general.language.label": "Idioma",
  "general.language.detail": "Aplica-se a esta janela e ao menu da bandeja.",
  "general.language.system": "Igual ao sistema",

  // General tab — updates
  "general.update.label": "Atualizar automaticamente",
  "general.update.detail": "Instala novas versões em segundo plano e reinicia em seguida.",
  "general.update.aria": "Instalar atualizações automaticamente",
  "general.update.version": "Versão {{version}}",
  "general.update.checkNow": "Verificar agora",
  "general.update.idle": "Ainda não verificado.",
  "general.update.checking": "Verificando atualizações…",
  "general.update.current": "Atualizado.",
  "general.update.downloading": "Baixando… {{percent}}%",
  "general.update.installing": "Instalando, depois reiniciando…",
  "general.update.failed": "Não foi possível atualizar: {{reason}}",
  "general.update.disabled": "Desativado — nenhuma versão é verificada.",
} as const;
