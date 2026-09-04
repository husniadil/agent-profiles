import type { Strings } from "./index";

export const de: Strings = {
  // Tabs
  "tab.profiles": "Agent-Profile",
  "tab.keepAwake": "Wach halten",
  "tab.schedule": "Zeitplan",
  "tab.general": "Allgemein",

  // Status strip and its tooltip
  "status.profile": "Profil",
  "status.profiles": "Profile",
  "status.running": "aktiv",
  "status.onDisk": "auf der Festplatte",
  "status.sizing": "Wird berechnet",
  "status.summaryProfile": "{{count}} Profil",
  "status.summaryProfiles": "{{count}} Profile",
  "status.summaryRunning": "{{count}} aktiv",
  "status.summaryOnDisk": "{{size}} auf der Festplatte",
  "status.summaryOnDiskApprox": "mindestens {{size}} auf der Festplatte",
  "status.sizeAtLeast": "mindestens {{size}}",
  "status.onDiskApproxWhy":
    "Einige Ordner konnten nicht gelesen werden; der tatsächliche Wert liegt um einen unbekannten Betrag höher.",
  "status.revealFolder": "Profilordner im Dateimanager anzeigen: {{path}}",

  // Start-at-login row, in the General tab
  "autostart.label": "Bei Anmeldung starten",
  "autostart.offered": "öffnet nur das Tray-Menü — es wird kein Profil gestartet",
  "autostart.unavailable": "verfügbar, sobald Agent Profiles installiert ist",
  "autostart.aria": "Agent Profiles bei der Anmeldung starten",

  // Nothing installed
  "empty.title": "Noch nichts zu öffnen",
  "empty.body":
    "Agent Profiles nutzt die Coding-Agenten, die bereits installiert sind: {{machine}}{{names}}. Ein Programm installieren und dieses Fenster erneut öffnen.",
  "empty.appsSupported": "{{count}} Apps unterstützt",

  // Add a profile
  "compose.heading": "Neues Profil",
  "compose.namePlaceholder": "Profilnamen eingeben",
  "compose.nameAria": "Profilname",
  "compose.appAria": "App",
  "compose.add": "Hinzufügen",
  "compose.adding": "Wird hinzugefügt",
  "compose.added": "Hinzugefügt",
  "compose.retry": "Erneut versuchen",
  "compose.needName": "Bitte einen Namen für dieses Profil eingeben.",
  "compose.noApp": "Keine unterstützte App gefunden, der ein Profil hinzugefügt werden kann.",
  "compose.thisApp": "Diese App",

  // A profile row
  "profiles.empty": "Noch keine Profile.",
  "profiles.unavailable": "Nicht verfügbar: {{reason}}",
  "row.running": "Aktiv",
  "row.sharedSignIn": "Gemeinsame Anmeldung",
  "row.open": "{{name}} öffnen",
  "row.rename": "{{name}} umbenennen",
  "row.deleteTrigger": "{{name}} löschen",
  "row.delete": "{{name}} endgültig löschen. Zum Bestätigen gedrückt halten.",
  "row.deleteUnavailable":
    "{{name}} ist die App-Installation selbst und kann nicht gelöscht werden",
  "row.renameNameAria": "Neuer Name für {{name}}",
  "row.saveName": "Namen speichern",
  "row.cancel": "Abbrechen",
  "row.holdToDelete": "Zum Löschen halten",
  "row.holdingLabel": "Weiter halten…",
  "row.completeLabel": "Wird gelöscht…",
  "row.keepIt": "Behalten",
  "row.deleteBody":
    "{{label}} und die {{bytes}} in seinem Ordner löschen. Das kann nicht rückgängig gemacht werden.",

  // Socket path budget
  "budget.aria": "Socket-Pfad-Budget",
  "budget.over": "{{bytes}} Bytes über dem Limit",
  "budget.under": "Socket-Pfad-Budget · {{system}} stoppt bei {{limit}}",
  "budget.ofLimit": " / {{limit}} Bytes",
  "budget.tooDeep":
    "Dieser Ordner liegt zu tief für die {{bytes}} Bytes des Socket-Pfads, die ein Profil benötigt. Hier kann kein Profil hinzugefügt werden.",
  "budget.cannotCreate":
    "{{app}} könnte hier keinen Socket erstellen. Das Datenverzeichnis an einen Ort mit kürzerem Pfad verschieben, um Platz zu schaffen.",

  // Keep Awake — status card
  "awake.off.title": "Aus",
  "awake.off.detail": "{{machine}} schläft beim Schließen des Deckels, wie gewohnt.",
  "awake.idle.title": "Beobachtung",
  "awake.idle.detail": "Gerade arbeitet nichts, daher wird nichts wachgehalten.",
  "awake.holding.title": "{{machine}} bleibt wach",
  "awake.holding.detail":
    "Der Deckel kann geschlossen werden — der Ruhezustand kehrt zurück, sobald die Arbeit endet.",
  "awake.lowBattery.title": "Pausiert — Akku schwach",
  "awake.lowBattery.detail": "Beendet, um den Akku zu schonen. Zum Fortsetzen aufladen.",
  "awake.tooHot.title": "Pausiert — {{machine}} ist zu heiß",
  "awake.tooHot.detail":
    "Das Wachhalten würde die Lage verschlimmern. Es wird fortgesetzt, sobald die Temperatur wieder sinkt.",
  "awake.stranded":
    "Agent Profiles wurde unerwartet beendet, während der Zustand „Deckel geschlossen“ aktiv war, und diese Einstellung übersteht einen Neustart.",
  "awake.restoreSleep": "Ruhezustand wiederherstellen",
  "awake.needsPassword":
    "Erfordert einmalig ein Administratorkennwort auf diesem Mac, nicht bei jedem Start. Gewährt werden genau zwei Befehle — die Einstellung bei geschlossenem Deckel ein- und auszuschalten — und sonst nichts. Zum späteren Entfernen: sudo rm /etc/sudoers.d/agent-profiles",

  // Keep Awake — status card bands (unsupported, stranded, unauthorized, failed hold)
  "awake.band.unavailable": "Hier nicht verfügbar",
  "awake.band.stranded": "Dieser Mac kann möglicherweise nicht in den Ruhezustand wechseln",
  "awake.band.notAuthorized": "Noch nicht autorisiert",
  "awake.band.holdFailed": "Nicht aktiv — Anfrage fehlgeschlagen",
  "awake.band.holdFailedDetail": "{{machine}} wird wie gewohnt in den Ruhezustand wechseln: {{error}}",
  "awake.unsupported.linux":
    "systemd-inhibit wurde nicht gefunden, daher kann hier keine Deckel-Sperre gesetzt werden. Für „Deckel geschlossen“ wird eine Desktop-Umgebung mit systemd-logind benötigt.",
  "awake.unsupported.generic":
    "{{system}} meldet: {{machine}} kann den Zustand „Deckel geschlossen“ nicht halten.",
  "awake.authorize": "Autorisieren…",

  // Keep Awake — status card's assembled status line
  "awake.status.noBattery": "Kein Akku",
  "awake.status.battery": "Akku {{percent}}%",
  "awake.status.pluggedIn": ", angeschlossen",
  "awake.status.held": " · gehalten {{duration}}",

  // Keep Awake — section legends
  "awake.section.hold": "Gerät wach halten",
  "awake.section.limits": "Grenzwerte",
  "awake.section.watching": "Beobachtung",

  // Keep Awake — low-battery control
  "awake.battery.name": "Bei niedrigem Akkustand pausieren",
  "awake.battery.aria": "Bei niedrigem Akkustand pausieren",
  "awake.battery.below": "unter {{percent}}%",

  // Keep Awake — thermal guard
  "awake.thermal.name": "Temperaturschutz",
  "awake.thermal.aria": "Temperaturschutz",

  // Keep Awake — hint paragraphs under each Limits setting
  "awake.hint.noBattery": "{{machine}} hat keinen Akku, daher greift dies nie.",
  "awake.hint.lowBattery":
    "Wird unterschritten, auch mitten in einer Aufgabe. Bei Netzbetrieb ignoriert.",
  "awake.hint.idleWindow":
    "{{machine}} wird sofort freigegeben, sobald ein Agent seinen Zug beendet hat. Das betrifft nur einen Agenten, der mittendrin gestoppt hat: Schreibt er so lange nichts, gilt er als beendet statt als aktiv.",
  "awake.hint.thermal":
    "Gibt die Sperre frei, sobald das Gerät eine Überhitzung meldet.",

  // Keep Awake — triggers and limits
  "awake.trigger.off": "Aus",
  "awake.trigger.agentActive": "Wenn ein Agent arbeitet",
  "awake.trigger.agentActiveDetail":
    "Eine Claude Code- oder Codex-Sitzung, in die geschrieben wird.",
  "awake.trigger.always": "Immer, während Agent Profiles läuft",
  "awake.trigger.alwaysDetail":
    "Für Agenten innerhalb einer Desktop-App, wo nichts erkannt werden kann.",
  "awake.limit.idleWindow": "Stillen Agenten aufgeben nach",
  "awake.limit.minutes": "Min.",
  "awake.limit.aria": "{{label}} ({{unit}})",

  // Keep Awake — watch list
  "awake.watch.empty":
    "Noch nichts zu beobachten. Claude Code und Codex werden automatisch erkannt, sobald sie eine Sitzung geschrieben haben.",
  "awake.watch.working": "Arbeitet",
  "awake.watch.never": "nie",
  "awake.watch.ago": "vor {{duration}}",
  "awake.watch.stalled": "hängt seit {{duration}}",
  "awake.watch.idle": "untätig seit {{duration}}",

  // Machine and system names — "this Mac" reads differently in six languages
  "system.macos": "macOS",
  "system.windows": "Windows",
  "system.linux": "Linux",
  "system.unknown": "dieses System",
  "machine.mac": "dieser Mac",
  "machine.pc": "dieser PC",
  "machine.computer": "dieser Computer",

  // General tab — language
  "general.language.label": "Sprache",
  "general.language.detail": "Gilt für dieses Fenster und das Tray-Menü.",
  "general.language.system": "Wie im System",

  // General tab — updates
  "general.update.label": "Automatisch aktualisieren",
  "general.update.detail": "Installiert neue Versionen im Hintergrund und startet dann neu.",
  "general.update.aria": "Updates automatisch installieren",
  "general.update.version": "Version {{version}}",
  "general.update.checkNow": "Jetzt prüfen",
  "general.update.checkFailed": "Suche nach Updates fehlgeschlagen",
  "general.update.lastChecked": "Zuletzt geprüft um {{time}}",
  "general.update.idle": "Noch nicht geprüft.",
  "general.update.checking": "Suche nach Updates…",
  "general.update.current": "Aktuell.",
  "general.update.downloading": "Wird heruntergeladen… {{percent}}%",
  "general.update.installing": "Wird installiert, dann neu gestartet…",
  "general.update.failed": "Update fehlgeschlagen: {{reason}}",
  "general.update.disabled": "Deaktiviert — es wird nicht nach Updates gesucht.",

  // Schedule
  "schedule.band.unavailable": "Hier nicht verfügbar",
  "schedule.unsupported.generic": "Geplantes Aufwecken ist nur unter macOS verfügbar.",
  "schedule.enable.name": "Computer aufwecken",
  "schedule.enable.hint":
    "Weckt einen schlafenden Mac auf, im Netzbetrieb wie im Akkubetrieb. Ein vollständig heruntergefahrener Mac bleibt aus — ihn wieder einzuschalten ist selbst am Netzteil nicht zuverlässig, daher wird es nicht versucht.",
  "schedule.days.legend": "Tage & Zeiten",
  "schedule.day.mon": "Montag",
  "schedule.day.tue": "Dienstag",
  "schedule.day.wed": "Mittwoch",
  "schedule.day.thu": "Donnerstag",
  "schedule.day.fri": "Freitag",
  "schedule.day.sat": "Samstag",
  "schedule.day.sun": "Sonntag",
  "schedule.day.off": "Aus",
  "schedule.day.toggleAria": "{{day}} umschalten",
  "schedule.time.name": "Uhrzeit",
  "schedule.app.name": "Zu startende App",
  "schedule.app.placeholder": "App wählen",
  "schedule.app.searchPlaceholder": "Apps suchen…",
  "schedule.app.empty": "Keine Apps gefunden",
  "schedule.caveat":
    "Weckt einen schlafenden Mac und öffnet die App zur Zeit des jeweiligen Tages, nur wenn Sie angemeldet sind — ein gesperrter Bildschirm zählt noch, nur ein tatsächliches Abmelden nicht — er funktioniert auch im Akkubetrieb, aber ein vollständig heruntergefahrener Mac bleibt aus, und bei zugeklapptem Deckel braucht die App einen externen Bildschirm, um sich wirklich zu öffnen. Weckvorgänge werden einige Wochen im Voraus geplant, öffnen Sie Agent Profiles also ab und zu, damit sie weiterlaufen.",
  "schedule.coverage.armed":
    "Weckvorgänge sind für die nächsten {{days}} Tage eingerichtet.",
  "schedule.coverage.none":
    "Noch nicht eingerichtet — speichern Sie diesen Zeitplan, um die erste Reihe von Weckvorgängen einzurichten.",
  "schedule.copy.tooltip": "Zeiten kopieren",
  "schedule.copy.aria": "Zeiten von {{day}} auf andere Tage kopieren",
  "schedule.copy.heading": "Zeiten kopieren nach",
  "schedule.copy.everyDay": "Jeden Tag",
  "schedule.copy.apply": "Anwenden",
} as const;
