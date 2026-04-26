use crate::config::Preset;

pub fn build_system_prompt(language: &str, preset: &Preset) -> String {
    format!(
        "Translate this text into {}.\n\nStyle and quality requirements:\n{}\n\nHard rules:\n- Return only the translated text.\n- Preserve paragraph breaks.\n- Preserve markdown structure.\n- Preserve URLs, code blocks, variables, placeholders, numbers, and product names unless translation is clearly required.\n- Do not explain the translation.",
        language.trim(),
        preset.instruction.trim()
    )
}
