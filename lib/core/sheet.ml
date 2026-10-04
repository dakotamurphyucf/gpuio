open Core

module Edge = struct
  type t =
    | Left
    | Right
    | Top
    | Bottom
  [@@deriving equal, sexp_of]
end

module Insets = struct
  type t = Gpuio_protocol.Sheet_insets_wire.t =
    { top : float
    ; right : float
    ; bottom : float
    ; left : float
    }
  [@@deriving equal, sexp_of]

  let zero = { top = 0.; right = 0.; bottom = 0.; left = 0. }

  let create ?(top = 0.) ?(right = 0.) ?(bottom = 0.) ?(left = 0.) () =
    let t = { top; right; bottom; left } in
    if Gpuio_protocol.Sheet_insets_wire.valid t
    then Ok t
    else
      Or_error.error_string "sheet insets must be finite and in 0..16384 logical pixels"
  ;;
end

module Config = struct
  type t =
    { edge : Edge.t
    ; overlay : Overlay.Config.t
    ; insets : Insets.t
    }
  [@@deriving equal, sexp_of]

  let create
        ~label
        ?(edge = Edge.Right)
        ?(extent = 360.)
        ?(insets = Insets.zero)
        ?(dismiss_on_escape = true)
        ?(dismiss_on_outside_pointer = true)
        ()
    =
    if (not (Float.is_finite extent)) || Float.(extent < 1. || extent > 16384.)
    then
      Or_error.error_string "sheet extent must be finite and in 1..16384 logical pixels"
    else (
      let%map.Or_error overlay =
        Overlay.Config.create
          ~label
          ~width:extent
          ~dismiss_on_escape
          ~dismiss_on_outside_pointer
          ()
      in
      { edge; overlay; insets })
  ;;
end

module Expert = struct
  let insets (t : Config.t) =
    if Insets.equal t.insets Insets.zero then None else Some t.insets
  ;;

  let insets_to_wire (t : Insets.t) = t
  let overlay (t : Config.t) = t.overlay

  let kind (t : Config.t) : Gpuio_protocol.Wire.Overlay_kind.t =
    match t.edge with
    | Left -> Sheet_left
    | Right -> Sheet_right
    | Top -> Sheet_top
    | Bottom -> Sheet_bottom
  ;;
end
