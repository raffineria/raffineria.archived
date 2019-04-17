use crate::futures::fsm::*;
use futures::prelude::*;

use super::*;

impl IntoFuture for GraphRunner {
    type Future = FSMFuture<Self>;
    type Item = <Self as FSM>::Item;
    type Error = <Self as FSM>::Error;

    fn into_future(self) -> Self::Future {
        trace!("<GraphRunner as IntoFuture>::into_future(...)");
        Self::Future::from(self)
    }
}
