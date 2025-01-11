use serde::{Serialize, de::DeserializeOwned};

pub trait Subscription: DeserializeOwned {
    const TYPE: &'static str;
    const VERSION: &'static str;

    type Condition: Serialize + DeserializeOwned;
}
