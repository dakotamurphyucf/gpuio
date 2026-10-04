open Core

let valid_id id =
  String.length id >= 1
  && String.length id <= 64
  && Char.is_alpha id.[0]
  && String.for_all id ~f:(fun c -> Char.is_alphanum c || String.mem "_.-" c)
;;

let valid_text text = Stdlib.String.is_valid_utf_8 text

module Action = struct
  type t =
    { id : string
    ; label : string
    ; enabled : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    valid_id t.id
    && (not (String.is_empty t.label))
    && String.length t.label <= 256
    && valid_text t.label
    && not (String.contains t.label '\000')
  ;;
end

module Config = struct
  type t =
    { epoch : int64
    ; observe : bool
    ; copy_code : bool
    ; copy_table : bool
    ; code : Action.t list
    ; table : Action.t list
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    let group actions =
      List.length actions <= 16
      && List.for_all actions ~f:Action.valid
      && Set.length (String.Set.of_list (List.map actions ~f:(fun a -> a.Action.id)))
         = List.length actions
    in
    Int64.(t.epoch > 0L)
    && group t.code
    && group t.table
    && (t.observe || (List.is_empty t.code && List.is_empty t.table))
  ;;
end

module Source_range = struct
  type t =
    { start_byte : int64
    ; end_byte : int64
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.start_byte >= 0L && t.start_byte <= t.end_byte && t.end_byte <= 65536L)
  ;;
end

module Block = struct
  type t =
    | Code of string option * string
    | Table of string list * string list list * string
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Code (language, code) ->
      Option.for_all language ~f:(fun l -> String.length l <= 4096 && valid_text l)
      && String.length code <= 65536
      && valid_text code
    | Table (headers, rows, markdown) ->
      List.length rows <= 4096
      && List.length headers <= 4096
      && String.length markdown <= 262144
      && valid_text markdown
      && List.fold rows ~init:(List.length headers) ~f:(fun total row ->
           total + List.length row)
         <= 4096
      && List.for_all headers ~f:valid_text
      && List.for_all rows ~f:(List.for_all ~f:valid_text)
      && List.fold (headers :: rows) ~init:(String.length markdown) ~f:(fun total row ->
           List.fold row ~init:total ~f:(fun n text -> n + String.length text))
         <= 262144
  ;;
end

module Event = struct
  type t =
    { config_epoch : int64
    ; action : string
    ; source_revision : int64
    ; source_generation : int64
    ; source_range : Source_range.t option
    ; block : Block.t
    ; activation : Document_wire.Activation.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.config_epoch > 0L && t.source_revision > 0L && t.source_generation > 0L)
    && valid_id t.action
    && Option.for_all t.source_range ~f:Source_range.valid
    && Block.valid t.block
    && Document_wire.Activation.valid t.activation
  ;;
end
