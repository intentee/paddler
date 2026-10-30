use paddler_agent_runtime::shuts_down::ShutsDown;

pub enum CapabilityPlan<TServed> {
    Keep(TServed),
    Start,
}

impl<TServed> CapabilityPlan<TServed>
where
    TServed: ShutsDown + Send,
{
    pub async fn retaining<TSettings: PartialEq>(
        served: TServed,
        served_settings: &TSettings,
        desired_settings: &TSettings,
    ) -> Self {
        if served_settings == desired_settings {
            Self::Keep(served)
        } else {
            served.shut_down().await;

            Self::Start
        }
    }
}

impl<TServed> ShutsDown for CapabilityPlan<TServed>
where
    TServed: ShutsDown + Send,
{
    async fn shut_down(self) {
        match self {
            Self::Keep(served) => served.shut_down().await,
            Self::Start => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use super::CapabilityPlan;
    use crate::served_decisions::ServedDecisions;

    #[tokio::test]
    async fn keeps_a_capability_whose_settings_did_not_change() {
        assert_eq!(
            discriminant(&CapabilityPlan::retaining(ServedDecisions::Disabled, &1, &1).await),
            discriminant(&CapabilityPlan::Keep(ServedDecisions::Disabled))
        );
    }

    #[tokio::test]
    async fn restarts_a_capability_whose_settings_changed() {
        assert_eq!(
            discriminant(&CapabilityPlan::retaining(ServedDecisions::Disabled, &1, &2).await),
            discriminant(&CapabilityPlan::<ServedDecisions>::Start)
        );
    }
}
