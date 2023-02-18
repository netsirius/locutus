import {decode} from "@msgpack/msgpack";
import base58 from "bs58";

import * as flatbuffers from "flatbuffers";
// @ts-ignore
import {
  ClientRequest,
  ClientRequestType,
  ContractContainer as FbsContractContainer,
  ContractInstanceId as FbsContractInstanceId,
  ContractRequest,
  ContractRequestType,
  ContractV1 as FbsContractV1,
  DeltaUpdate as FbsDeltaUpdate,
  DeltaWithContractInstanceUpdate as FbsDeltaWithContractInstanceUpdate,
  Disconnect,
  Get,
  Key as FbsKey,
  Put,
  RelatedContract,
  RelatedContracts as FbsRelatedContracts,
  State as FbsState,
  StateDelta as FbsStateDelta,
  StateDeltaUpdate as FbsStateDeltaUpdate,
  StateDeltaWithContractInstanceUpdate as FbsStateDeltaWithContractInstanceUpdate,
  StateUpdate as FbsStateUpdate,
  StateWithContractInstanceUpdate as FbsStateWithContractInstanceUpdate,
  Subscribe,
  Update,
  UpdateData as FbsUpdateData,
  UpdateDataType,
  WasmContract as FbsWasmContract,
} from "./client_request_generated";

import {
  HostResponse as FbsHostResponse,
  ContractResponse as FbsContractResponse,
  ComponentResponse as FbsComponentResponse,
  GenerateRandData as FbsGenerateRandData,
  GetResponse as FbsGetResponse,
  HostResponseType,
  PutResponse as FbsPutReponse,
  UpdateNotification as FbsUpdateNotification,
  UpdateResponse as FbsUpdateResponse, ContractResponseType,
  ContractContainer as FbsHostContractContainer,
} from "./host_response_generated";

const MAX_U8: number = 255;
const MIN_U8: number = 0;

// base interface types:

/**
 * The id of a live instance of a contract. This is effectively the tuple
 * of the hash of the hash of the contract code and a set of parameters used to run
 * the contract.
 * @public
 */
export type ContractInstanceId = Uint8Array;

/**
 * The key representing the tuple of a contract code and a set of parameters.
 * See {@link ContractInstanceId} for more information.
 * @public
 */
export class Key {
  private instance: ContractInstanceId;
  private code: Uint8Array | null;

  /**
   * @constructor
   * @param {ContractInstanceId} instance
   * @param {Uint8Array} [code]
   */
  constructor(instance: ContractInstanceId, code?: Uint8Array) {
    if (
      instance.length != 32 ||
      (typeof code != "undefined" && code.length != 32)
    ) {
      throw TypeError(
        "invalid array lenth (expected 32 bytes): " + instance.length
      );
    }
    this.instance = instance;
    if (typeof code == "undefined") {
      this.code = null;
    } else {
      this.code = code;
    }
  }

  /**
   * Generates key from base58 key spec representation
   * @example
   * Here's a simple example:
   * ```
   * const MODEL_CONTRACT = "DCBi7HNZC3QUZRiZLFZDiEduv5KHgZfgBk8WwTiheGq1";
   * const KEY = Key.fromSpec(MODEL_CONTRACT);
   * ```
   * @param spec - Base58 string representation of the key
   * @returns The key representation from given spec
   * @constructor
   */
  static fromInstanceId(spec: string): Key {
    let encoded = base58.decode(spec);
    return new Key(encoded);
  }

  /**
   * @returns {ContractInstanceId} Hash of the full key specification (contract code + parameter).
   */
  bytes(): ContractInstanceId {
    return this.instance;
  }

  /**
   * @returns {Uint8Array | null} Hash of the contract code part of the full specification, if is available.
   */
  codePart(): Uint8Array | null {
    return this.code;
  }

  /**
   * Generates the full key specification (contract code + parameter) encoded as base58 string.
   *
   * @returns {string} The encoded string representation.
   */
  encode(): string {
    return base58.encode(this.instance);
  }
}

/**
 * A contract and its key. It includes the contract data/code and the parameters run along with it.
 * @public
 */
export type ContractV1 = {
  key: Key;
  data: Uint8Array;
  parameters: Uint8Array;
  version: String;
};

/**
 * Representation of a contract state
 * @public
 */
export type State = Uint8Array;

/**
 * Representation of a contract state changes summary
 * @public
 */
export type StateSummary = Uint8Array;

/**
 * State delta representation
 * @public
 */
export type StateDelta = Uint8Array;

/** Update data from a notification for a contract which the client subscribed to.
 * It can be either the main contract or any related contracts to that main contract.
 * @public
 */
export type UpdateData =
  | { state: State }
  | { delta: StateDelta }
  | { state: State; delta: StateDelta }
  | { relatedTo: ContractInstanceId; state: State }
  | { relatedTo: ContractInstanceId; delta: StateDelta }
  | { relatedTo: ContractInstanceId; state: State; delta: StateDelta };

/**
 * A map of contract id's to their respective states in case this have
 * been successfully retrieved from the network.
 */
export type RelatedContracts = Map<ContractInstanceId, State | null>;

// ops:

/**
 * Representation of the client put request operation
 * @public
 */
export type PutRequest = {
  container: ContractContainer;
  state: State;
  relatedContracts: RelatedContracts;
};

/**
 * Representation of the client update request operation
 * @public
 */
export type UpdateRequest = {
  key: Key;
  data: UpdateData;
};

/**
 * Representation of the client get request operation
 * @public
 */
export type GetRequest = {
  key: Key;
  fetchContract: boolean;
};

/**
 * Representation of the client subscribe request operation
 * @public
 */
export type SubscribeRequest = {
  key: Key;
};

/**
 * Representation of the client disconnect request operation
 * @public
 */
export type DisconnectRequest = {
  cause?: string;
};

// API

/**
 * Interface to handle responses from the host
 *
 * @example
 * Here's a simple implementation example:
 * ```
 * const handler = {
 *  onPut: (_response: PutResponse) => {},
 *  onGet: (_response: GetResponse) => {},
 *  onUpdate: (_up: UpdateResponse) => {},
 *  onUpdateNotification: (_notif: UpdateNotification) => {},
 *  onErr: (err: HostError) => {},
 *  onOpen: () => {},
 * };
 * ```
 *
 * @public
 */
export interface ResponseHandler {
  /**
   * `Put` response handler
   */
  onPut: (response: PutResponse) => void;
  /**
   * `Get` response handler
   */
  onGet: (response: GetResponse) => void;
  /**
   * `Update` response handler
   */
  onUpdate: (response: UpdateResponse) => void;
  /**
   * `Update` notification handler
   */
  onUpdateNotification: (response: UpdateNotification) => void;
  /**
   * `Error` handler
   */
  onErr: (response: HostError) => void;
  /**
   * Callback executed after successfully establishing connection with websocket
   */
  onOpen: () => void;
}

/**
 * The `LocutusWsApi` provides the API to manage the connection to the host, handle responses, and send requests.
 * @example
 * Here's a simple example:
 * ```
 * const API_URL = new URL(`ws://${location.host}/contract/command/`);
 * const locutusApi = new LocutusWsApi(API_URL, handler);
 * ```
 */
export class LocutusWsApi {
  /**
   * Websocket object for creating and managing a WebSocket connection to a server,
   * as well as for sending and receiving data on the connection.
   * @private
   */
  private ws: WebSocket;
  /**
   * @private
   */
  private reponseHandler: ResponseHandler;

  /**
   * @constructor
   * @param url - The websocket URL to which to connect
   * @param handler - The ResponseHandler implementation
   */
  constructor(url: URL, handler: ResponseHandler) {
    this.ws = new WebSocket(url);
    this.ws.binaryType = "arraybuffer";
    this.reponseHandler = handler;
    this.ws.onmessage = (ev) => {
      this.handleResponse(ev);
    };
    this.ws.addEventListener("open", (_) => {
      handler.onOpen();
    });
  }

  /**
   * @private
   */
  private handleResponse(ev: MessageEvent<any>): void | Error {
    let response;
    try {
      let data = new Uint8Array(ev.data);
      response = new HostResponse(data);
    } catch (err) {
      console.log(`found error: ${err}`);
      return new Error(`${err}`);
    }
    if (response.isOk()) {
      switch (response.unwrapOk().kind) {
        case "put":
          this.reponseHandler.onPut(response.unwrapPut());
        case "get":
          this.reponseHandler.onGet(response.unwrapGet());
        case "update":
          this.reponseHandler.onUpdate(response.unwrapUpdate());
        case "updateNotification":
          this.reponseHandler.onUpdateNotification(
            response.unwrapUpdateNotification()
          );
      }
    } else {
      this.reponseHandler.onErr(response.unwrapErr());
    }
  }

  /**
   * Sends a put request to the host through websocket
   * @param put - The `PutRequest` object
   */
  async put(put: PutRequest): Promise<void> {
    let request = new FlatbuffersClientRequest(RequestType.PutRequest, put);
    let request_bytes = request.request_bytes;
    this.ws.send(request_bytes);
  }

  /**
   * Sends an update request to the host through websocket
   * @param update - The `UpdateRequest` object
   */
  async update(update: UpdateRequest): Promise<void> {
    let request = new FlatbuffersClientRequest(RequestType.UpdateRequest, update);
    let request_bytes = request.request_bytes;
    this.ws.send(request_bytes);
  }

  /**
   * Sends a get request to the host through websocket
   * @param get - The `GetRequest` object
   */
  async get(get: GetRequest): Promise<void> {
    let request = new FlatbuffersClientRequest(RequestType.GetRequest, get);
    let request_bytes = request.request_bytes;
    this.ws.send(request_bytes);
  }

  /**
   * Sends a subscribe request to the host through websocket
   * @param subscribe - The `SubscribeRequest` object
   */
  async subscribe(subscribe: SubscribeRequest): Promise<void> {
    let request = new FlatbuffersClientRequest(RequestType.SubscribeRequest, subscribe);
    let request_bytes = request.request_bytes;
    this.ws.send(request_bytes);
  }

  /**
   * Sends an disconnect notification to the host through websocket
   * @param disconnect - The `DisconnectRequest` object
   */
  async disconnect(disconnect: DisconnectRequest): Promise<void> {
    let request = new FlatbuffersClientRequest(RequestType.DisconnectRequest, disconnect);
    let request_bytes = request.request_bytes;
    this.ws.send(request_bytes);
    this.ws.close();
  }
}

// host replies:

/**
 * Host response types
 * @public
 */
export type Ok =
  | PutResponse
  | UpdateResponse
  | GetResponse
  | UpdateNotification;

/**
 * Host reponse error type
 * @public
 */
export type HostError = {
  cause: string;
};

/**
 * The response for a contract put operation
 * @public
 */
export interface PutResponse {
  readonly kind: "put";
  key: Key;
}

/**
 * The response for a contract update operation
 * @public
 */
export interface UpdateResponse {
  readonly kind: "update";
  key: Key;
  summary: StateSummary;
}

/**
 * The response for a contract get operation
 * @public
 */
export interface GetResponse {
  readonly kind: "get";
  contract?: ContractV1;
  state: State;
}

// TODO Implement response types as classes to build each type instance from flatbuffers
// class GetResponse {
//    public contract: ContractContainer | undefined;
//    public state: State | undefined;
//   constructor(_fbsResponse: FbsGetResponse) {
//     // const key = this.contract()?.contract(new FbsContractContainer()).
//     // const state_data = this._state()?.dataArray()?.buffer;
//   }
// }

/**
 * The response for a state update notification
 * @public
 */
export interface UpdateNotification {
  readonly kind: "updateNotification";
  key: Key;
  update: UpdateData;
}

/**
 * Check that the condition is met
 * @param condition - Condition to check
 * @param [msg] - Error message
 * @public
 */
function assert(condition: boolean, msg?: string) {
  if (!condition) throw new TypeError(msg);
}

// TODO Finish to implement this constructor to build HostResponse from flatbuffers
// export class HostResponse {
//   /**
//    * @private
//    */
//   private result: Ok | HostError;
//
//   /**
//    * Builds the response from the bytes received via the websocket interface.
//    * @param bytes - Response data
//    * @returns The corresponding response type result
//    * @constructor
//    */
//   constructor(bytes: Uint8Array) {
//     const buf = new flatbuffers.ByteBuffer(bytes);
//     const hostResponse = FbsHostResponse.getRootAsHostResponse(buf);
//     switch (hostResponse.responseType()) {
//       case HostResponseType.ContractResponse:
//         const contractResponse = hostResponse.response(new FbsContractResponse());
//         const contractResponseType = contractResponse.contractResponseType();
//
//         switch (contractResponseType) {
//           case ContractResponseType.GetResponse:
//             let getResponse = contractResponse.contractResponse(new GetResponse());
//             // const { key, data, parameters, version } = getResponse.
//             // const wasmContract = contractContainer.contract();
//             // const contractV1 = wasmContract.ContractV1();
//             this.result = getResponse;
//             break;
//           default:
//             break;
//         }
//         break;
//       case HostResponseType.ComponentResponse:
//         this.result = {cause: ""}
//         break;
//       case HostResponseType.GenerateRandData:
//         this.result = {cause: ""}
//         break;
//       default:
//         throw new Error(`Unexpected HostResponse type`);
//     }
//   }
// }

/**
 * A typed response from the host HTTP gateway to requests made via the API.
 *
 * @public
 */
export class HostResponse {
  /**
   * @private
   */
  private result: Ok | HostError;

  /**
   * Builds the response from the bytes received via the websocket interface.
   * @param bytes - Response data
   * @returns The corresponding response type result
   * @constructor
   */
  constructor(bytes: Uint8Array) {
    let decoded = decode(bytes) as object;
    if ("Ok" in decoded) {
      let ok = decoded as { Ok: any };
      if ("ContractResponse" in ok.Ok) {
        let response = ok.Ok as { ContractResponse: any };
        if ("PutResponse" in response.ContractResponse) {
          response.ContractResponse as { PutResponse: any };
          assert(Array.isArray(response.ContractResponse.PutResponse));
          let key = HostResponse.assertKey(
            response.ContractResponse.PutResponse[0][0]
          );
          this.result = { kind: "put", key };
          return;
        } else if ("UpdateResponse" in response.ContractResponse) {
          response.ContractResponse as { UpdateResponse: any };
          assert(Array.isArray(response.ContractResponse.UpdateResponse));
          assert(response.ContractResponse.UpdateResponse.length == 2);
          let key = HostResponse.assertKey(
            response.ContractResponse.UpdateResponse[0][0]
          );
          let summary = HostResponse.assertBytes(
            response.ContractResponse.UpdateResponse[1]
          );
          this.result = { kind: "update", key, summary };
          return;
        } else if ("GetResponse" in response.ContractResponse) {
          response.ContractResponse as { GetResponse: any };
          assert(Array.isArray(response.ContractResponse.GetResponse));
          assert(response.ContractResponse.GetResponse.length == 2);
          let contract;
          if (response.ContractResponse.GetResponse[0] !== null) {
            contract = {
              data: new Uint8Array(
                response.ContractResponse.GetResponse[0][0][1]
              ),
              parameters: new Uint8Array(
                response.ContractResponse.GetResponse[0][1]
              ),
              key: new Key(response.ContractResponse.GetResponse[0][2][0]),
            };
          } else {
            contract = null;
          }
          let get = {
            kind: "get",
            contract,
            state: response.ContractResponse.GetResponse[1],
          };
          this.result = get as GetResponse;
          return;
        } else if ("UpdateNotification" in response.ContractResponse) {
          response.ContractResponse as { UpdateNotification: any };
          assert(Array.isArray(response.ContractResponse.UpdateNotification));
          assert(response.ContractResponse.UpdateNotification.length == 2);
          let key = HostResponse.assertKey(
            response.ContractResponse.UpdateNotification[0][0]
          );
          let update = HostResponse.getUpdateData(
            response.ContractResponse.UpdateNotification[1]
          );
          this.result = {
            kind: "updateNotification",
            key,
            update,
          } as UpdateNotification;
          return;
        }
      }
    } else if ("Err" in decoded) {
      let err = decoded as { Err: Array<any> };
      if ("RequestError" in err.Err[0]) {
        function formatErr(kind: string, err: Array<any>): HostError {
          let contractKey = new Key(err[0][0]).encode();
          let cause =
            `${kind} error for contract ${contractKey}, cause: ` + err[1];
          return { cause };
        }

        if (typeof err.Err[0].RequestError === "string") {
          this.result = { cause: err.Err[0].RequestError };
          return;
        }
        if ("Put" in err.Err[0].RequestError) {
          let putErr = err.Err[0].RequestError.Put as Array<any>;
          this.result = formatErr("Put", putErr);
          return;
        } else if ("Update" in err.Err[0].RequestError) {
          let updateErr = err.Err[0].RequestError.Update as Array<any>;
          this.result = formatErr("Update", updateErr);
          return;
        } else if ("Get" in err.Err[0].RequestError) {
          let getErr = err.Err[0].RequestError.Get as Array<any>;
          this.result = formatErr("Get", getErr);
          return;
        } else if ("Disconnect" in err.Err[0].RequestError) {
          this.result = { cause: "client disconnected" };
          return;
        }
      }
    }
    throw new TypeError("bytes are not a valid HostResponse");
  }

  /**
   * Check if the response is ok or an error.
   * @returns True if contains the expected key otherwise false
   * @public
   */
  isOk(): boolean {
    if ("kind" in this.result) return true;
    else return false;
  }

  /**
   * Try to get the response content.
   * @returns The specific response content
   * @public
   */
  unwrapOk(): Ok {
    if ("kind" in this.result) {
      return this.result;
    } else throw new TypeError();
  }

  /**
   * Check if the response is an error.
   * @returns True if is an error otherwise false
   * @public
   */
  isErr(): boolean {
    if (this.result instanceof Error) return true;
    else return false;
  }

  /**
   * Get the specific error object from the response content
   * @returns The specific error
   * @public
   */
  unwrapErr(): HostError {
    if (this.result instanceof Error) return this.result as HostError;
    else throw new TypeError();
  }

  /**
   * Check if is a put response.
   * @returns True if is a put response otherwise false
   * @public
   */
  isPut(): boolean {
    return this.isOfType("put");
  }

  /**
   * Try to get the response content as a `PutResponse` object
   * @returns The PutResponse object
   * @public
   */
  unwrapPut(): PutResponse {
    if (this.isOfType("put")) return this.result as PutResponse;
    else throw new TypeError();
  }

  /**
   * Check if is an update response.
   * @returns True if is an update response otherwise false
   * @public
   */
  isUpdate(): boolean {
    return this.isOfType("update");
  }

  /**
   * Try to get the response content as an `UpdateResponse` object
   * @returns The UpdateResponse object
   * @public
   */
  unwrapUpdate(): UpdateResponse {
    if (this.isOfType("update")) return this.result as UpdateResponse;
    else throw new TypeError();
  }

  /**
   * Check if is a get response.
   * @returns True if is a get response otherwise false
   * @public
   */
  isGet(): boolean {
    return this.isOfType("get");
  }

  /**
   * Try to get the response content as a GetResponse object
   * @returns The GetResponse object
   * @public
   */
  unwrapGet(): GetResponse {
    if (this.isOfType("get")) return this.result as GetResponse;
    else throw new TypeError();
  }

  /**
   * Check if is a update notification response.
   * @returns True if is a update notification response otherwise false
   * @public
   */
  isUpdateNotification(): boolean {
    return this.isOfType("updateNotification");
  }

  /**
   * Try to get the response content as a UpdateNotification object
   * @returns The UpdateNotification object
   * @public
   */
  unwrapUpdateNotification(): UpdateNotification {
    if (this.isOfType("updateNotification"))
      return this.result as UpdateNotification;
    else throw new TypeError();
  }

  /**
   * @private
   */
  private isOfType(ty: string): boolean {
    return "kind" in this.result && this.result.kind === ty;
  }

  /**
   * @private
   */
  private static assertKey(key: any): Key {
    let bytes = HostResponse.assertBytes(key);
    assert(bytes.length === 32, "expected exactly 32 bytes");
    return new Key(bytes as Uint8Array);
  }

  /**
   * @private
   */
  private static assertBytes(state: any): Uint8Array {
    assert(Array.isArray(state) || ArrayBuffer.isView(state));
    assert(
      state.every((value: any) => {
        if (typeof value === "number" && value >= MIN_U8 && value <= MAX_U8)
          return true;
        else return false;
      }),
      "expected an array of bytes"
    );
    return state as Uint8Array;
  }

  private static getUpdateData(update: UpdateData): UpdateData {
    if ("Delta" in update) {
      let delta = update["Delta"];
      return {
        delta: HostResponse.assertBytes(delta),
      };
    } else {
      throw new TypeError("Invalid update data while building HostResponse");
    }
  }
}

// Versioning:
type WasmContract = ContractV1;

type ContractContainer = WasmContract;

export enum RequestType {
  PutRequest,
  UpdateRequest,
  GetRequest,
  SubscribeRequest,
  DisconnectRequest,
}

export class FlatbuffersClientRequest {
  request_bytes: Uint8Array;
  constructor(requestType: RequestType, requestData: any) {
    let builder = new flatbuffers.Builder();
    let clientReqeustOffset;
    switch (requestType) {
      case RequestType.PutRequest:
        const { container, state, relatedContracts } = requestData as PutRequest;

        const putContainerOffset = this.buildContractContainer(builder, container);
        const putStateOffset = this.buildState(builder, state);
        const putRelatedContractsOffset = this.buildRelatedContracts(builder, relatedContracts);

        Put.startPut(builder);
        Put.addContainer(builder, putContainerOffset);
        Put.addState(builder, putStateOffset);
        Put.addRelatedContracts(builder, putRelatedContractsOffset);
        const putRequestOffset = Put.endPut(builder);

        ContractRequest.startContractRequest(builder);
        ContractRequest.addContractRequestType(
            builder,
            ContractRequestType.Put
        );
        ContractRequest.addContractRequest(
            builder,
            putRequestOffset
        );
        const putContractRequestOffset = ContractRequest.endContractRequest(builder);
        ClientRequest.startClientRequest(builder);
        ClientRequest.addClientRequestType(builder, ClientRequestType.ContractRequest);
        ClientRequest.addClientRequest(
            builder,
            putContractRequestOffset
        );
        clientReqeustOffset = ClientRequest.endClientRequest(builder);
        ClientRequest.finishClientRequestBuffer(builder, clientReqeustOffset);
        break;
      case RequestType.GetRequest:
        const { key: getKey, fetchContract } = requestData as GetRequest;

        const getKeyOffset = this.buildKey(builder, getKey);
        Get.startGet(builder);
        Get.addKey(builder, getKeyOffset);
        Get.addFetchContract(builder, fetchContract);
        const getRequestOffset = Get.endGet(builder);

        ContractRequest.startContractRequest(builder);
        ContractRequest.addContractRequestType(
            builder,
            ContractRequestType.Get
        );
        ContractRequest.addContractRequest(
            builder,
            getRequestOffset
        );

        const getContractRequestOffset = ContractRequest.endContractRequest(builder);

        ClientRequest.startClientRequest(builder);
        ClientRequest.addClientRequestType(builder, ClientRequestType.ContractRequest);
        ClientRequest.addClientRequest(
            builder,
            getContractRequestOffset
        );
        clientReqeustOffset = ClientRequest.endClientRequest(builder);
        ClientRequest.finishClientRequestBuffer(builder, clientReqeustOffset);
        break;
      case RequestType.SubscribeRequest:
        const { key: subscribeKey } = requestData as SubscribeRequest;

        const subscribeKeyOffset = this.buildKey(builder, subscribeKey);
        Subscribe.startSubscribe(builder);
        Subscribe.addKey(builder, subscribeKeyOffset);
        const subscribeRequestOffset = Subscribe.endSubscribe(builder);

        ContractRequest.startContractRequest(builder);
        ContractRequest.addContractRequestType(
            builder,
            ContractRequestType.Subscribe
        );
        ContractRequest.addContractRequest(
            builder,
            subscribeRequestOffset
        );
        const subscribeContractRequestOffset = ContractRequest.endContractRequest(builder);

        ClientRequest.startClientRequest(builder);
        ClientRequest.addClientRequestType(builder, ClientRequestType.ContractRequest);
        ClientRequest.addClientRequest(
            builder,
            subscribeContractRequestOffset
        );
        clientReqeustOffset = ClientRequest.endClientRequest(builder);
        ClientRequest.finishClientRequestBuffer(builder, clientReqeustOffset);
        break;
      case RequestType.DisconnectRequest:
        const { cause } = requestData as DisconnectRequest;

        const causeString = builder.createString(cause);
        Disconnect.startDisconnect(builder);
        Disconnect.addCause(builder, causeString)
        const disconnectOffset = ContractRequest.endContractRequest(builder);
        ClientRequest.startClientRequest(builder);
        ClientRequest.addClientRequestType(builder, ClientRequestType.Disconnect);
        ClientRequest.addClientRequest(
            builder,
            disconnectOffset
        );
        clientReqeustOffset = ClientRequest.endClientRequest(builder);
        ClientRequest.finishClientRequestBuffer(builder, clientReqeustOffset);
        break;
      case RequestType.UpdateRequest:
        const { key: updateKey, data: updateData } = requestData as UpdateRequest;

        const updateKeyOffset = this.buildKey(builder, updateKey);
        const updateDataOffset = this.buildUpdateData(builder, updateData);
        Update.startUpdate(builder);
        Update.addKey(builder,  updateKeyOffset);
        Update.addData(builder, updateDataOffset);
        const updateRequestOffset = Update.endUpdate(builder);

        ContractRequest.startContractRequest(builder);
        ContractRequest.addContractRequestType(
            builder,
            ContractRequestType.Update
        );
        ContractRequest.addContractRequest(
            builder,
            updateRequestOffset
        );

        const updateContractRequestOffset = ContractRequest.endContractRequest(
            builder
        );

        ClientRequest.startClientRequest(builder);
        ClientRequest.addClientRequestType(builder, ClientRequestType.ContractRequest);
        ClientRequest.addClientRequest(
            builder,
            updateContractRequestOffset
        );
        clientReqeustOffset = ClientRequest.endClientRequest(builder);
        ClientRequest.finishClientRequestBuffer(builder, clientReqeustOffset);
        break;
    }
    this.request_bytes = builder.asUint8Array();
  }

  buildContractInstanceId(builder: flatbuffers.Builder, instanceId: Uint8Array): flatbuffers.Offset {
    const data = FbsContractInstanceId.createDataVector(builder, instanceId);
    FbsContractInstanceId.startContractInstanceId(builder);
    FbsContractInstanceId.addData(builder, data);

    return FbsContractInstanceId.endContractInstanceId(builder);
  }

  buildKey(builder: flatbuffers.Builder, key: Key): flatbuffers.Offset {
    const instance = key.bytes();
    let code = key.codePart();
    if (code !== null) {
      code = code as Uint8Array;
    } else {
      code = new Uint8Array();
    }
    const code_vector = FbsKey.createCodeVector(builder, code);
    const instanceOffset = this.buildContractInstanceId(builder, instance);
    FbsKey.startKey(builder);
    FbsKey.addInstance(builder, instanceOffset);
    FbsKey.addCode(builder, code_vector);
    return FbsKey.endKey(builder);
  }

  buildState(builder: flatbuffers.Builder, data: Uint8Array): flatbuffers.Offset {
    const data_vector = FbsKey.createCodeVector(builder, data);
    FbsState.startState(builder);
    FbsState.addData(builder, data_vector);
    return FbsState.endState(builder);
  }

  buildRelatedContract(builder: flatbuffers.Builder, instanceId: Uint8Array, state: Uint8Array): flatbuffers.Offset {
    const instanceOffset = this.buildContractInstanceId(builder, instanceId);
    const stateOffset = this.buildState(builder, state);
    RelatedContract.startRelatedContract(builder);
    RelatedContract.addInstanceId(builder, instanceOffset);
    RelatedContract.addState(builder, stateOffset);
    return RelatedContract.endRelatedContract(builder);
  }

  buildRelatedContracts(builder: flatbuffers.Builder, contracts: RelatedContracts): flatbuffers.Offset {

    let contractOffsets: flatbuffers.Offset[] = [];
    contracts.forEach((instanceId, state) => {
      contractOffsets.push(this.buildRelatedContract(builder, instanceId?.buffer as Uint8Array, state))
    });

    const contracts_vector = FbsRelatedContracts.createContractsVector(builder, contractOffsets);
    FbsRelatedContracts.startRelatedContracts(builder);
    FbsRelatedContracts.addContracts(builder, contracts_vector);
    return FbsRelatedContracts.endRelatedContracts(builder);
  }

  buildContractContainer(builder: flatbuffers.Builder, container: ContractContainer): flatbuffers.Offset {
    const key = container.key
    const data = container.data
    const parameters = container.parameters
    const version = container.version

    const wasmKey = this.buildKey(builder, key);
    const data_vector = FbsContractV1.createDataVector(builder, data);
    const parameters_vector = FbsContractV1.createDataVector(builder, parameters);
    const versionString = builder.createString(version.toString());

    FbsContractV1.startContractV1(builder);
    FbsContractV1.addKey(builder, wasmKey);
    FbsContractV1.addData(builder, data_vector);
    FbsContractV1.addParameters(builder, parameters_vector);
    FbsContractV1.addVersion(builder, versionString);
    const contractOffset = FbsContractV1.endContractV1(builder);

    FbsContractContainer.startContractContainer(builder);
    FbsContractContainer.addContractType(builder, FbsWasmContract.ContractV1);
    FbsContractContainer.addContract(builder, contractOffset);
    return FbsContractContainer.endContractContainer(builder);
  }

  buildUpdateData(builder: flatbuffers.Builder, updateData: UpdateData): flatbuffers.Offset {
    if ('state' in updateData) {
      if ('delta' in updateData) {
        const { state, delta } = updateData;

        const state_vector = FbsState.createDataVector(builder, state);
        FbsState.startState(builder);
        FbsState.createState(builder, state_vector);
        const stateOffset = FbsState.endState(builder);

        const deltaVector = FbsStateDelta.createDataVector(builder, delta);
        FbsStateDelta.startStateDelta(builder);
        FbsStateDelta.createStateDelta(builder, deltaVector);
        const deltaOffset = FbsStateDelta.endStateDelta(builder);

        FbsStateDeltaUpdate.startStateDeltaUpdate(builder);
        FbsStateDeltaUpdate.addState(builder, stateOffset);
        FbsStateDeltaUpdate.addDelta(builder, deltaOffset);
        const stateUpdateOffset = FbsDeltaUpdate.endDeltaUpdate(builder);

        FbsUpdateData.startUpdateData(builder);
        FbsUpdateData.addUpdateDataType(
            builder,
            UpdateDataType.StateUpdate
        );
        FbsUpdateData.addUpdateData(
            builder, stateUpdateOffset
        );
        return FbsUpdateData.endUpdateData(builder);
      } else {
        const { state } = updateData;

        const stateVector = FbsState.createDataVector(builder, state);
        const stateOffset = FbsState.endState(builder);
        FbsState.startState(builder);
        FbsState.createState(builder, stateVector);
        FbsStateUpdate.startStateUpdate(builder);
        FbsStateUpdate.addState(builder, stateOffset);
        const stateUpdateOffset = FbsStateUpdate.endStateUpdate(builder);

        FbsUpdateData.startUpdateData(builder);
        FbsUpdateData.addUpdateDataType(
            builder,
            UpdateDataType.StateUpdate
        );
        FbsUpdateData.addUpdateData(
            builder, stateUpdateOffset
        );
        return FbsUpdateData.endUpdateData(builder);
      }
    } else if ('delta' in updateData) {
      if ('relatedTo' in updateData) {
        const { relatedTo, delta } = updateData;

        const contractInstanceVector = FbsContractInstanceId.createDataVector(builder, relatedTo);
        FbsContractInstanceId.startContractInstanceId(builder);
        FbsContractInstanceId.addData(builder, contractInstanceVector);
        const contractInstanceOffset = FbsContractInstanceId.endContractInstanceId(builder);

        const deltaVector = FbsStateDelta.createDataVector(builder, delta);
        FbsStateDelta.startStateDelta(builder);
        FbsStateDelta.createStateDelta(builder, deltaVector);
        const deltaOffset = FbsStateDelta.endStateDelta(builder);

        FbsDeltaWithContractInstanceUpdate.startDeltaWithContractInstanceUpdate(builder);
        FbsDeltaWithContractInstanceUpdate.addDelta(builder, deltaOffset);
        FbsDeltaWithContractInstanceUpdate.addRelatedTo(builder, contractInstanceOffset);
        const deltaUpdateOffset = FbsDeltaWithContractInstanceUpdate.endDeltaWithContractInstanceUpdate(builder);

        FbsUpdateData.startUpdateData(builder);
        FbsUpdateData.addUpdateDataType(
            builder,
            UpdateDataType.DeltaWithContractInstanceUpdate
        );
        FbsUpdateData.addUpdateData(
            builder, deltaUpdateOffset
        );
        return FbsUpdateData.endUpdateData(builder);
      } else {
        const { delta } = updateData;

        const deltaVector = FbsStateDelta.createDataVector(builder, delta);
        FbsStateDelta.startStateDelta(builder);
        FbsStateDelta.createStateDelta(builder, deltaVector);
        const deltaOffset = FbsStateDelta.endStateDelta(builder);

        FbsDeltaUpdate.startDeltaUpdate(builder);
        FbsDeltaUpdate.addDelta(builder, deltaOffset);
        const stateUpdateOffset = FbsDeltaUpdate.endDeltaUpdate(builder);

        FbsUpdateData.startUpdateData(builder);
        FbsUpdateData.addUpdateDataType(
            builder,
            UpdateDataType.StateUpdate
        );
        FbsUpdateData.addUpdateData(
            builder, stateUpdateOffset
        );
        return FbsUpdateData.endUpdateData(builder);
      }
    } else if ('relatedTo' in updateData) {
      const { relatedTo, state } = updateData;

      const contractInstanceVector = FbsContractInstanceId.createDataVector(builder, relatedTo);
      FbsContractInstanceId.startContractInstanceId(builder);
      FbsContractInstanceId.addData(builder, contractInstanceVector);
      const contractInstanceOffset = FbsContractInstanceId.endContractInstanceId(builder);

      const stateVector = FbsState.createDataVector(builder, state);
      FbsContractInstanceId.endContractInstanceId(builder);
      FbsState.startState(builder);
      FbsState.createState(builder, stateVector);
      const stateOffset = FbsState.endState(builder);

      FbsStateWithContractInstanceUpdate.startStateWithContractInstanceUpdate(builder);
      FbsStateWithContractInstanceUpdate.addState(builder, stateOffset);
      FbsStateWithContractInstanceUpdate.addRelatedTo(builder, contractInstanceOffset);
      const stateUpdateOffset = FbsStateWithContractInstanceUpdate.endStateWithContractInstanceUpdate(builder);

      FbsUpdateData.startUpdateData(builder);
      FbsUpdateData.addUpdateDataType(
          builder,
          UpdateDataType.StateWithContractInstanceUpdate
      );
      FbsUpdateData.addUpdateData(
          builder, stateUpdateOffset
      );
      return FbsUpdateData.endUpdateData(builder);
    } else {
      const { relatedTo, state, delta } = updateData;

      const contractInstanceVector = FbsContractInstanceId.createDataVector(builder, relatedTo);
      FbsContractInstanceId.startContractInstanceId(builder);
      FbsContractInstanceId.addData(builder, contractInstanceVector);
      const contractInstanceOffset = FbsContractInstanceId.endContractInstanceId(builder);

      const stateVector = FbsState.createDataVector(builder, state);
      FbsState.startState(builder);
      FbsState.createState(builder, stateVector);
      const stateOffset = FbsState.endState(builder);

      const deltaVector = FbsStateDelta.createDataVector(builder, delta);
      FbsStateDelta.startStateDelta(builder);
      FbsStateDelta.createStateDelta(builder, deltaVector);
      const deltaOffset = FbsStateDelta.endStateDelta(builder);

      FbsStateDeltaWithContractInstanceUpdate.startStateDeltaWithContractInstanceUpdate(builder);
      FbsStateDeltaWithContractInstanceUpdate.addState(builder, stateOffset);
      FbsStateDeltaWithContractInstanceUpdate.addDelta(builder, deltaOffset);
      FbsStateDeltaWithContractInstanceUpdate.addRelatedTo(builder, contractInstanceOffset);
      const stateDeltaWithContractInstanceUpdateOffset = FbsStateDeltaWithContractInstanceUpdate.endStateDeltaWithContractInstanceUpdate(builder);

      FbsUpdateData.startUpdateData(builder);
      FbsUpdateData.addUpdateDataType(
          builder,
          UpdateDataType.StateDeltaWithContractInstanceUpdate
      );
      FbsUpdateData.addUpdateData(
          builder, stateDeltaWithContractInstanceUpdateOffset
      );
      return FbsUpdateData.endUpdateData(builder);
    }
  }
}