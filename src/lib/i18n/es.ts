import type { Strings } from "./index";

export const es: Strings = {
  // Tabs
  "tab.profiles": "Perfiles de agente",
  "tab.keepAwake": "Mantener activo",
  "tab.general": "General",

  // Status strip and its tooltip
  "status.profile": "perfil",
  "status.profiles": "perfiles",
  "status.running": "en ejecución",
  "status.onDisk": "en disco",
  "status.sizing": "Calculando",
  "status.summaryProfile": "{{count}} perfil",
  "status.summaryProfiles": "{{count}} perfiles",
  "status.summaryRunning": "{{count}} en ejecución",
  "status.summaryOnDisk": "{{size}} en disco",
  "status.summaryOnDiskApprox": "al menos {{size}} en disco",
  "status.sizeAtLeast": "al menos {{size}}",
  "status.onDiskApproxWhy":
    "No se pudieron leer algunas carpetas, así que el total real es mayor en una cantidad desconocida.",
  "status.revealFolder": "Mostrar la carpeta de perfiles en el gestor de archivos: {{path}}",

  // Start-at-login row, in the General tab
  "autostart.label": "Abrir al iniciar sesión",
  "autostart.offered": "solo abre la bandeja — no se inicia ningún perfil",
  "autostart.unavailable": "disponible una vez instalado Agent Profiles",
  "autostart.aria": "Abrir Agent Profiles al iniciar sesión",

  // Nothing installed
  "empty.title": "Aún no hay nada que abrir",
  "empty.body":
    "Agent Profiles ejecuta los agentes de código ya instalados en {{machine}}{{names}}. Instala uno y vuelve a abrir esta ventana.",
  "empty.appsSupported": "{{count}} apps compatibles",

  // Add a profile
  "compose.heading": "Nuevo perfil",
  "compose.namePlaceholder": "Nombra este perfil",
  "compose.nameAria": "Nombre del perfil",
  "compose.appAria": "App",
  "compose.add": "Añadir",
  "compose.adding": "Añadiendo",
  "compose.added": "Añadido",
  "compose.retry": "Reintentar",
  "compose.needName": "Introduce un nombre para este perfil.",
  "compose.noApp": "No se encontró ninguna app compatible para añadir un perfil.",
  "compose.thisApp": "Esta app",

  // A profile row
  "profiles.empty": "Aún no hay perfiles.",
  "row.running": "En ejecución",
  "row.sharedSignIn": "Inicio de sesión compartido",
  "row.open": "Abrir {{name}}",
  "row.rename": "Renombrar {{name}}",
  "row.deleteTrigger": "Eliminar {{name}}",
  "row.delete": "Eliminar {{name}} de forma permanente. Mantén pulsado para confirmar.",
  "row.deleteUnavailable":
    "{{name}} es la instalación propia de la app y no se puede eliminar",
  "row.renameNameAria": "Nuevo nombre para {{name}}",
  "row.saveName": "Guardar nombre",
  "row.cancel": "Cancelar",
  "row.holdToDelete": "Mantén pulsado para eliminar",
  "row.holdingLabel": "Sigue pulsando…",
  "row.completeLabel": "Eliminando…",
  "row.keepIt": "Conservarlo",
  "row.deleteBody":
    "Eliminar {{label}} y los {{bytes}} de su carpeta. Esta acción no se puede deshacer.",

  // Socket path budget
  "budget.aria": "Presupuesto de la ruta del socket",
  "budget.over": "{{bytes}} bytes por encima del límite",
  "budget.under": "presupuesto de la ruta del socket · {{system}} se detiene en {{limit}}",
  "budget.ofLimit": " / {{limit}} bytes",
  "budget.tooDeep":
    "Esta carpeta es demasiado profunda para los {{bytes}} bytes de ruta de socket que necesita un perfil. Aquí no se puede añadir ningún perfil.",
  "budget.cannotCreate":
    "{{app}} no podría crear su socket aquí. Mueve la raíz de datos a una ubicación con una ruta más corta para hacer espacio.",

  // Keep Awake — status card
  "awake.off.title": "Desactivado",
  "awake.off.detail": "{{machine}} se suspende al cerrar la tapa, como de costumbre.",
  "awake.idle.title": "Observando",
  "awake.idle.detail": "Ahora mismo no hay nada trabajando, así que no se mantiene nada activo.",
  "awake.holding.title": "Manteniendo activo {{machine}}",
  "awake.holding.detail":
    "Puedes cerrar la tapa — la suspensión vuelve en cuanto el trabajo se detiene.",
  "awake.lowBattery.title": "En pausa — batería baja",
  "awake.lowBattery.detail": "Se pausó para proteger la batería. Conecta el cargador para reanudar.",
  "awake.tooHot.title": "En pausa — {{machine}} está muy caliente",
  "awake.tooHot.detail":
    "Mantenerlo activo empeoraría la situación. Se reanuda en cuanto se enfríe.",
  "awake.stranded":
    "Agent Profiles se cerró inesperadamente mientras mantenía activo el estado de tapa cerrada, y ese ajuste persiste tras un reinicio.",
  "awake.restoreSleep": "Restaurar la suspensión",
  "awake.needsPassword":
    "Necesita una contraseña de administrador una vez por sesión. Un asistente activa el ajuste mientras un agente trabaja, lo desactiva cuando se detiene, y se cierra junto con Agent Profiles.",

  // Keep Awake — status card bands (unsupported, stranded, unauthorized, failed hold)
  "awake.band.unavailable": "No disponible aquí",
  "awake.band.stranded": "Puede que tu Mac no pueda suspenderse",
  "awake.band.notAuthorized": "Aún no autorizado",
  "awake.band.holdFailed": "No se mantiene activo — la solicitud falló",
  "awake.band.holdFailedDetail": "{{machine}} se suspenderá como de costumbre: {{error}}",
  "awake.unsupported.linux":
    "No se encontró systemd-inhibit, así que nada aquí puede retener el bloqueo del interruptor de tapa. Mantener la tapa cerrada activa requiere un escritorio con systemd-logind en ejecución.",
  "awake.unsupported.generic":
    "{{system}} en {{machine}} informa que no puede mantener la tapa cerrada.",
  "awake.authorize": "Autorizar…",

  // Keep Awake — status card's assembled status line
  "awake.status.noBattery": "Sin batería",
  "awake.status.battery": "Batería {{percent}}%",
  "awake.status.pluggedIn": ", conectado",
  "awake.status.held": " · activo {{duration}}",

  // Keep Awake — section legends
  "awake.section.hold": "Mantener el equipo activo",
  "awake.section.limits": "Límites",
  "awake.section.watching": "Vigilancia",

  // Keep Awake — low-battery control
  "awake.battery.name": "Pausar con batería baja",
  "awake.battery.aria": "Pausar con batería baja",
  "awake.battery.below": "por debajo del {{percent}}%",

  // Keep Awake — thermal guard
  "awake.thermal.name": "Protección térmica",
  "awake.thermal.aria": "Protección térmica",

  // Keep Awake — hint paragraphs under each Limits setting
  "awake.hint.noBattery": "{{machine}} no tiene batería, así que esto nunca se aplica.",
  "awake.hint.lowBattery":
    "Cae por debajo de esta carga, incluso a mitad de una tarea. Se ignora mientras está conectado.",
  "awake.hint.idleWindow":
    "Un agente que terminó su turno libera {{machine}} de inmediato. Esto solo limita a uno que se detuvo a mitad de camino: tras este tiempo sin escribir nada, se considera ausente en lugar de trabajando.",
  "awake.hint.thermal":
    "Libera la retención cuando el equipo informa que se está sobrecalentando.",

  // Keep Awake — triggers and limits
  "awake.trigger.off": "Desactivado",
  "awake.trigger.agentActive": "Cuando un agente está trabajando",
  "awake.trigger.agentActiveDetail":
    "Una sesión de Claude Code o Codex a la que se está escribiendo.",
  "awake.trigger.always": "Siempre que Agent Profiles esté en ejecución",
  "awake.trigger.alwaysDetail":
    "Para agentes dentro de una app de escritorio, donde no hay nada que detectar.",
  "awake.limit.idleWindow": "Renunciar a un agente silencioso tras",
  "awake.limit.minutes": "min",
  "awake.limit.aria": "{{label}} ({{unit}})",

  // Keep Awake — watch list
  "awake.watch.empty":
    "Aún no hay nada que vigilar. Claude Code y Codex se detectan automáticamente en cuanto han escrito una sesión.",
  "awake.watch.working": "Trabajando",
  "awake.watch.never": "nunca",
  "awake.watch.ago": "hace {{duration}}",
  "awake.watch.stalled": "estancado {{duration}}",
  "awake.watch.idle": "inactivo {{duration}}",

  // Machine and system names — "this Mac" reads differently in six languages
  "system.macos": "macOS",
  "system.windows": "Windows",
  "system.linux": "Linux",
  "system.unknown": "este sistema",
  "machine.mac": "este Mac",
  "machine.pc": "este PC",
  "machine.computer": "este equipo",

  // General tab — language
  "general.language.label": "Idioma",
  "general.language.detail": "Se aplica a esta ventana y al menú de la bandeja.",
  "general.language.system": "Igual que el sistema",

  // General tab — updates
  "general.update.label": "Actualizar automáticamente",
  "general.update.detail": "Instala las nuevas versiones en segundo plano y luego reinicia.",
  "general.update.aria": "Instalar actualizaciones automáticamente",
  "general.update.version": "Versión {{version}}",
  "general.update.checkNow": "Buscar ahora",
  "general.update.checkFailed": "No se pudo buscar actualizaciones",
  "general.update.lastChecked": "Última comprobación a las {{time}}",
  "general.update.idle": "Aún no se ha comprobado.",
  "general.update.checking": "Buscando actualizaciones…",
  "general.update.current": "Actualizado.",
  "general.update.downloading": "Descargando… {{percent}}%",
  "general.update.installing": "Instalando y reiniciando…",
  "general.update.failed": "No se pudo actualizar: {{reason}}",
  "general.update.disabled": "Desactivado — no se busca ninguna versión.",
} as const;
