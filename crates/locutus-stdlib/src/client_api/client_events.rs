use std::fmt::Display;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::host_response_generated::schemas::host::{ContractContainerArgs, ContractContainerBuilder, ContractInstanceIdBuilder, ContractResponseArgs, ContractResponseBuilder, ContractResponseType, ContractV1Args, ContractV1Builder, DeltaUpdateArgs, DeltaUpdateBuilder, DeltaWithContractInstanceUpdateArgs, DeltaWithContractInstanceUpdateBuilder, GetResponseArgs, GetResponseBuilder, HostResponseArgs, HostResponseBuilder, HostResponseType, KeyArgs, KeyBuilder, PutResponseArgs, PutResponseBuilder, PutResponseT, StateArgs, StateBuilder, StateDeltaArgs, StateDeltaBuilder, StateDeltaUpdateArgs, StateDeltaUpdateBuilder, StateDeltaWithContractInstanceUpdateArgs, StateDeltaWithContractInstanceUpdateBuilder, StateSummaryArgs, StateSummaryBuilder, StateUpdateArgs, StateUpdateBuilder, StateWithContractInstanceUpdateArgs, StateWithContractInstanceUpdateBuilder, UpdateDataArgs, UpdateDataBuilder, UpdateDataType, UpdateNotificationArgs, UpdateNotificationBuilder, UpdateResponseArgs, UpdateResponseBuilder};
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
                    let mut instance_builder = ContractInstanceIdBuilder::new(&mut builder);
                    instance_builder.add_data(data);
                    let instance_offset = instance_builder.finish();

                    let mut key_builder = KeyBuilder::new(&mut builder);
                    key_builder.add_instance(instance_offset);
                    let key_offset = key_builder.finish();

                    let mut put_builder = PutResponseBuilder::new(&mut builder);
                    put_builder.add_key(key_offset);
                    let put_offset = put_builder.finish();

                    let mut response_builder = HostResponseBuilder::new(&mut builder);
                    response_builder.add_response(put_offset.as_union_value());
                    response_builder.add_response_type(HostResponseType::ContractResponse);
                    let response_offset = response_builder.finish();

                    builder.finish(response_offset, None);
                    builder.finished_data().to_vec()
                }
                ContractResponse::UpdateResponse { key, summary } => {
                    let data = builder.create_vector(&key.bytes());
                    let mut instance_builder = ContractInstanceIdBuilder::new(&mut builder);
                    instance_builder.add_data(data);
                    let instance_offset = instance_builder.finish();

                    let mut key_builder = KeyBuilder::new(&mut builder);
                    key_builder.add_instance(instance_offset);
                    let key_offset = key_builder.finish();

                    let data = builder.create_vector(&summary.into_bytes());
                    let mut summary_builder = StateSummaryBuilder::new(&mut builder);
                    summary_builder.add_data(data);
                    let summary_offset = summary_builder.finish();

                    let mut update_response_builder = UpdateResponseBuilder::new(&mut builder);
                    update_response_builder.add_key(key_offset);
                    update_response_builder.add_summary(summary_offset);
                    let update_response_offset = update_response_builder.finish();

                    let mut contract_response_builder = ContractResponseBuilder::new(&mut builder);
                    contract_response_builder.add_contract_response(update_response_offset.as_union_value());
                    contract_response_builder.add_contract_response_type(ContractResponseType::UpdateResponse);
                    let contract_response_offset = contract_response_builder.finish();

                    let mut host_response_builder = HostResponseBuilder::new(&mut builder);
                    host_response_builder.add_response(contract_response_offset.as_union_value());
                    host_response_builder.add_response_type(HostResponseType::ContractResponse);
                    let host_response_offset = host_response_builder.finish();

                    builder.finish(host_response_offset, None);
                    builder.finished_data().to_vec()
                }
                ContractResponse::GetResponse { state, contract } => {let state_data = builder.create_vector(&state.to_vec());
                    let mut state_builder = StateBuilder::new(&mut builder);
                    state_builder.add_data(state_data);
                    let state_offset = state_builder.finish();

                    let contract = contract.unwrap();
                    let data = builder.create_vector(&contract.key().bytes());
                    let mut instance_builder = ContractInstanceIdBuilder::new(&mut builder);
                    instance_builder.add_data(data);
                    let instance_offset = instance_builder.finish();

                    let mut key_builder = KeyBuilder::new(&mut builder);
                    key_builder.add_instance(instance_offset);
                    let key_offset = key_builder.finish();

                    let contract_data = builder.create_vector(&contract.data());
                    let contract_params = builder.create_vector(&contract.params().into_bytes());
                    let contract_version = builder.create_string(&contract.version());

                    let contract_builder = match contract {
                        Wasm(WasmAPIVersion::V1(..)) => {
                            let mut contract_v1_builder = ContractV1Builder::new(&mut builder);
                            contract_v1_builder.add_key(key_offset);
                            contract_v1_builder.add_data(contract_data);
                            contract_v1_builder.add_parameters(contract_params);
                            contract_v1_builder.add_version(contract_version);
                            contract_v1_builder
                        }
                    };
                    let contract_offset = contract_builder.finish();

                    let mut container_builder = ContractContainerBuilder::new(&mut builder);
                    container_builder.add_contract_type(FbsWasmContract::ContractV1);
                    container_builder.add_contract(contract_offset.as_union_value());
                    let container_offset = container_builder.finish();

                    let mut get_builder = GetResponseBuilder::new(&mut builder);
                    get_builder.add_contract(container_offset);
                    get_builder.add_state(state_offset);
                    let get_offset = get_builder.finish();

                    let mut contract_response_builder = ContractResponseBuilder::new(&mut builder);
                    contract_response_builder.add_contract_response_type(ContractResponseType::PutResponse);
                    contract_response_builder.add_contract_response(get_offset.as_union_value());
                    let contract_response_offset = contract_response_builder.finish();

                    let mut response_builder = HostResponseBuilder::new(&mut builder);
                    response_builder.add_response(contract_response_offset.as_union_value());
                    response_builder.add_response_type(HostResponseType::ContractResponse);
                    let response_offset = response_builder.finish();

                    builder.finish(response_offset, None);
                    builder.finished_data().to_vec()
                }
                ContractResponse::UpdateNotification { key, update } => {
                    let data = builder.create_vector(&key.bytes());
                    let mut instance_builder = ContractInstanceIdBuilder::new(&mut builder);
                    instance_builder.add_data(data);
                    let instance_offset = instance_builder.finish();

                    let mut key_builder = KeyBuilder::new(&mut builder);
                    key_builder.add_instance(instance_offset);
                    let key_offset = key_builder.finish();

                    let update_data = match update {
                        State(state) => {
                            let state_data = builder.create_vector(&state.into_bytes());
                            let mut state_builder = StateBuilder::new(&mut builder);
                            state_builder.add_data(state_data);
                            let state_offset = state_builder.finish();

                            let mut state_update_builder = StateUpdateBuilder::new(&mut builder);
                            state_update_builder.add_state(state_offset);
                            let state_update_offset = state_update_builder.finish();

                            let mut update_data_builder = UpdateDataBuilder::new(&mut builder);
                            update_data_builder.add_update_data_type(UpdateDataType::StateUpdate);
                            update_data_builder.add_update_data(state_update_offset.as_union_value());
                            let update_data_offset = update_data_builder.finish();
                            update_data_offset
                        }
                        Delta(delta) => {
                            let delta_data = builder.create_vector(&delta.into_bytes());
                            let mut delta_builder = StateDeltaBuilder::new(&mut builder);
                            delta_builder.add_data(delta_data);
                            let delta_offset = delta_builder.finish();

                            let mut update_builder = DeltaUpdateBuilder::new(&mut builder);
                            update_builder.add_delta(delta_offset);
                            let update_offset = update_builder.finish();

                            let mut update_data_builder = UpdateDataBuilder::new(&mut builder);
                            update_data_builder.add_update_data_type(UpdateDataType::DeltaUpdate);
                            update_data_builder.add_update_data(update_offset.as_union_value());
                            let update_data_offset = update_data_builder.finish();
                            update_data_offset
                        }
                        StateAndDelta { state, delta } => {
                            let state_data = builder.create_vector(&state.into_bytes());
                            let mut state_builder = StateBuilder::new(&mut builder);
                            state_builder.add_data(state_data);
                            let state_offset = state_builder.finish();

                            let delta_data = builder.create_vector(&delta.into_bytes());
                            let mut delta_builder = StateDeltaBuilder::new(&mut builder);
                            delta_builder.add_data(delta_data);
                            let delta_offset = delta_builder.finish();

                            let mut update_builder = StateDeltaUpdateBuilder::new(&mut builder);
                            update_builder.add_state(state_offset);
                            update_builder.add_delta(delta_offset);
                            let update_offset = update_builder.finish();

                            let mut update_data_builder = UpdateDataBuilder::new(&mut builder);
                            update_data_builder.add_update_data_type(UpdateDataType::StateDeltaUpdate);
                            update_data_builder.add_update_data(update_offset.as_union_value());
                            let update_data_offset = update_data_builder.finish();
                            update_data_offset
                        }
                        RelatedState { related_to, state } => {
                            let state_data = builder.create_vector(&state.into_bytes());
                            let mut state_builder = StateBuilder::new(&mut builder);
                            state_builder.add_data(state_data);
                            let state_offset = state_builder.finish();

                            let instance_data = builder.create_vector(&related_to.as_bytes());
                            let mut instance_builder = ContractInstanceIdBuilder::new(&mut builder);
                            instance_builder.add_data(instance_data);
                            let instance_offset = instance_builder.finish();

                            let mut update_builder = StateWithContractInstanceUpdateBuilder::new(&mut builder);
                            update_builder.add_related_to(instance_offset);
                            update_builder.add_state(state_offset);
                            let update_offset = update_builder.finish();

                            let mut update_data_builder = UpdateDataBuilder::new(&mut builder);
                            update_data_builder.add_update_data_type(UpdateDataType::StateDeltaUpdate);
                            update_data_builder.add_update_data(update_offset.as_union_value());
                            let update_data_offset = update_data_builder.finish();
                            update_data_offset
                        }
                        RelatedDelta { related_to, delta } => {
                            let instance_data = builder.create_vector(&related_to.as_bytes());
                            let mut instance_builder = ContractInstanceIdBuilder::new(&mut builder);
                            instance_builder.add_data(instance_data);
                            let instance_offset = instance_builder.finish();

                            let delta_data = builder.create_vector(&delta.into_bytes());
                            let mut delta_builder = StateDeltaBuilder::new(&mut builder);
                            delta_builder.add_data(delta_data);
                            let delta_offset = delta_builder.finish();

                            let mut update_builder = DeltaWithContractInstanceUpdateBuilder::new(&mut builder);
                            update_builder.add_related_to(instance_offset);
                            update_builder.add_delta(delta_offset);
                            let update_offset = update_builder.finish();

                            let mut update_data_builder = UpdateDataBuilder::new(&mut builder);
                            update_data_builder.add_update_data_type(UpdateDataType::StateDeltaUpdate);
                            update_data_builder.add_update_data(update_offset.as_union_value());
                            let update_data_offset = update_data_builder.finish();
                            update_data_offset
                        }
                        RelatedStateAndDelta {
                            related_to,
                            state,
                            delta,
                        } => {
                            let instance_data = builder.create_vector(&related_to.as_bytes());
                            let mut instance_builder = ContractInstanceIdBuilder::new(&mut builder);
                            instance_builder.add_data(instance_data);
                            let instance_offset = instance_builder.finish();

                            let state_data = builder.create_vector(&state.into_bytes());
                            let mut state_builder = StateBuilder::new(&mut builder);
                            state_builder.add_data(state_data);
                            let state_offset = state_builder.finish();

                            let delta_data = builder.create_vector(&delta.into_bytes());
                            let mut delta_builder = StateDeltaBuilder::new(&mut builder);
                            delta_builder.add_data(delta_data);
                            let delta_offset = delta_builder.finish();

                            let mut update_builder = StateDeltaWithContractInstanceUpdateBuilder::new(&mut builder);
                            update_builder.add_related_to(instance_offset);
                            update_builder.add_state(state_offset);
                            update_builder.add_delta(delta_offset);
                            let update_offset = update_builder.finish();

                            let mut update_data_builder = UpdateDataBuilder::new(&mut builder);
                            update_data_builder.add_update_data_type(UpdateDataType::StateDeltaUpdate);
                            update_data_builder.add_update_data(update_offset.as_union_value());
                            let update_data_offset = update_data_builder.finish();
                            update_data_offset

                        }
                    };

                    let mut update_notification = UpdateNotificationBuilder::new(&mut builder);
                    update_notification.add_key(key_offset);
                    update_notification.add_update(update_data);
                    let update_notification_offset = update_notification.finish();

                    let mut put_response_builder = ContractResponseBuilder::new(&mut builder);
                    put_response_builder.add_contract_response_type(ContractResponseType::PutResponse);
                    put_response_builder.add_contract_response(update_notification_offset.as_union_value());
                    let put_response_offset = put_response_builder.finish();

                    let mut host_response_builder = HostResponseBuilder::new(&mut builder);
                    host_response_builder.add_response_type(HostResponseType::ContractResponse);
                    host_response_builder.add_response(put_response_offset.as_union_value());
                    let host_response_offset = host_response_builder.finish();

                    builder.finish(host_response_offset, None);
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
