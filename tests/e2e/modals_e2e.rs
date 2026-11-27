use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use draco::lsp::client::CompletionCandidate;
use draco::plugins::PluginManager;
use draco::settings::Settings;
use draco::theme::Theme;
use draco::ui::command_palette::{self, CommandPaletteState, PaletteCommand, PaletteEntry};
use draco::ui::completion::{self, CompletionState};
use draco::ui::find_bar::{self, FindAction, FindBarState};
use draco::ui::quick_open::{self, QuickOpenState};
use draco::ui::settings_modal::{self, SettingsModalState, SettingsTab};
use draco::workspace::fuzzy::WorkspaceIndexer;
use eframe::egui::{self, Key, Modifiers, Pos2};

use crate::common::{key_press, raw_input, TempDir};

#[test]
fn quick_open_and_completion_keyboard_navigation_e2e() {
    let tmp = TempDir::new("e2e_modals");
    tmp.write_file("src/alpha.rs", "");
    tmp.write_file("src/beta.php", "<?php");

    let indexer = WorkspaceIndexer::default();
    let ready = Arc::new(AtomicBool::new(false));
    let ready_cb = Arc::clone(&ready);
    indexer.start_index(
        tmp.path().to_path_buf(),
        Arc::new(move || ready_cb.store(true, Ordering::SeqCst)),
    );
    let deadline = Instant::now() + Duration::from_secs(3);
    while !ready.load(Ordering::SeqCst) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(ready.load(Ordering::SeqCst));

    let ctx = egui::Context::default();
    let theme = Theme::dark();

    // Quick Open: ArrowDown + Enter selects second item
    let mut qo = QuickOpenState::default();
    qo.show_modal();
    let _ = ctx.run(raw_input(vec![key_press(Key::ArrowDown, Modifiers::NONE)]), |ctx| {
        quick_open::show_quick_open(ctx, &mut qo, &indexer, Some(tmp.path()), &theme);
    });
    assert_eq!(qo.selected, 1);

    let mut chosen: Option<PathBuf> = None;
    let _ = ctx.run(raw_input(vec![key_press(Key::Enter, Modifiers::NONE)]), |ctx| {
        chosen = quick_open::show_quick_open(ctx, &mut qo, &indexer, Some(tmp.path()), &theme);
    });
    assert!(chosen.is_some());
    assert!(!qo.open);

    // Completion popup: ArrowDown + Enter accepts second candidate
    let mut comp = CompletionState::default();
    comp.show_items(vec![
        CompletionCandidate {
            label: "first".into(),
            detail: None,
            insert_text: "first_fn()".into(),
        },
        CompletionCandidate {
            label: "second".into(),
            detail: Some("fn()".into()),
            insert_text: "second_fn()".into(),
        },
    ]);
    let mut accepted = None;
    let _ = ctx.run(
        raw_input(vec![
            key_press(Key::ArrowDown, Modifiers::NONE),
            key_press(Key::Enter, Modifiers::NONE),
        ]),
        |ctx| {
            accepted = completion::show_completion_popup(
                ctx,
                &mut comp,
                Pos2::new(100.0, 100.0),
                &theme,
            );
        },
    );
    assert_eq!(accepted.as_deref(), Some("second_fn()"));
    assert!(!comp.open);
}

#[test]
fn command_palette_settings_modal_and_find_bar_escape_e2e() {
    let ctx = egui::Context::default();
    let theme = Theme::dark();

    // Command Palette executes selected entry
    let mut pal = CommandPaletteState::default();
    pal.show_modal();
    let entries = vec![
        PaletteEntry {
            title: "Disable Auto-Save".into(),
            category: "File".into(),
            command: PaletteCommand::ToggleAutoSave,
        },
        PaletteEntry {
            title: "New Terminal Tab".into(),
            category: "Terminal".into(),
            command: PaletteCommand::NewTerminalTab,
        },
    ];
    let mut ran = None;
    let _ = ctx.run(
        raw_input(vec![
            key_press(Key::ArrowDown, Modifiers::NONE),
            key_press(Key::Enter, Modifiers::NONE),
        ]),
        |ctx| {
            ran = command_palette::show_command_palette(ctx, &mut pal, &entries, &theme);
        },
    );
    assert!(matches!(ran, Some(PaletteCommand::NewTerminalTab)));
    assert!(!pal.open);

    // Settings Modal renders all 3 tabs and closes on Escape
    let mut settings = Settings::default();
    let plugins = PluginManager::new(std::env::temp_dir().join("draco_e2e_no_plugins"));
    let mut modal = SettingsModalState {
        open: true,
        tab: SettingsTab::Editor,
    };
    for tab in [SettingsTab::Editor, SettingsTab::Terminal, SettingsTab::Plugins] {
        modal.tab = tab;
        let out = ctx.run(raw_input(vec![]), |ctx| {
            settings_modal::show_settings_modal(ctx, &mut modal, &mut settings, &plugins, &theme);
        });
        assert!(!out.shapes.is_empty());
    }
    let _ = ctx.run(raw_input(vec![key_press(Key::Escape, Modifiers::NONE)]), |ctx| {
        settings_modal::show_settings_modal(ctx, &mut modal, &mut settings, &plugins, &theme);
    });
    assert!(!modal.open);

    // FindBar closes on Escape
    let mut find = FindBarState {
        open: true,
        show_replace: true,
        query: "fn".into(),
        ..Default::default()
    };
    let mut closed = false;
    let _ = ctx.run(raw_input(vec![key_press(Key::Escape, Modifiers::NONE)]), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            if matches!(find_bar::show_find_bar(ui, &mut find, &theme), Some(FindAction::Close)) {
                closed = true;
            }
        });
    });
    assert!(closed && !find.open);
}
