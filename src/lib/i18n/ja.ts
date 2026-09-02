import type { Strings } from "./index";

/// Japanese has no plural inflection, so the `.profile`/`.profiles` and
/// `.summaryProfile`/`.summaryProfiles` pairs hold the same string. Deliberate.
export const ja: Strings = {
  // Tabs
  "tab.profiles": "エージェントプロファイル",
  "tab.keepAwake": "スリープ防止",
  "tab.schedule": "スケジュール",
  "tab.general": "一般",

  // Status strip and its tooltip
  "status.profile": "プロファイル",
  "status.profiles": "プロファイル",
  "status.running": "実行中",
  "status.onDisk": "ディスク使用量",
  "status.sizing": "計測中",
  "status.summaryProfile": "{{count}} 件のプロファイル",
  "status.summaryProfiles": "{{count}} 件のプロファイル",
  "status.summaryRunning": "実行中 {{count}} 件",
  "status.summaryOnDisk": "ディスク使用量 {{size}}",
  "status.summaryOnDiskApprox": "ディスク使用量 {{size}} 以上",
  "status.sizeAtLeast": "{{size}} 以上",
  "status.onDiskApproxWhy":
    "読み取れなかったフォルダーがあるため、実際の使用量はこれより不明な分だけ大きくなります。",
  "status.revealFolder": "ファイルマネージャーでプロファイルフォルダを表示: {{path}}",

  // Start-at-login row, in the General tab
  "autostart.label": "ログイン時に起動",
  "autostart.offered": "トレイのみ開きます — プロファイルは起動しません",
  "autostart.unavailable": "Agent Profiles をインストールすると利用できます",
  "autostart.aria": "ログイン時に Agent Profiles を起動",

  // Nothing installed
  "empty.title": "まだ開くものがありません",
  "empty.body":
    "Agent Profiles は、{{machine}}{{names}} にインストール済みのコーディングエージェントを実行します。いずれかをインストールしてから、このウィンドウを開き直してください。",
  "empty.appsSupported": "{{count}} 個のアプリに対応",

  // Add a profile
  "compose.heading": "新しいプロファイル",
  "compose.namePlaceholder": "プロファイル名を入力",
  "compose.nameAria": "プロファイル名",
  "compose.appAria": "アプリ",
  "compose.add": "追加",
  "compose.adding": "追加中",
  "compose.added": "追加しました",
  "compose.retry": "再試行",
  "compose.needName": "プロファイルの名前を入力してください。",
  "compose.noApp": "プロファイルを追加できる対応アプリが見つかりませんでした。",
  "compose.thisApp": "このアプリ",

  // A profile row
  "profiles.empty": "プロファイルはまだありません。",
  "profiles.unavailable": "利用できません: {{reason}}",
  "row.running": "実行中",
  "row.sharedSignIn": "サインインを共有",
  "row.open": "{{name}} を開く",
  "row.rename": "{{name}} の名前を変更",
  "row.deleteTrigger": "{{name}} を削除",
  "row.delete": "{{name}} を完全に削除します。長押しで確定します。",
  "row.deleteUnavailable":
    "{{name}} はアプリ本体のインストールのため削除できません",
  "row.renameNameAria": "{{name}} の新しい名前",
  "row.saveName": "名前を保存",
  "row.cancel": "キャンセル",
  "row.holdToDelete": "長押しで削除",
  "row.holdingLabel": "そのまま押し続けてください…",
  "row.completeLabel": "削除中…",
  "row.keepIt": "残す",
  "row.deleteBody":
    "{{label}} とそのフォルダ内の {{bytes}} を削除します。元に戻せません。",

  // Socket path budget
  "budget.aria": "ソケットパスの上限",
  "budget.over": "上限を {{bytes}} バイト超過",
  "budget.under": "ソケットパスの上限 · {{system}} では {{limit}} まで",
  "budget.ofLimit": " / {{limit}} バイト",
  "budget.tooDeep":
    "このフォルダは、プロファイルに必要なソケットパスの {{bytes}} バイトに対して深すぎます。ここにはプロファイルを追加できません。",
  "budget.cannotCreate":
    "{{app}} はここにソケットを作成できません。データルートをより短いパスに移動してください。",

  // Keep Awake — status card
  "awake.off.title": "オフ",
  "awake.off.detail": "{{machine}} は、いつも通りふたを閉じるとスリープします。",
  "awake.idle.title": "監視中",
  "awake.idle.detail": "現在動作しているものがないため、スリープは保持されていません。",
  "awake.holding.title": "{{machine}} を起動状態に保持中",
  "awake.holding.detail":
    "ふたを閉じても構いません — 作業が終わるとスリープに戻ります。",
  "awake.lowBattery.title": "一時停止 — バッテリー残量低下",
  "awake.lowBattery.detail": "バッテリーを保護するため停止しました。電源に接続すると再開します。",
  "awake.tooHot.title": "一時停止 — {{machine}} が高温です",
  "awake.tooHot.detail":
    "起動状態を保持すると悪化するおそれがあります。冷えれば再開します。",
  "awake.stranded":
    "Agent Profiles はふたを閉じた状態を保持したまま予期せず終了しました。この設定は再起動後も維持されます。",
  "awake.restoreSleep": "スリープを復元",
  "awake.needsPassword":
    "実行ごとに一度、管理者パスワードが必要です。ヘルパーがエージェントの動作中は設定をオンにし、停止するとオフに戻し、Agent Profiles と共に終了します。",

  // Keep Awake — status card bands (unsupported, stranded, unauthorized, failed hold)
  "awake.band.unavailable": "ここでは利用できません",
  "awake.band.stranded": "この Mac はスリープできない可能性があります",
  "awake.band.notAuthorized": "まだ許可されていません",
  "awake.band.holdFailed": "保持されていません — リクエストが失敗しました",
  "awake.band.holdFailedDetail": "{{machine}} はいつも通りスリープします: {{error}}",
  "awake.unsupported.linux":
    "systemd-inhibit が見つからないため、ふたスイッチのロックを取得できません。ふたを閉じた状態を保持するには、systemd-logind が動作するデスクトップ環境が必要です。",
  "awake.unsupported.generic":
    "{{machine}} の {{system}} は、ふたを閉じた状態を保持できないと報告しています。",
  "awake.authorize": "認証…",

  // Keep Awake — status card's assembled status line
  "awake.status.noBattery": "バッテリーなし",
  "awake.status.battery": "バッテリー {{percent}}%",
  "awake.status.pluggedIn": "、電源接続中",
  "awake.status.held": " · {{duration}} 保持",

  // Keep Awake — section legends
  "awake.section.hold": "起動状態を保持",
  "awake.section.limits": "制限",
  "awake.section.watching": "監視対象",

  // Keep Awake — low-battery control
  "awake.battery.name": "バッテリー低下時に一時停止",
  "awake.battery.aria": "バッテリー低下時に一時停止",
  "awake.battery.below": "{{percent}}% 未満",

  // Keep Awake — thermal guard
  "awake.thermal.name": "サーマルガード",
  "awake.thermal.aria": "サーマルガード",

  // Keep Awake — hint paragraphs under each Limits setting
  "awake.hint.noBattery": "{{machine}} にはバッテリーがないため、これは適用されません。",
  "awake.hint.lowBattery":
    "作業中であってもこの残量を下回ると停止します。電源接続中は無視されます。",
  "awake.hint.idleWindow":
    "ターンを終えたエージェントは即座に {{machine}} を解放します。これは途中で止まったエージェントにのみ適用され、この時間だけ何も書き込まれないと、動作中ではなく終了したものとして扱われます。",
  "awake.hint.thermal":
    "マシンが過熱を報告した場合は保持を解除します。",

  // Keep Awake — triggers and limits
  "awake.trigger.off": "オフ",
  "awake.trigger.agentActive": "エージェントが動作中のとき",
  "awake.trigger.agentActiveDetail":
    "Claude Code または Codex のセッションに書き込みがある間。",
  "awake.trigger.always": "Agent Profiles の実行中は常に",
  "awake.trigger.alwaysDetail":
    "検出手段のない、デスクトップアプリ内のエージェント向けです。",
  "awake.limit.idleWindow": "エージェントが無応答になってから諦めるまで",
  "awake.limit.minutes": "分",
  "awake.limit.aria": "{{label}}（{{unit}}）",

  // Keep Awake — watch list
  "awake.watch.empty":
    "監視対象はまだありません。Claude Code と Codex は、セッションへの書き込みが行われると自動的に検出されます。",
  "awake.watch.working": "動作中",
  "awake.watch.never": "なし",
  "awake.watch.ago": "{{duration}}前",
  "awake.watch.stalled": "{{duration}} 停止中",
  "awake.watch.idle": "{{duration}} アイドル",

  // Machine and system names — "this Mac" reads differently in six languages
  "system.macos": "macOS",
  "system.windows": "Windows",
  "system.linux": "Linux",
  "system.unknown": "このシステム",
  "machine.mac": "この Mac",
  "machine.pc": "この PC",
  "machine.computer": "このコンピュータ",

  // General tab — language
  "general.language.label": "言語",
  "general.language.detail": "このウィンドウとトレイメニューに適用されます。",
  "general.language.system": "システムに合わせる",

  // General tab — updates
  "general.update.label": "自動的に更新",
  "general.update.detail": "バックグラウンドで新しいリリースをインストールし、再起動します。",
  "general.update.aria": "更新を自動的にインストール",
  "general.update.version": "バージョン {{version}}",
  "general.update.checkNow": "今すぐ確認",
  "general.update.checkFailed": "アップデートを確認できませんでした",
  "general.update.lastChecked": "最終確認 {{time}}",
  "general.update.idle": "まだ確認していません。",
  "general.update.checking": "更新を確認中…",
  "general.update.current": "最新の状態です。",
  "general.update.downloading": "ダウンロード中… {{percent}}%",
  "general.update.installing": "インストール中、その後再起動します…",
  "general.update.failed": "更新できませんでした: {{reason}}",
  "general.update.disabled": "オフになっています — リリースは確認されません。",

  // Schedule
  "schedule.band.unavailable": "ここでは利用できません",
  "schedule.unsupported.generic": "スケジュール起動は macOS でのみ利用できます。",
  "schedule.enable.name": "コンピュータを復帰",
  "schedule.enable.hint":
    "スリープ中の Mac を、AC電源でもバッテリーでも復帰させます。完全にシャットダウンした Mac はそのまま起動しません — 電源につないでいても再起動は信頼できないため、試みません。",
  "schedule.days.legend": "曜日と時刻",
  "schedule.day.mon": "月曜日",
  "schedule.day.tue": "火曜日",
  "schedule.day.wed": "水曜日",
  "schedule.day.thu": "木曜日",
  "schedule.day.fri": "金曜日",
  "schedule.day.sat": "土曜日",
  "schedule.day.sun": "日曜日",
  "schedule.day.off": "オフ",
  "schedule.day.toggleAria": "{{day}}を切り替え",
  "schedule.time.name": "時刻",
  "schedule.app.name": "起動するアプリ",
  "schedule.app.placeholder": "アプリを選択",
  "schedule.app.searchPlaceholder": "アプリを検索…",
  "schedule.app.empty": "アプリが見つかりません",
  "schedule.caveat":
    "各曜日の時刻に、スリープ中の Mac を復帰させ、ログイン中の場合のみアプリを起動します(画面がロックされていても該当しますが、実際にログアウトしている場合は該当しません)。バッテリーでも動作しますが、完全にシャットダウンしている Mac はそのまま起動しません。またふたを閉じている場合、外部ディスプレイがないとアプリは実際には開きません。復帰は数週間先まで予約されるため、継続させるには時々 Agent Profiles を開いてください。",
  "schedule.coverage.armed": "今後 {{days}} 日分の復帰が予約されています。",
  "schedule.coverage.none":
    "まだ予約されていません — このスケジュールを保存すると最初の復帰が予約されます。",
  "schedule.copy.tooltip": "時刻をコピー",
  "schedule.copy.aria": "{{day}}の時刻を他の曜日にコピー",
  "schedule.copy.heading": "時刻のコピー先",
  "schedule.copy.everyDay": "毎日",
  "schedule.copy.apply": "適用",
} as const;
