//! Whether an answer says it changed something, or that it will: "I am
//! proposing the edit", "I have written SiteHost.rr", "here is the updated
//! code", "I ran it", "I'll add the list" — in English, Spanish and
//! Portuguese. A small model often says so without
//! calling a tool, and then nothing reached the user: the assistant asks it
//! once to make the change for real (`rai`), and says plainly when it
//! still didn't. A short list on purpose: it only has to catch the usual
//! ways of saying it, and a miss only means no second chance.

/// Phrases that claim a change was made, proposed or run (lower case, `'`
/// for any apostrophe).
const CLAIMS: &[&str] = &[
    // (English: proposing, writing, changing, running)
    "i am proposing", "i'm proposing", "i have proposed", "i've proposed", "i proposed",
    "i am writing", "i'm writing", "i have written", "i've written", "i wrote",
    "i have updated", "i've updated", "i updated", "i am updating", "i'm updating",
    "i have created", "i've created", "i created",
    "i have modified", "i've modified", "i modified",
    "i have added", "i've added",
    "i have applied", "i've applied", "i applied",
    "i have implemented", "i've implemented", "i implemented",
    "i have made the change", "i've made the change", "i have made these change", "i've made these change",
    "i have fixed", "i've fixed", "i fixed", "i have corrected", "i've corrected", "i corrected", "i have replaced", "i've replaced", "i replaced",
    "i am committing", "i'm committing", "i have committed", "i've committed", "i am submitting", "i'm submitting", "i have submitted", "i've submitted",
    "i am making the", "i'm making the", "i am now ", "i'm now ", "i have now ", "i've now ",
    // (saying it will, and not doing it: a tool call was due)
    "i will now ", "i'll now ", "i will create", "i'll create", "i will build", "i'll build", "i will write", "i'll write", "i will generate", "i'll generate",
    "i will implement", "i'll implement", "i will add", "i'll add", "i will update", "i'll update", "i will modify", "i'll modify", "i will propose", "i'll propose",
    "let me create", "let me write", "let me build", "let me implement", "let me add", "let me update",
    "here is the updated", "here's the updated", "here is the new ", "here's the new ", "here is the complete", "here's the complete", "here is the modified", "here's the modified", "here is the full", "here's the full", "here is the revised", "here's the revised",
    "has been applied", "have been applied", "has been updated", "have been updated", "has been created", "have been created",
    "i am running", "i'm running", "i ran ", "i have run", "i've run", "i have tested", "i've tested", "i tested",
    "please review the proposed", "please review the diff", "please review the change", "please review it", "review the proposed",
    // (Spanish)
    "propongo", "he propuesto", "estoy proponiendo", "te propuse", "he escrito", "escribí", "he creado", "he actualizado", "he modificado", "he añadido", "he agregado", "he aplicado", "he implementado",
    "aquí está el código actualizado", "aquí está el código completo", "aquí tienes el código actualizado", "aquí tienes el código completo",
    "voy a crear", "voy a escribir", "voy a modificar", "voy a actualizar", "voy a añadir", "voy a agregar", "voy a implementar",
    "he ejecutado", "lo ejecuté", "revisa el cambio", "revisa los cambios",
    // (Portuguese)
    "estou propondo", "eu propus", "propus ", "escrevi", "criei", "atualizei", "modifiquei", "adicionei", "apliquei", "implementei",
    "aqui está o código atualizado", "aqui está o código completo",
    "vou criar", "vou escrever", "vou modificar", "vou atualizar", "vou adicionar", "vou implementar",
    "executei", "revise a alteração", "revise as alterações",
];

/// Whether `text` says a change was made, proposed or run.
pub fn claims_change(text: &str) -> bool {
    let low = text.to_lowercase().replace(['\u{2019}', '\u{2018}', '`'], "'");
    CLAIMS.iter().any(|c| low.contains(c))
}

/// Phrases of a user telling the assistant to go on and make the change.
const GO_AHEAD: &[&str] = &[
    "go ahead", "just do", "do it", "implement", "continue", "write all", "write it", "keep working", "keep going", "proceed", "carry on", "go on", "build it", "make it",
    "adelante", "hazlo", "hágalo", "continúa", "continua", "sigue", "implementa", "escríbelo", "dale",
    "vai em frente", "faça", "pode fazer", "prossiga", "siga",
];

/// Short answers that agree, when a message starts with one ("Yes, let's
/// go", "Ok").
const AGREE: &[&str] = &["yes", "ok", "okay", "sure", "yep", "please do", "sí", "si", "sim", "claro", "vale"];

/// Whether the user's message (a short one) tells the assistant to go on:
/// "Sure, go ahead, write all", "continue", "Ok", "Just implement" — then
/// an answer with no change in it missed what was asked.
pub fn go_ahead(question: &str) -> bool {
    let low = question.trim().to_lowercase().replace(['\u{2019}', '\u{2018}'], "'");
    if low.chars().count() > 120 || low.contains('?') {
        return false;
    }
    let first: String = low.chars().take_while(|c| c.is_alphanumeric() || *c == ' ' || *c == '\'').collect();
    let first = first.trim();
    GO_AHEAD.iter().any(|g| low.contains(g)) || AGREE.iter().any(|a| first == *a || first.starts_with(&format!("{a} ")) || low.starts_with(&format!("{a},")) || low.starts_with(&format!("{a}.")) || low.starts_with(&format!("{a}!")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claims() {
        for yes in [
            "I am proposing the edit to SiteHost.rr. Please review it.",
            "I’ve updated main.rr with the Start button.",
            "Here is the updated code:\n```basic\n…",
            "I ran the program and it works.",
            "Perfecto, he escrito el módulo del servidor.",
            "Voy a crear el formulario principal.",
            "Pronto! Atualizei o arquivo main.rr.",
            "The change has been applied.",
            "Okay, I will add a field for the number of people. Shall I go on?",
        ] {
            assert!(claims_change(yes), "{yes}");
        }
        for no in [
            "The Start button calls StartServer, which opens the port.",
            "I can't run programs: press Run (F5) to try it.",
            "Which port should the server use?",
            "I haven't changed anything yet: shall I add the list?",
            "",
        ] {
            assert!(!claims_change(no), "{no}");
        }
    }

    #[test]
    fn going_ahead() {
        for yes in ["Sure, go ahead, write all.", "continue", "Ok", "Yes, let's go step by step. Simple first.", "Just implement. Keep working in the real thing.", "Don't ask that much. Just do!", "Implement the server. Structure everything module by module, then we go step by step.", "Sí, hazlo", "ok!"] {
            assert!(go_ahead(yes), "{yes}");
        }
        for no in ["What does the Start button do?", "Build me a tip calculator? Not yet, first explain RForm.", "Make the total bigger and bold", "okra soup recipe", "Explain the timer", ""] {
            assert!(!go_ahead(no), "{no}");
        }
    }
}
