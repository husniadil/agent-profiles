import type { Strings } from "./index";

/// Indonesian has no plural inflection, so the `.profile`/`.profiles` and
/// `.summaryProfile`/`.summaryProfiles` pairs hold the same string. Deliberate.
export const id: Strings = {
  // Tabs
  "tab.profiles": "Profil Agen",
  "tab.keepAwake": "Tetap Terjaga",
  "tab.schedule": "Jadwal",
  "tab.general": "Umum",

  // Status strip and its tooltip
  "status.profile": "profil",
  "status.profiles": "profil",
  "status.running": "berjalan",
  "status.onDisk": "di disk",
  "status.sizing": "Menghitung ukuran",
  "status.summaryProfile": "{{count}} profil",
  "status.summaryProfiles": "{{count}} profil",
  "status.summaryRunning": "{{count}} berjalan",
  "status.summaryOnDisk": "{{size}} di disk",
  "status.summaryOnDiskApprox": "setidaknya {{size}} di disk",
  "status.sizeAtLeast": "setidaknya {{size}}",
  "status.onDiskApproxWhy":
    "Ada folder yang tidak bisa dibaca, jadi totalnya sebenarnya lebih besar sebanyak jumlah yang tidak diketahui.",
  "status.revealFolder": "Tampilkan folder profil di pengelola berkas: {{path}}",

  // Start-at-login row, in the General tab
  "autostart.label": "Jalankan saat login",
  "autostart.offered": "hanya membuka tray — tidak ada profil yang dijalankan",
  "autostart.unavailable": "tersedia setelah Agent Profiles terpasang",
  "autostart.aria": "Jalankan Agent Profiles saat login",

  // Nothing installed
  "empty.title": "Belum ada yang bisa dibuka",
  "empty.body":
    "Agent Profiles menjalankan agen coding yang sudah terpasang di {{machine}}{{names}}. Pasang salah satu, lalu buka kembali jendela ini.",
  "empty.appsSupported": "{{count}} aplikasi didukung",

  // Add a profile
  "compose.heading": "Profil baru",
  "compose.namePlaceholder": "Beri nama profil ini",
  "compose.nameAria": "Nama profil",
  "compose.appAria": "Aplikasi",
  "compose.add": "Tambah",
  "compose.adding": "Menambahkan",
  "compose.added": "Ditambahkan",
  "compose.retry": "Coba lagi",
  "compose.needName": "Masukkan nama untuk profil ini.",
  "compose.noApp": "Tidak ditemukan aplikasi yang didukung untuk menambahkan profil.",
  "compose.thisApp": "Aplikasi ini",

  // A profile row
  "profiles.empty": "Belum ada profil.",
  "row.running": "Berjalan",
  "row.sharedSignIn": "Masuk bersama",
  "row.open": "Buka {{name}}",
  "row.rename": "Ganti nama {{name}}",
  "row.deleteTrigger": "Hapus {{name}}",
  "row.delete": "Hapus {{name}} secara permanen. Tekan dan tahan untuk konfirmasi.",
  "row.deleteUnavailable":
    "{{name}} adalah instalasi asli aplikasi dan tidak dapat dihapus",
  "row.renameNameAria": "Nama baru untuk {{name}}",
  "row.saveName": "Simpan nama",
  "row.cancel": "Batal",
  "row.holdToDelete": "Tahan untuk menghapus",
  "row.holdingLabel": "Terus tahan…",
  "row.completeLabel": "Menghapus…",
  "row.keepIt": "Simpan saja",
  "row.deleteBody":
    "Hapus {{label}} beserta {{bytes}} di dalam foldernya. Tindakan ini tidak dapat dibatalkan.",

  // Socket path budget
  "budget.aria": "Batas jalur socket",
  "budget.over": "{{bytes}} byte melebihi batas",
  "budget.under": "batas jalur socket · {{system}} berhenti di {{limit}}",
  "budget.ofLimit": " / {{limit}} byte",
  "budget.tooDeep":
    "Folder ini terlalu dalam untuk {{bytes}} byte jalur socket yang dibutuhkan sebuah profil. Tidak ada profil yang dapat ditambahkan di sini.",
  "budget.cannotCreate":
    "{{app}} tidak akan dapat membuat socket-nya di sini. Pindahkan root data ke lokasi dengan jalur yang lebih pendek.",

  // Keep Awake — status card
  "awake.off.title": "Nonaktif",
  "awake.off.detail": "{{machine}} akan tidur seperti biasa saat Anda menutup layar.",
  "awake.idle.title": "Memantau",
  "awake.idle.detail": "Tidak ada yang sedang bekerja saat ini, jadi tidak ada yang ditahan.",
  "awake.holding.title": "Menjaga {{machine}} tetap terjaga",
  "awake.holding.detail":
    "Anda bisa menutup layar — mode tidur akan kembali aktif saat pekerjaan berhenti.",
  "awake.lowBattery.title": "Dijeda — baterai lemah",
  "awake.lowBattery.detail": "Dihentikan sementara untuk melindungi baterai. Sambungkan pengisi daya untuk melanjutkan.",
  "awake.tooHot.title": "Dijeda — {{machine}} terlalu panas",
  "awake.tooHot.detail":
    "Menjaganya tetap terjaga hanya akan memperparah panasnya. Ini akan berlanjut setelah suhunya turun.",
  "awake.stranded":
    "Agent Profiles berhenti secara tak terduga saat sedang menahan mode layar tertutup, dan pengaturan itu tetap bertahan setelah dimulai ulang.",
  "awake.restoreSleep": "Pulihkan mode tidur",
  "awake.needsPassword":
    "Membutuhkan kata sandi administrator sekali setiap kali dijalankan. Sebuah helper mengaktifkan pengaturan ini selama agen bekerja, menonaktifkannya saat berhenti, dan ikut berhenti bersama Agent Profiles.",

  // Keep Awake — status card bands (unsupported, stranded, unauthorized, failed hold)
  "awake.band.unavailable": "Tidak tersedia di sini",
  "awake.band.stranded": "Mac Anda mungkin tidak dapat tidur",
  "awake.band.notAuthorized": "Belum diotorisasi",
  "awake.band.holdFailed": "Tidak menahan — permintaan gagal",
  "awake.band.holdFailedDetail": "{{machine}} akan tidur seperti biasa: {{error}}",
  "awake.unsupported.linux":
    "systemd-inhibit tidak ditemukan, sehingga tidak ada yang bisa mengambil kunci lid-switch di sini. Menahan layar tertutup membutuhkan desktop yang menjalankan systemd-logind.",
  "awake.unsupported.generic":
    "{{system}} pada {{machine}} melaporkan tidak dapat menahan layar tertutup.",
  "awake.authorize": "Otorisasi…",

  // Keep Awake — status card's assembled status line
  "awake.status.noBattery": "Tanpa baterai",
  "awake.status.battery": "Baterai {{percent}}%",
  "awake.status.pluggedIn": ", tersambung ke daya",
  "awake.status.held": " · tertahan {{duration}}",

  // Keep Awake — section legends
  "awake.section.hold": "Tahan perangkat agar tetap terjaga",
  "awake.section.limits": "Batasan",
  "awake.section.watching": "Pemantauan",

  // Keep Awake — low-battery control
  "awake.battery.name": "Jeda saat baterai lemah",
  "awake.battery.aria": "Jeda saat baterai lemah",
  "awake.battery.below": "di bawah {{percent}}%",

  // Keep Awake — thermal guard
  "awake.thermal.name": "Pengaman suhu",
  "awake.thermal.aria": "Pengaman suhu",

  // Keep Awake — hint paragraphs under each Limits setting
  "awake.hint.noBattery": "{{machine}} tidak memiliki baterai, jadi ini tidak pernah berlaku.",
  "awake.hint.lowBattery":
    "Turun di bawah level ini, bahkan di tengah pekerjaan. Diabaikan saat tersambung ke daya.",
  "awake.hint.idleWindow":
    "Agen yang telah menyelesaikan gilirannya langsung melepaskan {{machine}}. Ini hanya membatasi agen yang berhenti di tengah jalan: setelah tidak menulis apa pun selama waktu ini, agen dianggap sudah tidak ada, bukan sedang bekerja.",
  "awake.hint.thermal":
    "Lepaskan penahanan saat perangkat melaporkan kepanasan.",

  // Keep Awake — triggers and limits
  "awake.trigger.off": "Nonaktif",
  "awake.trigger.agentActive": "Saat agen sedang bekerja",
  "awake.trigger.agentActiveDetail":
    "Sesi Claude Code atau Codex yang sedang ditulis.",
  "awake.trigger.always": "Selalu selama Agent Profiles berjalan",
  "awake.trigger.alwaysDetail":
    "Untuk agen di dalam aplikasi desktop, yang tidak memiliki apa pun untuk dideteksi.",
  "awake.limit.idleWindow": "Berhenti menunggu agen yang diam setelah",
  "awake.limit.minutes": "mnt",
  "awake.limit.aria": "{{label}} ({{unit}})",

  // Keep Awake — watch list
  "awake.watch.empty":
    "Belum ada yang dipantau. Claude Code dan Codex akan ditemukan secara otomatis setelah menulis sebuah sesi.",
  "awake.watch.working": "Bekerja",
  "awake.watch.never": "tidak pernah",
  "awake.watch.ago": "{{duration}} lalu",
  "awake.watch.stalled": "macet {{duration}}",
  "awake.watch.idle": "diam {{duration}}",

  // Machine and system names — "this Mac" reads differently in six languages
  "system.macos": "macOS",
  "system.windows": "Windows",
  "system.linux": "Linux",
  "system.unknown": "sistem ini",
  "machine.mac": "Mac ini",
  "machine.pc": "PC ini",
  "machine.computer": "komputer ini",

  // General tab — language
  "general.language.label": "Bahasa",
  "general.language.detail": "Berlaku untuk jendela ini dan menu tray.",
  "general.language.system": "Sama seperti sistem",

  // General tab — updates
  "general.update.label": "Perbarui secara otomatis",
  "general.update.detail": "Memasang rilis baru di latar belakang, lalu memulai ulang.",
  "general.update.aria": "Pasang pembaruan secara otomatis",
  "general.update.version": "Versi {{version}}",
  "general.update.checkNow": "Periksa sekarang",
  "general.update.checkFailed": "Tidak dapat memeriksa pembaruan",
  "general.update.lastChecked": "Terakhir diperiksa {{time}}",
  "general.update.idle": "Belum diperiksa.",
  "general.update.checking": "Memeriksa pembaruan…",
  "general.update.current": "Sudah versi terbaru.",
  "general.update.downloading": "Mengunduh… {{percent}}%",
  "general.update.installing": "Memasang, lalu memulai ulang…",
  "general.update.failed": "Tidak dapat memperbarui: {{reason}}",
  "general.update.disabled": "Nonaktif — tidak ada rilis yang diperiksa.",

  // Schedule
  "schedule.band.unavailable": "Tidak tersedia di sini",
  "schedule.unsupported.generic": "Bangun terjadwal hanya tersedia di macOS.",
  "schedule.enable.name": "Bangunkan komputer",
  "schedule.days.legend": "Hari & waktu",
  "schedule.day.mon": "Senin",
  "schedule.day.tue": "Selasa",
  "schedule.day.wed": "Rabu",
  "schedule.day.thu": "Kamis",
  "schedule.day.fri": "Jumat",
  "schedule.day.sat": "Sabtu",
  "schedule.day.sun": "Minggu",
  "schedule.day.off": "Nonaktif",
  "schedule.day.toggleAria": "Alihkan {{day}}",
  "schedule.time.name": "Waktu",
  "schedule.app.name": "Aplikasi yang dijalankan",
  "schedule.app.placeholder": "Pilih aplikasi",
  "schedule.app.searchPlaceholder": "Cari aplikasi…",
  "schedule.app.empty": "Aplikasi tidak ditemukan",
  "schedule.caveat":
    "Membangunkan Mac yang tidur dan membuka aplikasi pada waktu setiap hari, hanya jika Anda masih masuk — Mac bangun sekitar satu menit lebih awal dan bisa bekerja dengan baterai, tetapi Mac yang benar-benar dimatikan akan tetap mati, dan jika layar tertutup, aplikasi memerlukan monitor eksternal agar benar-benar terbuka. Bangun dijadwalkan beberapa minggu ke depan, jadi buka Agent Profiles sesekali agar tetap berjalan.",
  "schedule.copy.tooltip": "Salin waktu",
  "schedule.copy.aria": "Salin waktu {{day}} ke hari lain",
  "schedule.copy.heading": "Salin waktu ke",
  "schedule.copy.everyDay": "Setiap hari",
  "schedule.copy.apply": "Terapkan",
} as const;
