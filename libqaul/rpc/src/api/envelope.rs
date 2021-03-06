use super::*;
use libqaul::{
    api::SubId,
    contacts::ContactEntry,
    messages::{Message, MsgId, MsgRef},
    users::{UserAuth, UserProfile},
    Identity,
};

#[feature(chat)]
use qaul_chat::{
    room::{Room, RoomId},
    Chat, ChatMessage,
};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt::Display};

/// Represents a libqaul RPC request envelope
///
/// Because in some rpc systems requests will be processed in a
/// non-knowable order, making it hard to associtate requests with
/// responses.  This is what the request ID is for, and should be set,
/// even on systems that don't have this problem.
#[derive(Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub id: String,
    pub data: EnvelopeType,
}

/// A generic wrapper for requests and responses
///
/// In the rpc layer, the return data is then namespaced as "request"
/// and "response", which should be used to disambiguate data on the
/// wire.
#[derive(Clone, Serialize, Deserialize)]
pub enum EnvelopeType {
    /// A libqaul request
    Request(Request),
    /// A libqaul response
    Response(Response),
}

/// A wrapper enum to disambiguate request types in the envelope.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Request {
    /// Poll the next chat message
    #[cfg(features = "chat")]
    ChatMsgNext(chat::messages::Next),

    /// Create a subscription for chat messages
    #[cfg(features = "chat")]
    ChatMsgSub(chat::messages::Subscribe),

    /// Send a chat message
    #[cfg(features = "chat")]
    ChatMsgSend(chat::messages::Send),

    /// Query the chat message store
    #[cfg(features = "chat")]
    ChatMsgQuery,

    /// List all available chat rooms
    #[cfg(features = "chat")]
    ChatRoomList(chat::rooms::List),

    /// Get data about a chat room
    #[cfg(features = "chat")]
    ChatRoomGet(chat::rooms::Get),

    /// Create a new chat room
    #[cfg(features = "chat")]
    ChatRoomCreate(chat::rooms::Create),

    /// Modify a chat room
    #[cfg(features = "chat")]
    ChatRoomModify(chat::rooms::Modify),

    /// Delete a chat room
    #[cfg(features = "chat")]
    ChatRoomDelete(chat::rooms::Delete),

    /// Modify a user's contact
    ContactModify(contacts::Modify),

    /// Get a user contact
    ContactGet(contacts::Get),

    /// Query a user's contacts
    ContactQuery(contacts::Query),

    /// Get all user contacts
    ContactAll(contacts::All),

    /// Send a raw libqaul message
    MsgSend(messages::Send),

    /// Poll the next raw libqaul message
    MsgNext(messages::Next),

    /// Create a subscription for raw libqaul messages
    MsgSub(messages::Subscribe),

    /// Query existing raw libqaul messages
    MsgQuery(messages::Query),

    /// List local available users
    UserList(users::List),

    /// Create a new user
    UserCreate(users::Create),

    /// Delete a local user
    UserDelete(users::Delete),

    /// Change a user's passphrase
    UserChangePw(users::ChangePw),

    /// Login as a user to get an auth token
    UserLogin(users::Login),

    /// End a user session
    UserLogout(users::Logout),

    /// Get data on a particular user
    UserGet(users::Get),

    /// Update a user
    UserUpdate(users::Update),
}

/// Wrap around all possible response values for piped Rpc protocols
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Response {
    /// Return an auth object
    Auth(UserAuth),

    /// Return a set of chat messages
    #[cfg(features = "chat")]
    ChatMessage(Vec<ChatMessage>),

    /// Return a set of contact entries
    Contact(Vec<ContactEntry>),

    /// Return an error type
    Error(String),

    /// Return a set of message
    Message(Vec<Message>),

    /// Return a message ID
    MsgId(MsgId),

    /// Return chat room data
    #[cfg(features = "chat")]
    Room(Room),

    /// Get a set of chat room IDs
    #[cfg(features = "chat")]
    RoomId(Vec<RoomId>),

    /// Confirmation for a new subscription
    Subscription(SubId),

    /// A generic success message
    Success,

    /// Return a set of user profiles
    User(Vec<UserProfile>),

    /// Return available user IDs
    UserId(Vec<Identity>),
}

impl From<UserAuth> for Response {
    fn from(auth: UserAuth) -> Self {
        Response::Auth(auth)
    }
}

#[cfg(features = "chat")]
impl From<ChatMessage> for Response {
    fn from(msg: ChatMessage) -> Self {
        Response::ChatMessage(vec![msg])
    }
}

#[cfg(features = "chat")]
impl From<Vec<ChatMessage>> for Response {
    fn from(msgs: Vec<ChatMessage>) -> Self {
        Response::ChatMessage(msgs)
    }
}

impl From<ContactEntry> for Response {
    fn from(contact: ContactEntry) -> Self {
        Response::Contact(vec![contact])
    }
}

impl From<Vec<ContactEntry>> for Response {
    fn from(contacts: Vec<ContactEntry>) -> Self {
        Response::Contact(contacts)
    }
}

impl<T: Into<Response>, E: Display> From<Result<T, E>> for Response {
    fn from(result: Result<T, E>) -> Self {
        match result {
            Ok(t) => t.into(),
            Err(e) => Response::Error(e.to_string()),
        }
    }
}

impl From<MsgRef> for Response {
    fn from(msg: MsgRef) -> Self {
        Response::Message(vec![msg.as_ref().clone()])
    }
}

impl From<Vec<MsgRef>> for Response {
    fn from(msgs: Vec<MsgRef>) -> Self {
        Response::Message(msgs.into_iter().map(|msg| msg.as_ref().clone()).collect())
    }
}

#[cfg(features = "chat")]
impl From<Room> for Response {
    fn from(room: Room) -> Self {
        Response::Room(room)
    }
}

impl From<()> for Response {
    fn from(_: ()) -> Self {
        Self::Success
    }
}

impl From<UserProfile> for Response {
    fn from(user: UserProfile) -> Self {
        Response::User(vec![user])
    }
}

impl From<Vec<UserProfile>> for Response {
    fn from(users: Vec<UserProfile>) -> Self {
        Response::User(users)
    }
}

impl From<Vec<Identity>> for Response {
    fn from(ids: Vec<Identity>) -> Self {
        Self::UserId(ids)
    }
}
