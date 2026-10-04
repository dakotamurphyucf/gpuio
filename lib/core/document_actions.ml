open Core
module Wire = Gpuio_protocol.Document_actions_wire

module Id = struct
  type t = string [@@deriving equal, compare, sexp_of]

  let of_string id =
    if Wire.valid_id id then Ok id else Or_error.error_string "invalid document action ID"
  ;;

  let to_string t = t
end

module Action = struct
  type t = Wire.Action.t [@@deriving equal, sexp_of]

  let create ~id ~label ?(enabled = true) () =
    let t = { Wire.Action.id; label; enabled } in
    if Wire.Action.valid t
    then Ok t
    else Or_error.error_string "invalid document action label"
  ;;

  let id t = t.Wire.Action.id
  let label t = t.Wire.Action.label
  let enabled t = t.Wire.Action.enabled
end

module Config = struct
  type t =
    { code : Action.t list
    ; table : Action.t list
    ; copy_code : bool
    ; copy_table : bool
    }
  [@@deriving equal, sexp_of]

  let create ?(code = []) ?(table = []) ?(copy_code = true) ?(copy_table = true) () =
    if
      Wire.Config.valid { epoch = 1L; observe = true; code; table; copy_code; copy_table }
    then Ok { code; table; copy_code; copy_table }
    else Or_error.error_string "document actions require at most16 unique IDs per family"
  ;;

  let default = { code = []; table = []; copy_code = true; copy_table = true }
  let code t = t.code
  let table t = t.table
  let copy_code t = t.copy_code
  let copy_table t = t.copy_table
end

module Source_range = struct
  type t =
    { start_byte : int
    ; end_byte : int
    }
  [@@deriving equal, sexp_of]
end

module Block = struct
  type t =
    | Code of
        { language : string option
        ; code : string
        }
    | Table of
        { headers : string list
        ; rows : string list list
        ; markdown : string
        }
  [@@deriving equal, sexp_of]
end

module Event = struct
  type t =
    { action : Id.t
    ; source_revision : int64
    ; source_generation : int64
    ; source_range : Source_range.t option
    ; block : Block.t
    ; activation : Document_activation.t
    }
  [@@deriving equal, sexp_of]
end

module Expert = struct
  let to_wire (t : Config.t) ~epoch ~observe : Wire.Config.t =
    { epoch
    ; observe
    ; code = t.code
    ; table = t.table
    ; copy_code = t.copy_code
    ; copy_table = t.copy_table
    }
  ;;

  let event_of_wire ~config (event : Wire.Event.t) =
    let actions =
      match event.block with
      | Code _ -> Config.code config
      | Table _ -> Config.table config
    in
    if
      (not (Wire.Event.valid event))
      || not
           (List.exists actions ~f:(fun a ->
              Action.enabled a && Id.equal (Action.id a) event.action))
    then Or_error.error_string "invalid or unavailable document action"
    else
      let open Or_error.Let_syntax in
      let%map activation = Document_activation.Expert.of_wire event.activation in
      let block : Block.t =
        match event.block with
        | Code (language, code) -> Code { language; code }
        | Table (headers, rows, markdown) -> Table { headers; rows; markdown }
      in
      { Event.action = event.action
      ; source_revision = event.source_revision
      ; source_generation = event.source_generation
      ; activation
      ; block
      ; source_range =
          Option.map event.source_range ~f:(fun span ->
            { Source_range.start_byte = Int64.to_int_exn span.start_byte
            ; end_byte = Int64.to_int_exn span.end_byte
            })
      }
  ;;
end
