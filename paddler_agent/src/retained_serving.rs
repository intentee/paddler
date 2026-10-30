use crate::loaded_model_serving_plan::LoadedModelServingPlan;
use crate::model_serving_settings::ModelServingSettings;

pub enum RetainedServing {
    LoadedModel(LoadedModelServingPlan),
    ModelToLoad(ModelServingSettings),
}
