use futures::prelude::*;

pub enum TurnOk<S: FSM> {
    PollMore(S),
    Suspend(S),
    Ready(S::Item),
}

pub type TurnResult<S> = Result<TurnOk<S>, <S as FSM>::Error>;

pub trait FSM: Sized {
    type Item;
    type Error;

    fn turn(self) -> TurnResult<Self>;

    fn into_fsm_future(self) -> FSMFuture<Self> {
        self.into()
    }
}

pub struct FSMFuture<S: FSM> {
    state_opt: Option<S>,
}

impl<S: FSM> From<S> for FSMFuture<S> {
    fn from(state: S) -> Self {
        Self {
            state_opt: Some(state),
        }
    }
}

impl<S> Future for FSMFuture<S>
where
    S: FSM,
{
    type Item = S::Item;
    type Error = S::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Some(mut state) = self.state_opt.take() {
            loop {
                match state.turn() {
                    Ok(TurnOk::PollMore(state_next)) => {
                        state = state_next;
                    }
                    Ok(TurnOk::Suspend(state_next)) => {
                        self.state_opt = Some(state_next);
                        return Ok(Async::NotReady);
                    }
                    Ok(TurnOk::Ready(value)) => return Ok(Async::Ready(value)),
                    Err(reason) => return Err(reason),
                }
            }
        } else {
            panic!("this future is ready and cannot be polled")
        }
    }
}
