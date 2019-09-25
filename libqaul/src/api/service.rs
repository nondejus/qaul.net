//! Defines a basic interface for a Qaul service

use super::models::Message;
use std::any::Any;

pub trait Service: Any {
    fn receive_message(&self, message: Message);
}

pub trait ServiceName: Service {
    fn name() -> String;
}

pub trait InterserviceMessenger: Service {
    type Input;
    type Output;

    fn receive_ism(&self, message: Self::Input) -> Self::Output;
}
