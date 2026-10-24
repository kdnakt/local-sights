# Functional Design — 質問（U1 言語切替、u1-language-switch）

元にしたもの：`inception/units-generation/unit-of-work.md`（U1）・`unit-of-work-story-map.md`、`inception/requirements-analysis/requirements.md`（FR1.1〜FR1.6、NFR1・NFR3・NFR4・NFR6、RA Q1・Q2）、`inception/domain-design/components.md`（AppSettings・AppSession・MessageCatalog・AppShell・SettingsDialog・TauriApp・ApiBridge）・`decisions.md`（ADR-005）、今のコード（`cache/settings.rs` の SettingsFile・SettingsLoad・load_settings・save_settings、`session.rs` の SettingsWork・SettingsNotice・SessionView、`src/main.tsx` の detectLocale と I18nProvider、`src/components/SettingsDialog.tsx`、`src/i18n/regions.ts` の regionLabel）。

要件分析（RA Q1 A：設定ダイアログ、Q2 A：覚える・既定は OS に合わせる）とドメイン設計（ADR-005）、作業単位の分割のレビュー（R-01：言語の値の読みはキャッシュの項目と独立、LanguageSetting は実行時の写し）で、この作業単位の決めるべき点は決まっている。残る細部（設定ファイルの値の綴り、`formatVersion` を上げるかどうか、選択肢の表示名、設定が届く前の最初の描画、OS の言語が起動中に変わったとき）は、利用者の世界の事実ではなく設計の判断で決められるため、質問は設けず、`rules.md` と `functional-spec.md` の「決めたこと」と「前提」に書いた。

質問：なし。
