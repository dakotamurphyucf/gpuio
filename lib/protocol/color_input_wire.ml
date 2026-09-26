open Core

let max_config_bytes = 98_304
let max_event_bytes = 4352
let max_command_bytes = 32

module Channel = struct
  type t =
    | Hue
    | Saturation
    | Lightness
    | Alpha
  [@@deriving bin_io, equal, sexp_of]
end

module Field = struct
  type t =
    | Hex
    | Channel of Channel.t
  [@@deriving bin_io, equal, sexp_of]
end

module Alpha_policy = struct
  type t =
    | Allow_alpha
    | Opaque_only
  [@@deriving bin_io, equal, sexp_of]
end

module Interaction_kind = struct
  type t =
    | Drag of Channel.t
    | Text of Field.t
  [@@deriving bin_io, equal, sexp_of]
end

module Draft_status = struct
  type t =
    | Empty
    | Incomplete
    | Invalid
    | Out_of_range
    | Forbidden
    | Valid
  [@@deriving bin_io, equal, sexp_of]
end

module Source = struct
  type t =
    | Pointer
    | Keyboard
    | Accessibility
    | Text
    | Palette
    | Clear
  [@@deriving bin_io, equal, sexp_of]
end

module Cancel_reason = struct
  type t =
    | Escape
    | Configuration_changed
    | Disabled
    | Read_only
    | Hidden
    | Modal
    | Window_inactive
    | Unmounted
    | Programmatic
    | Interrupted
  [@@deriving bin_io, equal, sexp_of]
end

module Error = struct
  type t =
    | Not_mounted
    | Closed
    | Stale_color_input
    | Stale_revision
    | Stale_interaction
    | Disabled
    | Read_only
    | Focus_blocked
    | Busy
    | Invalid_value
    | Invalid_config
    | Invalid_draft
    | Composing
    | Limit_exceeded
    | Native_failure
  [@@deriving bin_io, equal, sexp_of]
end

module Rgba = struct
  type t = int64 [@@deriving bin_io, equal, sexp_of]

  let valid t = Int64.(t >= 0L && t <= 0xffffffffL)
end

module Value = struct
  type t =
    | Empty
    | Color of Rgba.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Empty -> true
    | Color rgba -> Rgba.valid rgba
  ;;
end

module Hsla = struct
  type t =
    { hue_degrees : float
    ; saturation : float
    ; lightness : float
    ; alpha : float
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Float.is_finite t.hue_degrees
    && Float.(t.hue_degrees >= 0. && t.hue_degrees <= 360.)
    && List.for_all [ t.saturation; t.lightness; t.alpha ] ~f:(fun x ->
      Float.is_finite x && Float.(x >= 0. && x <= 1.))
  ;;

  (* Wire validation must not depend on the higher-level Color_value module.
     This is the same encoded-sRGB conversion, checked against independent fixtures. *)
  let rgba t =
    let hue = (if Float.equal t.hue_degrees 360. then 0. else t.hue_degrees) /. 60. in
    let c = (1. -. Float.abs ((2. *. t.lightness) -. 1.)) *. t.saturation in
    let x = c *. (1. -. Float.abs (Float.mod_float hue 2. -. 1.)) in
    let r, g, b =
      match Float.to_int hue with
      | 0 -> c, x, 0.
      | 1 -> x, c, 0.
      | 2 -> 0., c, x
      | 3 -> 0., x, c
      | 4 -> x, 0., c
      | _ -> c, 0., x
    in
    let offset = t.lightness -. (c /. 2.) in
    let byte value =
      Float.to_int (Float.round_down ((Float.max 0. (Float.min 1. value) *. 255.) +. 0.5))
      |> Int64.of_int
    in
    Int64.(
      bit_or
        (shift_left (byte (r +. offset)) 24)
        (bit_or
           (shift_left (byte (g +. offset)) 16)
           (bit_or (shift_left (byte (b +. offset)) 8) (byte t.alpha))))
  ;;
end

let valid_label text limit =
  String.length text <= limit
  && Stdlib.String.is_valid_utf_8 text
  && (not (String.contains text '\000'))
  && not (String.is_empty (String.strip text ~drop:Numeric_wire.Draft.whitespace))
;;

module Palette_entry = struct
  type t =
    { color : Rgba.t
    ; label : string
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = Rgba.valid t.color && valid_label t.label 256
end

module Labels = struct
  type t =
    { control : string
    ; hue : string
    ; saturation : string
    ; lightness : string
    ; alpha : string
    ; hex : string
    ; clear : string
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    List.for_all
      [ t.control; t.hue; t.saturation; t.lightness; t.alpha; t.hex; t.clear ]
      ~f:(fun s -> valid_label s 4096)
  ;;
end

module Config = struct
  type t =
    { labels : Labels.t
    ; palette : Palette_entry.t list
    ; alpha_policy : Alpha_policy.t
    ; allow_empty : bool
    ; disabled : bool
    ; read_only : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Labels.valid t.labels
    && List.length t.palette <= 256
    && List.for_all t.palette ~f:Palette_entry.valid
  ;;

  let allows t = function
    | Value.Empty -> t.allow_empty
    | Color c ->
      (match t.alpha_policy with
       | Allow_alpha -> true
       | Opaque_only -> Int64.equal (Int64.bit_and c 255L) 255L)
  ;;
end

module Interaction = struct
  type t =
    { id : int64
    ; kind : Interaction_kind.t
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Draft = struct
  type t =
    { text : string
    ; composing : bool
    ; status : Draft_status.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    String.length t.text <= 4096
    && Stdlib.String.is_valid_utf_8 t.text
    && not
         (String.exists t.text ~f:(function
            | '\000' | '\n' | '\r' -> true
            | _ -> false))
  ;;
end

let draft_status_valid (draft : Draft.t) field (channels : Hsla.t) =
  let status, candidate_alpha =
    match field with
    | Field.Hex ->
      let text = draft.text in
      let digits =
        if String.is_prefix text ~prefix:"#" then String.drop_prefix text 1 else text
      in
      let digit = function
        | '0' .. '9' as c -> Some (Char.to_int c - Char.to_int '0')
        | 'a' .. 'f' as c -> Some (Char.to_int c - Char.to_int 'a' + 10)
        | 'A' .. 'F' as c -> Some (Char.to_int c - Char.to_int 'A' + 10)
        | _ -> None
      in
      if String.is_empty text
      then Draft_status.Empty, None
      else if
        String.length text > 9
        || not (String.for_all digits ~f:(fun c -> Option.is_some (digit c)))
      then Invalid, None
      else (
        match String.length digits with
        | 0 | 1 | 2 | 5 | 7 -> Incomplete, None
        | 3 | 6 -> Valid, Some 1.
        | 4 ->
          Valid, Some (Float.of_int (Option.value_exn (digit digits.[3]) * 17) /. 255.)
        | 8 ->
          ( Valid
          , Some
              (Float.of_int
                 ((Option.value_exn (digit digits.[6]) * 16)
                  + Option.value_exn (digit digits.[7]))
               /. 255.) )
        | _ -> Invalid, None)
    | Channel channel ->
      let max =
        match channel with
        | Hue -> 360.
        | Saturation | Lightness | Alpha -> 100.
      in
      (match Numeric_wire.Draft.parse { min = 0.; max; step = 1. } draft.text with
       | Empty -> Draft_status.Empty, None
       | Incomplete -> Incomplete, None
       | Invalid _ -> Invalid, None
       | Out_of_range _ -> Out_of_range, None
       | Valid value ->
         ( Valid
         , Some
             (match channel with
              | Alpha -> value /. 100.
              | Hue | Saturation | Lightness -> channels.alpha) ))
  in
  Draft_status.equal draft.status status
  || (Draft_status.equal draft.status Forbidden
      && Option.exists candidate_alpha ~f:(fun a -> not (Float.equal a 1.)))
;;

module Snapshot = struct
  type t =
    { revision : int64
    ; value : Value.t
    ; committed : Value.t
    ; channels : Hsla.t
    ; interaction : Interaction.t option
    ; draft : Draft.t option
    ; value_allowed : bool
    ; committed_allowed : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.revision >= 0L)
    && Value.valid t.value
    && Value.valid t.committed
    && Hsla.valid t.channels
    && (match t.value with
        | Empty -> true
        | Color c -> Int64.equal c (Hsla.rgba t.channels))
    &&
    match t.interaction, t.draft with
    | None, None ->
      Value.equal t.value t.committed && Bool.equal t.value_allowed t.committed_allowed
    | Some { id; kind = Drag _ }, None -> Int64.(id > 0L && id <= t.revision)
    | Some { id; kind = Text field }, Some draft ->
      Int64.(id > 0L && id <= t.revision)
      && Draft.valid draft
      && draft_status_valid draft field t.channels
    | _ -> false
  ;;
end

module Event = struct
  type t =
    | Observed of Snapshot.t
    | Started of Snapshot.t
    | Preview of Snapshot.t
    | Committed of Source.t * Snapshot.t
    | Cancelled of Cancel_reason.t * Snapshot.t
  [@@deriving bin_io, equal, sexp_of]

  let snapshot = function
    | Observed s | Started s | Preview s | Committed (_, s) | Cancelled (_, s) -> s
  ;;

  let valid t =
    let s = snapshot t in
    Snapshot.valid s
    &&
    match t with
    | Observed _ -> true
    | Started _ ->
      Option.exists s.interaction ~f:(fun i -> Int64.equal i.id s.revision)
      && Value.equal s.value s.committed
    | Preview _ -> Option.is_some s.interaction
    | Committed _ | Cancelled _ -> Option.is_none s.interaction && Int64.(s.revision > 0L)
  ;;
end

module Command = struct
  type t =
    | Set of
        { value : Value.t
        ; if_revision : int64 option
        }
    | Reset of { if_revision : int64 option }
    | Cancel
    | Focus of Field.t
    | Read_snapshot
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Set { value; if_revision } ->
      Value.valid value && Option.for_all if_revision ~f:(fun r -> Int64.(r >= 0L))
    | Reset { if_revision } -> Option.for_all if_revision ~f:(fun r -> Int64.(r >= 0L))
    | Cancel | Focus _ | Read_snapshot -> true
  ;;
end

module Response = struct
  type t =
    | Applied of Snapshot.t
    | Failed of Error.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Applied s -> Snapshot.valid s
    | Failed _ -> true
  ;;
end
