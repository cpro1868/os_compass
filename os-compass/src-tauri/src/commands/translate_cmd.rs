use crate::translate::{detect_language, needs_translation, translate_text, translate_text_with_llm};

#[tauri::command]
pub async fn translate(text: String, target_lang: String) -> Result<String, String> {
    translate_text(&text, &target_lang).await
}

#[tauri::command]
pub async fn translate_with_llm(text: String, target_lang: String) -> Result<String, String> {
    translate_text_with_llm(&text, &target_lang).await
}

#[tauri::command]
pub async fn detect_text_language(text: String) -> Result<String, String> {
    detect_language(&text).await
}

#[tauri::command]
pub fn should_translate(source_lang: Option<String>, target_lang: String) -> bool {
    needs_translation(source_lang.as_deref(), &target_lang)
}
