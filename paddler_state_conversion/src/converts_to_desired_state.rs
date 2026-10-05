pub trait ConvertsToDesiredState {
    type DesiredState;
    type Source;

    fn to_desired_state(&self, source: Self::Source) -> Self::DesiredState;
}
