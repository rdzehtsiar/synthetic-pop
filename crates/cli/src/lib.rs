use synthetic_pop_core::{current_status, CoreEngine, PRODUCT_NAME, PROJECT_PROMISE};
use synthetic_pop_scenarios::FIRST_SCENARIO;

#[must_use]
pub fn placeholder_message() -> String {
    let capabilities = CoreEngine::default().capabilities();
    let core_status = if capabilities.offline_core_available && capabilities.local_core_available {
        "Rust core foundation is available"
    } else {
        "Rust core foundation is not available"
    };
    let deterministic_rng_status = if capabilities.deterministic_rng_available {
        "deterministic RNG is available"
    } else {
        "deterministic RNG is not available"
    };
    format!(
        "{PRODUCT_NAME}: {PROJECT_PROMISE} ({status}).\n\
         {core_status}; {deterministic_rng_status}; Generation commands are not implemented yet.\n\
         Planned first command: {PRODUCT_NAME} generate {FIRST_SCENARIO} --seed demo --users 10000",
        status = current_status().label()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_message_names_the_planned_command() {
        let message = placeholder_message();

        assert!(message.contains("Rust core foundation is available"));
        assert!(message.contains("deterministic RNG is available"));
        assert!(message.contains(current_status().label()));
        assert!(message.contains("synthetic-pop generate forum"));
    }

    #[test]
    fn placeholder_message_does_not_claim_generation_is_available() {
        let message = placeholder_message();

        assert!(message.contains("Generation commands are not implemented yet"));
        assert!(!message.contains("Generation commands are available"));
    }
}
