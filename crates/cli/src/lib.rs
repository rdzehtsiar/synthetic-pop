use synthetic_pop_core::{current_status, PRODUCT_NAME, PROJECT_PROMISE};
use synthetic_pop_scenarios::FIRST_SCENARIO;

#[must_use]
pub fn placeholder_message() -> String {
    format!(
        "{PRODUCT_NAME}: {PROJECT_PROMISE} ({status}).\n\
         Generation commands are not implemented yet.\n\
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

        assert!(message.contains("Generation commands are not implemented yet"));
        assert!(message.contains("synthetic-pop generate forum"));
    }
}

