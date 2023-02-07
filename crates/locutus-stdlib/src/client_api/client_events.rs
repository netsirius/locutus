use std::fmt::Display;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::host_response_generated::schemas::host::{
    ContractContainerArgs, ContractResponseArgs, ContractResponseType, ContractV1Args,
    DeltaUpdateArgs, DeltaWithContractInstanceUpdateArgs, GetResponseArgs, HostResponseArgs,
    HostResponseType, KeyArgs, PutResponseArgs, StateArgs, StateDeltaArgs, StateDeltaUpdateArgs,
    StateDeltaWithContractInstanceUpdateArgs, StateSummaryArgs, StateUpdateArgs,
    StateWithContractInstanceUpdateArgs, UpdateDataArgs, UpdateDataType, UpdateNotificationArgs,
    UpdateResponseArgs,
};
use crate::prelude::ContractContainer::Wasm;
use crate::prelude::UpdateData::{
    Delta, RelatedDelta, RelatedState, RelatedStateAndDelta, State, StateAndDelta,
};
use crate::prelude::WasmAPIVersion;
use crate::{
    client_request_generated::schemas::client::{
        root_as_client_request, ClientRequestType, ContractRequest as FbsContractRequest,
        ContractRequestType,
    },
    component_interface::{Component, ComponentKey, InboundComponentMsg, OutboundComponentMsg},
    host_response_generated::schemas::host::{
        ContractContainer as FbsContractContainer, ContractInstanceId as FbsContractInstanceId,
        ContractInstanceIdArgs, ContractResponse as FbsContractResponse,
        ContractV1 as FbsContractV1, DeltaUpdate as FbsDeltaUpdate,
        DeltaWithContractInstanceUpdate as FbsDeltaWithContractInstanceUpdate,
        GetResponse as FbsGetResponse, HostResponse as FbsHostResponse, Key as FbsKey,
        PutResponse as FbsPutResponse, State as FbsState, StateDelta as FbsStateDelta,
        StateDeltaUpdate as FbsStateDeltaUpdate,
        StateDeltaWithContractInstanceUpdate as FbsStateDeltaWithContractInstanceUpdate,
        StateSummary as FbsStateSummary, StateUpdate as FbsStateUpdate,
        StateWithContractInstanceUpdate as FbsStateWithContractInstanceUpdate,
        UpdateData as FbsUpdateData, UpdateNotification as FbsUpdateNotification,
        UpdateResponse as FbsUpdateResponse, WasmContract as FbsWasmContract,
    },
    prelude::{
        ContractKey, RelatedContracts, StateSummary, TryFromTsStd, UpdateData, WrappedState,
        WsApiError,
    },
    versioning::ContractContainer,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientError {
    kind: ErrorKind,
}

impl ClientError {
    pub fn kind(&self) -> ErrorKind {
        self.kind.clone()
    }
}

impl From<ErrorKind> for ClientError {
    fn from(kind: ErrorKind) -> Self {
        ClientError { kind }
    }
}

impl From<String> for ClientError {
    fn from(cause: String) -> Self {
        ClientError {
            kind: ErrorKind::Unhandled { cause },
        }
    }
}

#[derive(thiserror::Error, Debug, Serialize, Deserialize, Clone)]
pub enum ErrorKind {
    #[error("comm channel between client/host closed")]
    ChannelClosed,
    #[error("error while deserializing: {cause}")]
    DeserializationError { cause: String },
    #[error("client disconnected")]
    Disconnect,
    #[error("failed while trying to unpack state for {0}")]
    IncorrectState(ContractKey),
    #[error("node not available")]
    NodeUnavailable,
    #[error("undhandled error: {0}")]
    Other(String),
    #[error("lost the connection with the protocol hanling connections")]
    TransportProtocolDisconnect,
    #[error("unhandled error: {cause}")]
    Unhandled { cause: String },
    #[error("unknown client id: {0}")]
    UnknownClient(usize),
}

impl Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "client error: {}", self.kind)
    }
}

impl std::error::Error for ClientError {}

/// A request from a client application to the host.
#[derive(Serialize, Deserialize, Debug, Clone)]
// #[cfg_attr(test, derive(arbitrary::Arbitrary))]
pub enum ClientRequest<'a> {
    ComponentOp(#[serde(borrow)] ComponentRequest<'a>),
    ContractOp(#[serde(borrow)] ContractRequest<'a>),
    GenerateRandData { bytes: usize },
    Disconnect { cause: Option<String> },
}

impl ClientRequest<'_> {
    pub fn into_owned(self) -> ClientRequest<'static> {
        match self {
            ClientRequest::ContractOp(op) => {
                let owned = match op {
                    ContractRequest::Put {
                        contract,
                        state,
                        related_contracts,
                    } => {
                        let related_contracts = related_contracts.into_owned();
                        ContractRequest::Put {
                            contract,
                            state,
                            related_contracts,
                        }
                    }
                    ContractRequest::Update { key, data } => {
                        let data = data.into_owned();
                        ContractRequest::Update { key, data }
                    }
                    ContractRequest::Get {
                        key,
                        fetch_contract,
                    } => ContractRequest::Get {
                        key,
                        fetch_contract,
                    },
                    ContractRequest::Subscribe { key } => ContractRequest::Subscribe { key },
                };
                owned.into()
            }
            ClientRequest::ComponentOp(op) => {
                let op = op.into_owned();
                ClientRequest::ComponentOp(op)
            }
            ClientRequest::GenerateRandData { bytes } => ClientRequest::GenerateRandData { bytes },
            ClientRequest::Disconnect { cause } => ClientRequest::Disconnect { cause },
        }
    }

    pub fn is_disconnect(&self) -> bool {
        matches!(self, Self::Disconnect { .. })
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum ContractRequest<'a> {
    /// Insert a new value in a contract corresponding with the provided key.
    Put {
        contract: ContractContainer,
        /// Value to upsert in the contract.
        state: WrappedState,
        /// Related contracts.
        #[serde(borrow)]
        related_contracts: RelatedContracts<'a>,
    },
    /// Update an existing contract corresponding with the provided key.
    Update {
        key: ContractKey,
        #[serde(borrow)]
        data: UpdateData<'a>,
    },
    /// Fetch the current state from a contract corresponding to the provided key.
    Get {
        /// Key of the contract.
        key: ContractKey,
        /// If this flag is set then fetch also the contract itself.
        fetch_contract: bool,
    },
    /// Subscribe to the changes in a given contract. Implicitly starts a get operation
    /// if the contract is not present yet.
    Subscribe { key: ContractKey },
}

impl<'a> From<ContractRequest<'a>> for ClientRequest<'a> {
    fn from(op: ContractRequest<'a>) -> Self {
        ClientRequest::ContractOp(op)
    }
}

impl<'a> TryFromTsStd<&[u8]> for ClientRequest<'a> {
    fn try_decode(msg: &[u8]) -> Result<Self, WsApiError> {
        let req: ClientRequest = {
            match root_as_client_request(msg) {
                Ok(r) => match r.client_request_type() {
                    ClientRequestType::ContractRequest => {
                        let contract_request = r.client_request_as_contract_request().unwrap();
                        ContractRequest::try_decode(&contract_request)?.into()
                    }
                    ClientRequestType::ComponentRequest => todo!(),
                    ClientRequestType::GenerateRandData => todo!(),
                    ClientRequestType::Disconnect => todo!(),
                    _ => unreachable!(),
                },
                Err(e) => {
                    return Err(WsApiError::FlatbufferDecodeError {
                        cause: format!("{e}"),
                    })
                }
            }
        };

        Ok(req.into_owned())
    }
}

/// Deserializes a `ContractRequest` from a MessagePack encoded request.
impl<'a> TryFromTsStd<&FbsContractRequest<'a>> for ContractRequest<'a> {
    fn try_decode(request: &FbsContractRequest) -> Result<Self, WsApiError> {
        let req: ContractRequest = {
            match request.contract_request_type() {
                ContractRequestType::Get => {
                    let get = request.contract_request_as_get().unwrap();
                    let key = ContractKey::try_decode(&get.key())?;
                    let fetch_contract = get.fetch_contract();
                    ContractRequest::Get {
                        key,
                        fetch_contract,
                    }
                }
                ContractRequestType::Put => {
                    let put = request.contract_request_as_put().unwrap();
                    let contract = ContractContainer::try_decode(&put.container())?;
                    let state = WrappedState::try_decode(&put.state())?;
                    let related_contracts =
                        RelatedContracts::try_decode(&put.related_contracts())?.into_owned();
                    ContractRequest::Put {
                        contract,
                        state,
                        related_contracts,
                    }
                }
                ContractRequestType::Update => {
                    let update = request.contract_request_as_update().unwrap();
                    let key = ContractKey::try_decode(&update.key())?;
                    let data = UpdateData::try_decode(&update.data())?.into_owned();
                    ContractRequest::Update { key, data }
                }
                ContractRequestType::Subscribe => {
                    let subscribe = request.contract_request_as_subscribe().unwrap();
                    let key = ContractKey::try_decode(&subscribe.key())?;
                    ContractRequest::Subscribe { key }
                }
                _ => unreachable!(),
            }
        };

        Ok(req)
    }
}

impl<'a> From<ComponentRequest<'a>> for ClientRequest<'a> {
    fn from(op: ComponentRequest<'a>) -> Self {
        ClientRequest::ComponentOp(op)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ComponentRequest<'a> {
    ApplicationMessages {
        key: ComponentKey,
        inbound: Vec<InboundComponentMsg<'a>>,
    },
    RegisterComponent {
        #[serde(borrow)]
        component: Component<'a>,
        cipher: [u8; 24],
        nonce: [u8; 24],
    },
    UnregisterComponent(ComponentKey),
}

impl ComponentRequest<'_> {
    pub fn into_owned(self) -> ComponentRequest<'static> {
        match self {
            ComponentRequest::ApplicationMessages { key, inbound } => {
                ComponentRequest::ApplicationMessages {
                    key,
                    inbound: inbound.into_iter().map(|e| e.into_owned()).collect(),
                }
            }
            ComponentRequest::RegisterComponent {
                component,
                cipher,
                nonce,
            } => {
                let component = component.into_owned();
                ComponentRequest::RegisterComponent {
                    component,
                    cipher,
                    nonce,
                }
            }
            ComponentRequest::UnregisterComponent(key) => {
                ComponentRequest::UnregisterComponent(key)
            }
        }
    }
}

impl Display for ClientRequest<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientRequest::ContractOp(ops) => match ops {
                ContractRequest::Put {
                    contract, state, ..
                } => {
                    write!(f, "put request for contract {contract} with state {state}")
                }
                ContractRequest::Update { key, .. } => write!(f, "Update request for {key}"),
                ContractRequest::Get {
                    key,
                    fetch_contract: contract,
                    ..
                } => {
                    write!(f, "get request for {key} (fetch full contract: {contract})")
                }
                ContractRequest::Subscribe { key, .. } => write!(f, "subscribe request for {key}"),
            },
            ClientRequest::ComponentOp(_op) => write!(f, "component request"),
            ClientRequest::Disconnect { .. } => write!(f, "client disconnected"),
            ClientRequest::GenerateRandData { bytes } => write!(f, "generate {bytes} random bytes"),
        }
    }
}

/// A response to a previous [`ClientRequest`]
#[derive(Serialize, Deserialize, Debug)]
pub enum HostResponse<T = WrappedState, U = Vec<u8>> {
    ContractResponse(#[serde(bound(deserialize = "T: DeserializeOwned"))] ContractResponse<T>),
    ComponentResponse {
        key: ComponentKey,
        values: Vec<OutboundComponentMsg>,
    },
    GenerateRandData(U),
    /// A requested action which doesn't require an answer was performed successfully.
    Ok,
}

impl HostResponse {
    pub fn unwrap_put(self) -> ContractKey {
        if let Self::ContractResponse(ContractResponse::PutResponse { key }) = self {
            key
        } else {
            panic!("called `HostResponse::unwrap_put()` on other than `PutResponse` value")
        }
    }

    pub fn unwrap_get(self) -> (WrappedState, Option<ContractContainer>) {
        if let Self::ContractResponse(ContractResponse::GetResponse {
            contract, state, ..
        }) = self
        {
            (state, contract)
        } else {
            panic!("called `HostResponse::unwrap_put()` on other than `PutResponse` value")
        }
    }

    pub fn to_fbs_bytes(self) -> Vec<u8> {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        match self {
            HostResponse::ContractResponse(res) => match res {
                ContractResponse::PutResponse { key } => {
                    let data = builder.create_vector(&key.bytes());
                    let instance = FbsContractInstanceId::create(
                        &mut builder,
                        &ContractInstanceIdArgs { data: Some(data) },
                    );
                    let key = FbsKey::create(
                        &mut builder,
                        &KeyArgs {
                            instance: Some(instance),
                            code: None,
                        },
                    );
                    let put =
                        FbsPutResponse::create(&mut builder, &PutResponseArgs { key: Some(key) });
                    let put_response = FbsContractResponse::create(
                        &mut builder,
                        &ContractResponseArgs {
                            contract_response_type: ContractResponseType::PutResponse,
                            contract_response: Some(put.as_union_value()),
                        },
                    );
                    let host_response = FbsHostResponse::create(
                        &mut builder,
                        &HostResponseArgs {
                            response_type: HostResponseType::ContractResponse,
                            response: Some(put_response.as_union_value()),
                        },
                    );
                    builder.finish(host_response, None);
                    builder.finished_data().to_vec()
                }
                ContractResponse::UpdateResponse { key, summary } => {
                    let data = builder.create_vector(&key.bytes());
                    let instance = FbsContractInstanceId::create(
                        &mut builder,
                        &ContractInstanceIdArgs { data: Some(data) },
                    );
                    let key = FbsKey::create(
                        &mut builder,
                        &KeyArgs {
                            instance: Some(instance),
                            code: None,
                        },
                    );
                    let data = builder.create_vector(&summary.into_bytes());
                    let summary = FbsStateSummary::create(
                        &mut builder,
                        &StateSummaryArgs { data: Some(data) },
                    );
                    let update = FbsUpdateResponse::create(
                        &mut builder,
                        &UpdateResponseArgs {
                            key: Some(key),
                            summary: Some(summary),
                        },
                    );
                    let update_response = FbsContractResponse::create(
                        &mut builder,
                        &ContractResponseArgs {
                            contract_response_type: ContractResponseType::UpdateResponse,
                            contract_response: Some(update.as_union_value()),
                        },
                    );
                    let host_response = FbsHostResponse::create(
                        &mut builder,
                        &HostResponseArgs {
                            response_type: HostResponseType::ContractResponse,
                            response: Some(update_response.as_union_value()),
                        },
                    );
                    builder.finish(host_response, None);
                    builder.finished_data().to_vec()
                }
                ContractResponse::GetResponse { state, contract } => {
                    let state_data = builder.create_vector(&state.to_vec());
                    let state = FbsState::create(
                        &mut builder,
                        &StateArgs {
                            data: Some(state_data),
                        },
                    );
                    let contract = contract.unwrap();
                    let data = builder.create_vector(&contract.key().bytes());
                    let instance = FbsContractInstanceId::create(
                        &mut builder,
                        &ContractInstanceIdArgs { data: Some(data) },
                    );
                    let key = FbsKey::create(
                        &mut builder,
                        &KeyArgs {
                            instance: Some(instance),
                            code: None,
                        },
                    );
                    let contract_data = builder.create_vector(&contract.data());
                    let contract_params = builder.create_vector(&contract.params().into_bytes());
                    let contract_version = builder.create_string(&contract.version());
                    let contract = match contract {
                        Wasm(WasmAPIVersion::V1(..)) => FbsContractV1::create(
                            &mut builder,
                            &ContractV1Args {
                                key: Some(key),
                                data: Some(contract_data),
                                parameters: Some(contract_params),
                                version: Some(contract_version),
                            },
                        ),
                    };
                    let container = FbsContractContainer::create(
                        &mut builder,
                        &ContractContainerArgs {
                            contract_type: FbsWasmContract::ContractV1,
                            contract: Some(contract.as_union_value()),
                        },
                    );
                    let get = FbsGetResponse::create(
                        &mut builder,
                        &GetResponseArgs {
                            contract: Some(container),
                            state: Some(state),
                        },
                    );
                    let get_response = FbsContractResponse::create(
                        &mut builder,
                        &ContractResponseArgs {
                            contract_response_type: ContractResponseType::PutResponse,
                            contract_response: Some(get.as_union_value()),
                        },
                    );
                    let host_response = FbsHostResponse::create(
                        &mut builder,
                        &HostResponseArgs {
                            response_type: HostResponseType::ContractResponse,
                            response: Some(get_response.as_union_value()),
                        },
                    );
                    builder.finish(host_response, None);
                    builder.finished_data().to_vec()
                }
                ContractResponse::UpdateNotification { key, update } => {
                    let data = builder.create_vector(&key.bytes());
                    let instance = FbsContractInstanceId::create(
                        &mut builder,
                        &ContractInstanceIdArgs { data: Some(data) },
                    );
                    let key = FbsKey::create(
                        &mut builder,
                        &KeyArgs {
                            instance: Some(instance),
                            code: None,
                        },
                    );

                    let update_data = match update {
                        State(state) => {
                            let state_data = builder.create_vector(&state.into_bytes());
                            let state = FbsState::create(
                                &mut builder,
                                &StateArgs {
                                    data: Some(state_data),
                                },
                            );
                            let state = FbsStateUpdate::create(
                                &mut builder,
                                &StateUpdateArgs { state: Some(state) },
                            );

                            // Create the update data
                            FbsUpdateData::create(
                                &mut builder,
                                &UpdateDataArgs {
                                    update_data_type: UpdateDataType::StateUpdate,
                                    update_data: Some(state.as_union_value()),
                                },
                            )
                        }
                        Delta(delta) => {
                            let delta_data = builder.create_vector(&delta.into_bytes());
                            let delta = FbsStateDelta::create(
                                &mut builder,
                                &StateDeltaArgs {
                                    data: Some(delta_data),
                                },
                            );
                            let update = FbsDeltaUpdate::create(
                                &mut builder,
                                &DeltaUpdateArgs { delta: Some(delta) },
                            );

                            // Create the update data
                            FbsUpdateData::create(
                                &mut builder,
                                &UpdateDataArgs {
                                    update_data_type: UpdateDataType::DeltaUpdate,
                                    update_data: Some(update.as_union_value()),
                                },
                            )
                        }
                        StateAndDelta { state, delta } => {
                            let state_data = builder.create_vector(&state.into_bytes());
                            let state = FbsState::create(
                                &mut builder,
                                &StateArgs {
                                    data: Some(state_data),
                                },
                            );
                            let delta_data = builder.create_vector(&delta.into_bytes());
                            let delta = FbsStateDelta::create(
                                &mut builder,
                                &StateDeltaArgs {
                                    data: Some(delta_data),
                                },
                            );

                            let update = FbsStateDeltaUpdate::create(
                                &mut builder,
                                &StateDeltaUpdateArgs {
                                    state: Some(state),
                                    delta: Some(delta),
                                },
                            );

                            // Create the update data
                            FbsUpdateData::create(
                                &mut builder,
                                &UpdateDataArgs {
                                    update_data_type: UpdateDataType::StateDeltaUpdate,
                                    update_data: Some(update.as_union_value()),
                                },
                            )
                        }
                        RelatedState { related_to, state } => {
                            let data = builder.create_vector(&related_to.as_bytes());
                            let instance = FbsContractInstanceId::create(
                                &mut builder,
                                &ContractInstanceIdArgs { data: Some(data) },
                            );

                            let state_data = builder.create_vector(&state.into_bytes());
                            let state = FbsState::create(
                                &mut builder,
                                &StateArgs {
                                    data: Some(state_data),
                                },
                            );

                            let update = FbsStateWithContractInstanceUpdate::create(
                                &mut builder,
                                &StateWithContractInstanceUpdateArgs {
                                    related_to: Some(instance),
                                    state: Some(state),
                                },
                            );

                            // Create the update data
                            FbsUpdateData::create(
                                &mut builder,
                                &UpdateDataArgs {
                                    update_data_type: UpdateDataType::StateDeltaUpdate,
                                    update_data: Some(update.as_union_value()),
                                },
                            )
                        }
                        RelatedDelta { related_to, delta } => {
                            let data = builder.create_vector(&related_to.as_bytes());
                            let instance = FbsContractInstanceId::create(
                                &mut builder,
                                &ContractInstanceIdArgs { data: Some(data) },
                            );

                            let delta_data = builder.create_vector(&delta.into_bytes());
                            let delta = FbsStateDelta::create(
                                &mut builder,
                                &StateDeltaArgs {
                                    data: Some(delta_data),
                                },
                            );

                            let update = FbsDeltaWithContractInstanceUpdate::create(
                                &mut builder,
                                &DeltaWithContractInstanceUpdateArgs {
                                    related_to: Some(instance),
                                    delta: Some(delta),
                                },
                            );

                            // Create the update data
                            FbsUpdateData::create(
                                &mut builder,
                                &UpdateDataArgs {
                                    update_data_type: UpdateDataType::StateDeltaUpdate,
                                    update_data: Some(update.as_union_value()),
                                },
                            )
                        }
                        RelatedStateAndDelta {
                            related_to,
                            state,
                            delta,
                        } => {
                            let data = builder.create_vector(&related_to.as_bytes());
                            let instance = FbsContractInstanceId::create(
                                &mut builder,
                                &ContractInstanceIdArgs { data: Some(data) },
                            );

                            let state_data = builder.create_vector(&state.into_bytes());
                            let state = FbsState::create(
                                &mut builder,
                                &StateArgs {
                                    data: Some(state_data),
                                },
                            );

                            let delta_data = builder.create_vector(&delta.into_bytes());
                            let delta = FbsStateDelta::create(
                                &mut builder,
                                &StateDeltaArgs {
                                    data: Some(delta_data),
                                },
                            );

                            let update = FbsStateDeltaWithContractInstanceUpdate::create(
                                &mut builder,
                                &StateDeltaWithContractInstanceUpdateArgs {
                                    related_to: Some(instance),
                                    state: Some(state),
                                    delta: Some(delta),
                                },
                            );

                            // Create the update data
                            FbsUpdateData::create(
                                &mut builder,
                                &UpdateDataArgs {
                                    update_data_type: UpdateDataType::StateDeltaUpdate,
                                    update_data: Some(update.as_union_value()),
                                },
                            )
                        }
                    };

                    let update_notification = FbsUpdateNotification::create(
                        &mut builder,
                        &UpdateNotificationArgs {
                            key: Some(key),
                            update: Some(update_data),
                        },
                    );
                    let put_response = FbsContractResponse::create(
                        &mut builder,
                        &ContractResponseArgs {
                            contract_response_type: ContractResponseType::PutResponse,
                            contract_response: Some(update_notification.as_union_value()),
                        },
                    );
                    let host_response = FbsHostResponse::create(
                        &mut builder,
                        &HostResponseArgs {
                            response_type: HostResponseType::ContractResponse,
                            response: Some(put_response.as_union_value()),
                        },
                    );
                    builder.finish(host_response, None);
                    builder.finished_data().to_vec()
                }
            },
            HostResponse::ComponentResponse { .. } => todo!(),
            HostResponse::Ok => todo!(),
            HostResponse::GenerateRandData(_) => todo!(),
        }
    }
}

impl std::fmt::Display for HostResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostResponse::ContractResponse(res) => match res {
                ContractResponse::PutResponse { key } => {
                    f.write_fmt(format_args!("put response: {key}"))
                }
                ContractResponse::UpdateResponse { key, .. } => {
                    f.write_fmt(format_args!("update response ({key})"))
                }
                ContractResponse::GetResponse { state, .. } => {
                    f.write_fmt(format_args!("get response: {state}"))
                }
                ContractResponse::UpdateNotification { key, .. } => {
                    f.write_fmt(format_args!("update notification (key: {key})"))
                }
            },
            HostResponse::ComponentResponse { .. } => write!(f, "component responses"),
            HostResponse::Ok => write!(f, "ok response"),
            HostResponse::GenerateRandData(_) => write!(f, "random bytes"),
        }
    }
}

// todo: add a `AsBytes` trait for state representations
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum ContractResponse<T = WrappedState> {
    GetResponse {
        contract: Option<ContractContainer>,
        #[serde(bound(deserialize = "T: DeserializeOwned"))]
        state: T,
    },
    PutResponse {
        key: ContractKey,
    },
    /// Message sent when there is an update to a subscribed contract.
    UpdateNotification {
        key: ContractKey,
        #[serde(deserialize_with = "ContractResponse::<T>::deser_update_data")]
        update: UpdateData<'static>,
    },
    /// Successful update
    UpdateResponse {
        key: ContractKey,
        #[serde(deserialize_with = "ContractResponse::<T>::deser_state")]
        summary: StateSummary<'static>,
    },
}

impl<T> ContractResponse<T> {
    fn deser_update_data<'de, D>(deser: D) -> Result<UpdateData<'static>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <UpdateData as Deserialize>::deserialize(deser)?;
        Ok(value.into_owned())
    }

    fn deser_state<'de, D>(deser: D) -> Result<StateSummary<'static>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <StateSummary as Deserialize>::deserialize(deser)?;
        Ok(value.into_owned())
    }
}

impl<T> From<ContractResponse<T>> for HostResponse<T> {
    fn from(value: ContractResponse<T>) -> HostResponse<T> {
        HostResponse::ContractResponse(value)
    }
}
