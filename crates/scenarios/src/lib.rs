pub const FIRST_SCENARIO: &str = "forum";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioState {
    Planned,
}

#[must_use]
pub const fn first_scenario_state() -> ScenarioState {
    ScenarioState::Planned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_scenario_matches_the_initial_wedge() {
        assert_eq!(FIRST_SCENARIO, "forum");
        assert_eq!(first_scenario_state(), ScenarioState::Planned);
    }
}

