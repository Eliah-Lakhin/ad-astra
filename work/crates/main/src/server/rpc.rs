////////////////////////////////////////////////////////////////////////////////
// This file is part of "Ad Astra", an embeddable scripting programming       //
// language platform.                                                         //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, or contribute to this work, you must agree to    //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/ad-astra/blob/master/EULA.md               //
//                                                                            //
// The agreement grants a Basic Commercial License, allowing you to use       //
// this work in non-commercial and limited commercial products with a total   //
// gross revenue cap. To remove this commercial limit for one of your         //
// products, you must acquire a Full Commercial License.                      //
//                                                                            //
// If you contribute to the source code, documentation, or related materials, //
// you must grant me an exclusive license to these contributions.             //
// Contributions are governed by the "Contributions" section of the General   //
// License Agreement.                                                         //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted       //
// under the General License Agreement.                                       //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is", without any warranties, express or implied, //
// except where such disclaimers are legally invalid.                         //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

/////////////////////////////////////////////////////////////////////////////////////////////////////////////
// A part of this file's source code is an adaptation of Alex Kladov's and the authors'                    //
// "lsp-server" work.                                                                                      //
//                                                                                                         //
// The original work by Alex Kladov and the authors is available here:                                     //
// https://github.com/rust-lang/rust-analyzer/tree/a84685a58d3ce833d8d2517f0b9069569edd16db/lib/lsp-server //
//                                                                                                         //
// Alex Kladov and the authors provided their work under the following terms:                              //
//                                                                                                         //
//   Permission is hereby granted, free of charge, to any                                                  //
//   person obtaining a copy of this software and associated                                               //
//   documentation files (the "Software"), to deal in the                                                  //
//   Software without restriction, including without                                                       //
//   limitation the rights to use, copy, modify, merge,                                                    //
//   publish, distribute, sublicense, and/or sell copies of                                                //
//   the Software, and to permit persons to whom the Software                                              //
//   is furnished to do so, subject to the following                                                       //
//   conditions:                                                                                           //
//                                                                                                         //
//   The above copyright notice and this permission notice                                                 //
//   shall be included in all copies or substantial portions                                               //
//   of the Software.                                                                                      //
//                                                                                                         //
//   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF                                                 //
//   ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED                                               //
//   TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A                                                   //
//   PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT                                                   //
//   SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY                                              //
//   CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION                                               //
//   OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR                                               //
//   IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER                                                   //
//   DEALINGS IN THE SOFTWARE.                                                                             //
//                                                                                                         //
// Kindly be advised that the terms governing the distribution of my work are                              //
// distinct from those pertaining to the original "lsp-server" work.                                       //
/////////////////////////////////////////////////////////////////////////////////////////////////////////////

use std::{
    fmt::{Debug, Formatter},
    io,
    sync::{
        atomic::{AtomicI64, Ordering},
        mpsc::{channel, Receiver, Sender},
    },
};

use ahash::RandomState;
use compact_str::CompactString;
use lady_deirdre::{
    analysis::TaskHandle,
    sync::{Shared, Table, Trigger},
};
use log::{error, trace};
use lsp_types::{
    notification::{Exit, Notification},
    request::Request,
    NumberOrString,
};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{from_slice, from_value, to_value, to_vec, Value};

use crate::{report::system_panic, server::logger::RPC_LOG};

/// A sender end of the RPC channel.
///
/// You can create this object using the [RpcMessage::channel] function.
pub type RpcSender = Sender<RpcMessage>;

/// A receiver end of the RPC channel.
///
/// You can create this object using the [RpcMessage::channel] function.
pub type RpcReceiver = Receiver<RpcMessage>;

/// A message in the client-server communication channel.
///
/// This object represents an RPC message in the
/// [JSON-RPC 2.0](https://www.jsonrpc.org/specification) protocol, which serves
/// as the basis for the
/// [LSP](https://microsoft.github.io/language-server-protocol/) protocol.
///
/// You can create this object manually by deserializing a string containing the
/// RPC message using the [RpcMessage::from_input_bytes] function.
///
/// To serialize the message back into a string, use the
/// [RpcMessage::to_output_bytes] function.
#[derive(Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct RpcMessage(pub(super) RpcMessageInner);

#[derive(Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub(super) enum RpcMessageInner {
    Request(RpcRequest),
    Response(RpcResponse),
    Notification(RpcNotification),
}

impl Debug for RpcMessage {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            RpcMessageInner::Request(message) => Debug::fmt(message, formatter),
            RpcMessageInner::Response(message) => Debug::fmt(message, formatter),
            RpcMessageInner::Notification(message) => Debug::fmt(message, formatter),
        }
    }
}

impl From<RpcRequest> for RpcMessage {
    #[inline(always)]
    fn from(value: RpcRequest) -> Self {
        Self(RpcMessageInner::Request(value))
    }
}

impl From<RpcResponse> for RpcMessage {
    #[inline(always)]
    fn from(value: RpcResponse) -> Self {
        Self(RpcMessageInner::Response(value))
    }
}

impl From<RpcNotification> for RpcMessage {
    #[inline(always)]
    fn from(value: RpcNotification) -> Self {
        Self(RpcMessageInner::Notification(value))
    }
}

impl Serialize for RpcMessage {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        static TAG: &'static str = "2.0";

        #[derive(Serialize)]
        struct TaggedRequest<'a> {
            #[serde(rename(serialize = "jsonrpc"))]
            tag: &'static str,
            #[serde(flatten)]
            #[serde(rename(serialize = "msg"))]
            message: &'a RpcRequest,
        }

        #[derive(Serialize)]
        struct TaggedResponse<'a> {
            #[serde(rename(serialize = "jsonrpc"))]
            tag: &'static str,
            #[serde(flatten)]
            #[serde(rename(serialize = "msg"))]
            message: &'a RpcResponse,
        }

        #[derive(Serialize)]
        struct TaggedNotification<'a> {
            #[serde(rename(serialize = "jsonrpc"))]
            tag: &'static str,
            #[serde(flatten)]
            #[serde(rename(serialize = "msg"))]
            message: &'a RpcNotification,
        }

        match &self.0 {
            RpcMessageInner::Request(message) => {
                TaggedRequest { tag: TAG, message }.serialize(serializer)
            }
            RpcMessageInner::Response(message) => {
                TaggedResponse { tag: TAG, message }.serialize(serializer)
            }
            RpcMessageInner::Notification(message) => {
                TaggedNotification { tag: TAG, message }.serialize(serializer)
            }
        }
    }
}

impl RpcMessage {
    /// A helper function that creates a local RPC communication [channel].
    ///
    /// This function is useful for creating a channel for outgoing server
    /// messages when you manually instantiate the server using the
    /// [LspServer::new](crate::server::LspServer::new) constructor.
    #[inline(always)]
    pub fn channel() -> (RpcSender, RpcReceiver) {
        channel()
    }

    /// Returns true if this message is a client's notification to the server
    /// to shut down.
    pub fn is_exit(&self) -> bool {
        let RpcMessage(RpcMessageInner::Notification(notification)) = &self else {
            return false;
        };

        notification.is::<Exit>()
    }

    /// Deserializes an array of `bytes` representing a UTF-8 encoded RPC
    /// message string.
    ///
    /// The resulting object is expected to be an incoming client message
    /// that should be handled by the server using the
    /// [LspServer::handle](crate::server::LspServer::handle) function.
    ///
    /// The function returns None if deserialization fails. In this case,
    /// it prints an error message to the server's log.
    #[inline(always)]
    pub fn from_input_bytes(bytes: &[u8]) -> Option<Self> {
        let result: Self = match from_slice(bytes) {
            Ok(result) => result,
            Err(error) => {
                error!(target: RPC_LOG, ">> Body deserialization error: {error}");
                return None;
            }
        };

        trace!(target: RPC_LOG, ">> {:?}", result);

        Some(result)
    }

    /// Serializes this message into a UTF-8 encoded RPC message string.
    ///
    /// The serialized message is intended to be an outgoing server message
    /// to the client.
    ///
    /// The function returns None if serialization fails. In this case,
    /// an error message is printed to the server's log.
    #[inline(always)]
    pub fn to_output_bytes(&self) -> Option<Vec<u8>> {
        let bytes = match to_vec(&self) {
            Ok(body) => body,
            Err(error) => {
                error!(target: RPC_LOG, "<< Body serialization error: {error}");
                return None;
            }
        };

        Some(bytes)
    }

    pub(super) fn read(mut read: impl io::BufRead) -> Option<Self> {
        let mut body_length = None;
        let mut buffer = String::new();

        loop {
            buffer.clear();

            match read.read_line(&mut buffer) {
                Ok(0) => {
                    return None;
                }

                Ok(_) => (),

                Err(error) => {
                    error!(target: RPC_LOG, ">> Incoming stream error: {error}");
                    return None;
                }
            }

            if !buffer.ends_with("\r\n") {
                error!(
                    target: RPC_LOG,
                    ">> Header read error. Missing eol suffix in the header line: {buffer:?}.",
                );
                return None;
            }

            let line = &buffer[0..buffer.len() - 2];

            if line.is_empty() {
                break;
            }

            let mut key_value = line.splitn(2, ": ");

            let (Some(key), Some(value)) = (key_value.next(), key_value.next()) else {
                continue;
            };

            if key.eq_ignore_ascii_case("content-length") {
                let Ok(value) = value.parse::<usize>() else {
                    error!(
                        target: RPC_LOG,
                        ">> Header read error. Invalid Content-Length header entry format: {line:?}.",
                    );
                    return None;
                };

                body_length = Some(value);
            }

            buffer.clear();
        }

        let Some(length) = body_length else {
            error!(target: RPC_LOG, ">> Header read error. Missing Content-Length header entry.");
            return None;
        };

        let mut buffer = buffer.into_bytes();

        buffer.resize(length, 0);

        if let Err(error) = read.read_exact(&mut buffer) {
            error!(target: RPC_LOG, ">> Body read error: {error}");
            return None;
        }

        Self::from_input_bytes(&buffer)
    }

    pub(super) fn write(&self, mut write: impl io::Write) -> bool {
        let body = match to_vec(self) {
            Ok(body) => body,
            Err(error) => {
                error!(target: RPC_LOG, "<< Body serialization error: {error}");
                return false;
            }
        };

        let header = format!(
            "Content-Length: {}\r\n\
                Content-Type: application/vscode-jsonrpc; charset=utf-8\r\n\r\n",
            body.len(),
        );

        if let Err(error) = write.write_all(header.as_bytes()) {
            error!(target: RPC_LOG, "<< Header write error: {error}");
            return false;
        }

        if let Err(error) = write.write_all(&body) {
            error!(target: RPC_LOG, "<< Body write error: {error}");
            return false;
        }

        trace!(target: RPC_LOG, "<< {:?}", self);

        true
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct RpcRequest {
    pub(super) id: RpcId,
    pub(super) method: CompactString,
    #[serde(default = "Value::default")]
    #[serde(skip_serializing_if = "Value::is_null")]
    pub(super) params: Value,
}

impl Debug for RpcRequest {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let alternate = formatter.alternate();

        let mut debug_struct = formatter.debug_struct("RpcRequest");

        debug_struct
            .field("id", &self.id)
            .field("method", &self.method);

        if alternate {
            return debug_struct.field("params", &self.params).finish();
        }

        debug_struct.finish_non_exhaustive()
    }
}

impl RpcRequest {
    pub(super) fn new<T: Request>(params: T::Params) -> Self {
        static CLIENT_ID: AtomicI64 = AtomicI64::new(0);

        let id = CLIENT_ID.fetch_add(1, Ordering::SeqCst);

        let params = match to_value::<T::Params>(params) {
            Ok(params) => params,
            Err(error) => {
                system_panic!("RpcRequest serialization failure. {error}",)
            }
        };

        Self {
            id: RpcId(RpcIdInner::Number(id)),
            method: CompactString::from(T::METHOD),
            params,
        }
    }

    #[inline(always)]
    pub(super) fn is<T: Request>(&self) -> bool {
        self.method == T::METHOD
    }

    #[track_caller]
    #[inline(always)]
    pub(super) fn extract<T: Request>(self) -> (RpcId, T::Params) {
        let params = match from_value(self.params) {
            Ok(params) => params,
            Err(error) => panic!("RpcRequest deserialization failure. {error}",),
        };

        (self.id, params)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct RpcResponse {
    pub(super) id: RpcId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<RpcError>,
}

impl Debug for RpcResponse {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let alternate = formatter.alternate();

        let mut debug_struct = formatter.debug_struct("RpcResponse");

        debug_struct.field("id", &self.id);

        match alternate {
            true => debug_struct.field("result", &self.result),
            false => debug_struct.field("result", &self.result.is_some()),
        };

        match alternate {
            true => debug_struct.field("error", &self.error),
            false => debug_struct.field("error", &self.error.is_some()),
        };

        match alternate {
            true => debug_struct.finish(),
            false => debug_struct.finish_non_exhaustive(),
        }
    }
}

impl RpcResponse {
    #[inline(always)]
    pub(super) fn ok(id: RpcId, result: impl Serialize) -> Self {
        let result = match to_value(result) {
            Ok(result) => result,
            Err(error) => {
                system_panic!("RpcResponse result serialization failure. {error}",);
            }
        };

        Self {
            id,
            result: Some(result),
            error: None,
        }
    }

    #[inline(always)]
    pub(super) fn err(id: RpcId, code: i64, message: impl Into<CompactString>) -> Self {
        let error = RpcError {
            code,
            message: message.into(),
            data: None,
        };

        Self {
            id,
            result: None,
            error: Some(error),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub(super) struct RpcError {
    pub(super) code: i64,
    pub(super) message: CompactString,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) data: Option<Value>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct RpcNotification {
    pub(super) method: CompactString,
    #[serde(default = "Value::default")]
    #[serde(skip_serializing_if = "Value::is_null")]
    pub(super) params: Value,
}

impl Debug for RpcNotification {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let alternate = formatter.alternate();

        let mut debug_struct = formatter.debug_struct("RpcNotification");

        debug_struct.field("method", &self.method);

        if alternate {
            return debug_struct.field("params", &self.params).finish();
        }

        debug_struct.finish_non_exhaustive()
    }
}

impl RpcNotification {
    #[inline(always)]
    pub(super) fn new<N: Notification>(params: N::Params) -> Self {
        let params = match to_value(params) {
            Ok(result) => result,
            Err(error) => {
                system_panic!("RpcNotification result serialization failure. {error}",);
            }
        };

        Self {
            method: CompactString::from(N::METHOD),
            params,
        }
    }

    #[inline(always)]
    pub(super) fn is<T: Notification>(&self) -> bool {
        self.method == T::METHOD
    }

    #[track_caller]
    #[inline(always)]
    pub(super) fn extract<T: Notification>(self) -> T::Params {
        match from_value(self.params) {
            Ok(params) => params,
            Err(error) => system_panic!("RpcNotification deserialization failure. {error}",),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct RpcId(RpcIdInner);

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
enum RpcIdInner {
    Number(i64),
    String(CompactString),
}

impl From<NumberOrString> for RpcId {
    #[inline(always)]
    fn from(value: NumberOrString) -> Self {
        match value {
            NumberOrString::Number(id) => Self(RpcIdInner::Number(id as i64)),
            NumberOrString::String(string) => Self(RpcIdInner::String(CompactString::from(string))),
        }
    }
}

impl Debug for RpcId {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            RpcIdInner::Number(id) => Debug::fmt(id, formatter),
            RpcIdInner::String(id) => Debug::fmt(id, formatter),
        }
    }
}

pub(super) type RpcLatches = Shared<Table<RpcId, Trigger, RandomState>>;

#[derive(Default, Clone)]
pub(super) struct LspHandle {
    server: Trigger,
    client: Option<Trigger>,
}

impl TaskHandle for LspHandle {
    #[inline(always)]
    fn is_triggered(&self) -> bool {
        if self.server.is_active() {
            return true;
        }

        let Some(client) = &self.client else {
            return false;
        };

        client.is_active()
    }

    #[inline(always)]
    fn trigger(&self) {
        self.server.activate();
    }
}

impl LspHandle {
    #[inline(always)]
    pub fn new(lsp_latch: &Trigger) -> Self {
        Self {
            server: Trigger::new(),
            client: Some(lsp_latch.clone()),
        }
    }
}

pub(super) trait OutgoingEx: Sized {
    fn send_ok_response<R: Request>(&self, latches: &RpcLatches, id: RpcId, result: R::Result);

    fn send_err_response(
        &self,
        latches: &RpcLatches,
        id: RpcId,
        code: i64,
        message: impl AsRef<str>,
    );

    fn notify<N: Notification>(&self, params: N::Params);

    fn request<R: Request>(&self, params: R::Params);
}

impl OutgoingEx for RpcSender {
    #[track_caller]
    #[inline(always)]
    fn send_ok_response<R: Request>(&self, latches: &RpcLatches, id: RpcId, result: R::Result) {
        let _ = latches.as_ref().remove(&id);

        let message = RpcMessage::from(RpcResponse::ok(id, result));

        if self.send(message).is_err() {
            error!(target: RPC_LOG, "Outgoing channel closed.");
        }
    }

    #[track_caller]
    #[inline(always)]
    fn send_err_response(
        &self,
        latches: &RpcLatches,
        id: RpcId,
        code: i64,
        message: impl AsRef<str>,
    ) {
        let _ = latches.as_ref().remove(&id);

        let message = RpcMessage::from(RpcResponse::err(id, code, message.as_ref()));

        if self.send(message).is_err() {
            error!(target: RPC_LOG, "Outgoing channel closed.");
        }
    }

    #[track_caller]
    #[inline(always)]
    fn notify<N: Notification>(&self, params: N::Params) {
        let message = RpcMessage::from(RpcNotification::new::<N>(params));

        if self.send(message).is_err() {
            error!(target: RPC_LOG, "Outgoing channel closed.");
        }
    }

    #[track_caller]
    #[inline(always)]
    fn request<R: Request>(&self, params: R::Params) {
        let message = RpcMessage::from(RpcRequest::new::<R>(params));

        if self.send(message).is_err() {
            error!(target: RPC_LOG, "Outgoing channel closed.");
        }
    }
}
