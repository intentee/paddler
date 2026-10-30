#[derive(Debug, Eq, PartialEq)]
pub enum ResultStep<TPartial, TFinal> {
    Final(TFinal),
    Partial(TPartial),
}
