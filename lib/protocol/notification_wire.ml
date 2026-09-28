open Core

let max_tag_bytes = 128
let max_action_id_bytes = 64
let max_label_bytes = 128
let max_title_bytes = 256
let max_body_bytes = 8192
let max_actions = 4
let max_live = 128
let max_pending = 16
let max_events = max_live + 1

let text s ~limit ~multiline ~allow_empty =
  String.length s <= limit
  && Stdlib.String.is_valid_utf_8 s
  && (allow_empty || not (String.is_empty (String.strip s)))
  && String.for_all s ~f:(fun c ->
    let n = Char.to_int c in
    (n >= 32 && n <> 127)
    || (multiline && (Char.equal c '\n' || Char.equal c '\r' || Char.equal c '\t')))
;;

let valid_tag s = text s ~limit:max_tag_bytes ~multiline:false ~allow_empty:false

let valid_action_id s =
  (not (String.is_empty s))
  && String.length s <= max_action_id_bytes
  && String.for_all s ~f:(fun c ->
    Char.is_alpha c || Char.is_digit c || String.contains "._-" c)
;;

module Action = struct
  type t =
    { id : string
    ; label : string
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    valid_action_id t.id
    && text t.label ~limit:max_label_bytes ~multiline:false ~allow_empty:false
  ;;
end

module Sound = struct
  type t =
    | Silent
    | Default
  [@@deriving bin_io, equal, sexp_of]
end

module Content = struct
  type t =
    { title : string
    ; body : string
    ; actions : Action.t list
    ; sound : Sound.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    text t.title ~limit:max_title_bytes ~multiline:false ~allow_empty:false
    && text t.body ~limit:max_body_bytes ~multiline:true ~allow_empty:true
    && List.length t.actions <= max_actions
    && List.for_all t.actions ~f:Action.valid
    && not
         (List.contains_dup
            (List.map t.actions ~f:(fun a -> a.id))
            ~compare:String.compare)
  ;;
end

module Receipt = struct
  type t =
    { id : int64
    ; tag : string
    }
  [@@deriving bin_io, equal, compare, sexp_of]

  let valid t = Int64.(t.id > 0L) && valid_tag t.tag
end

module Authorization = struct
  type t =
    | Not_determined
    | Denied
    | Authorized
    | Provisional
    | Not_required
  [@@deriving bin_io, equal, sexp_of]
end

module Capabilities = struct
  type t =
    { body : bool
    ; actions : bool
    ; activation : bool
    ; replacement : bool
    ; dismissal : bool
    ; permission_request : bool
    ; sound : bool
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Error = struct
  type t =
    | Invalid_request
    | Not_ready
    | Unsupported
    | Unavailable
    | Denied
    | Busy
    | Closed
    | Stale
    | Native_failure
  [@@deriving bin_io, equal, sexp_of]
end

module Closed_reason = struct
  type t =
    | Expired
    | User
    | Platform
  [@@deriving bin_io, equal, sexp_of]
end

module Event = struct
  type t =
    | Activated of Receipt.t
    | Action of Receipt.t * string
    | Closed of Receipt.t * Closed_reason.t
    | Failed of Error.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Activated receipt | Closed (receipt, _) -> Receipt.valid receipt
    | Action (receipt, id) -> Receipt.valid receipt && valid_action_id id
    | Failed _ -> true
  ;;
end

module Request = struct
  type t =
    | Capabilities
    | Authorization
    | Request_authorization
    | Post of string * Content.t
    | Replace of Receipt.t * Content.t
    | Dismiss of Receipt.t
    | Take_events
    | Close
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Post (tag, content) -> valid_tag tag && Content.valid content
    | Replace (receipt, content) -> Receipt.valid receipt && Content.valid content
    | Dismiss receipt -> Receipt.valid receipt
    | Capabilities | Authorization | Request_authorization | Take_events | Close -> true
  ;;
end

module Response = struct
  type t =
    | Capabilities of Capabilities.t
    | Authorization of Authorization.t
    | Posted of Receipt.t
    | Replaced
    | Dismiss_requested
    | Events of Event.t list
    | Closed
    | Failed of Error.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Posted receipt -> Receipt.valid receipt
    | Events events ->
      List.length events <= max_events && List.for_all events ~f:Event.valid
    | Capabilities _ | Authorization _ | Replaced | Dismiss_requested | Closed | Failed _
      -> true
  ;;
end
