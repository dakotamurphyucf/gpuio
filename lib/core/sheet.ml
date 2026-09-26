open Core

module Edge = struct
  type t =
    | Left
    | Right
    | Top
    | Bottom
  [@@deriving equal, sexp_of]
end

module Config = struct
  type t =
    { edge : Edge.t
    ; overlay : Overlay.Config.t
    }
  [@@deriving equal, sexp_of]

  let create
        ~label
        ?(edge = Edge.Right)
        ?(extent = 360.)
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
      { edge; overlay })
  ;;
end

module Expert = struct
  let overlay (t : Config.t) = t.overlay

  let kind (t : Config.t) : Gpuio_protocol.Wire.Overlay_kind.t =
    match t.edge with
    | Left -> Sheet_left
    | Right -> Sheet_right
    | Top -> Sheet_top
    | Bottom -> Sheet_bottom
  ;;
end
