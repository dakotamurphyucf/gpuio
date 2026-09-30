open Core
module Wire = Gpuio_protocol.Highlight_wire

module Query = struct
  type t = Wire.Query.t [@@deriving equal, sexp_of]

  let create ?(case_sensitive = false) ?(whole_word = false) text =
    let t = { Wire.Query.text; case_sensitive; whole_word } in
    if Wire.Query.valid t
    then Ok t
    else Or_error.error_string "highlight query requires 1..4096 UTF-8 bytes without NUL"
  ;;

  let text t = t.Wire.Query.text
end

module Range = struct
  type t = Wire.Range.t [@@deriving equal, sexp_of]

  let create ~start_byte ~end_byte =
    let t = { Wire.Range.start_byte; end_byte } in
    if Wire.Range.valid t
    then Ok t
    else Or_error.error_string "highlight range requires 0 <= start_byte < end_byte"
  ;;

  let start_byte t = t.Wire.Range.start_byte
  let end_byte t = t.Wire.Range.end_byte
end

module Appearance = struct
  type t = Wire.Appearance.t [@@deriving equal, sexp_of]

  let with_opacity rgba percent =
    let alpha = Int64.bit_and rgba 0xffL |> Int64.to_int_exn in
    let alpha = ((alpha * percent) + 50) / 100 |> Int64.of_int in
    Int64.bit_or (Int64.bit_and rgba 0xffff_ff00L) alpha
  ;;

  let create ?color ?active_color ?(radius = 2.) ?(theme = Theme.default) () =
    let open Or_error.Let_syntax in
    let resolve explicit percent =
      match explicit with
      | Some color -> Theme.resolve theme color
      | None ->
        Theme.resolve theme (Color.token_exn "accent")
        |> Or_error.map ~f:(fun rgba -> with_opacity rgba percent)
    in
    let%bind color = resolve color 30 in
    let%bind active_color = resolve active_color 65 in
    let t = { Wire.Appearance.color; active_color; radius } in
    if Wire.Appearance.valid t
    then Ok t
    else Or_error.error_string "highlight radius must be finite and in 0..64"
  ;;

  let default = create () |> Or_error.ok_exn
end

module Spec = struct
  type t = Wire.Spec.t [@@deriving equal, sexp_of]

  let create
        ?query
        ?(ranges = [])
        ?(appearance = Appearance.default)
        ?active_index
        ?(match_index_offset = 0L)
        ()
    =
    let t = { Wire.Spec.query; ranges; appearance; active_index; match_index_offset } in
    if Wire.Spec.valid t
    then Ok t
    else
      Or_error.error_string
        "highlight spec requires a query or up to 4096 ranges and nonnegative match \
         indices"
  ;;
end

module Config = struct
  type t = Wire.Config.t [@@deriving equal, sexp_of]

  let empty = []

  let create t =
    if Wire.Config.valid t
    then Ok t
    else
      Or_error.error_string "highlight config exceeds limits or contains an invalid spec"
  ;;

  let specs t = t
end

module Count = struct
  type t = Wire.Count.t =
    { total : int64
    ; stored : int64
    }
  [@@deriving equal, sexp_of]
end

module Range_error = Wire.Range_error

module Invalid_range = struct
  type t =
    { spec_index : int
    ; range_index : int
    ; reason : Range_error.t
    }
  [@@deriving equal, sexp_of]
end

module Limit = Wire.Limit
module Failure = Wire.Failure

module State = struct
  type t =
    | Pending
    | Ready of Count.t list
    | Invalid_range of Invalid_range.t
    | Capacity of Limit.t
    | Failed of Failure.t
  [@@deriving equal, sexp_of]
end

module Observation = struct
  type t =
    { epoch : int64
    ; state : State.t
    }
  [@@deriving equal, sexp_of]
end

module Expert = struct
  let to_wire t = t
  let of_wire = Config.create

  let observation_of_wire (t : Wire.Observation.t) =
    if not (Wire.Observation.valid t)
    then Or_error.error_string "invalid highlight observation"
    else (
      let state : State.t =
        match t.state with
        | Pending -> Pending
        | Ready counts -> Ready counts
        | Invalid_range r ->
          Invalid_range
            { spec_index = Int64.to_int_exn r.spec_index
            ; range_index = Int64.to_int_exn r.range_index
            ; reason = r.reason
            }
        | Capacity limit -> Capacity limit
        | Failed failure -> Failed failure
      in
      Ok { Observation.epoch = t.epoch; state })
  ;;
end
