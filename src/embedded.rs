//! Файлы игры, вшитые в бинарник (см. `build.rs`).

include!(concat!(env!("OUT_DIR"), "/embedded.rs"));

/// Содержимое файла по пути относительно корня проекта, например `resources/bg.jpg`.
pub fn file(path: &str) -> &'static [u8] {
    FILES
        .iter()
        .find(|(p, _)| *p == path)
        .map(|(_, data)| *data)
        .unwrap_or_else(|| panic!("нет встроенного файла {path}"))
}
