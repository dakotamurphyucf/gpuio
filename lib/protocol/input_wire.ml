open Core
module Button = Pointer_wire.Button
module Modifiers = Pointer_wire.Modifiers

module Kind = struct
  type t =
    | Click
    | Auxiliary_click
    | Mouse_down
    | Mouse_up
    | Mouse_move
    | Mouse_enter
    | Mouse_leave
    | Mouse_down_outside
    | Key_down
    | Key_up
    | Focus
    | Blur
    | Scroll
  [@@deriving bin_io, equal, compare, sexp_of]
end

module Phase = struct
  type t =
    | Capture
    | Bubble
  [@@deriving bin_io, equal, sexp_of]
end

module Policy = struct
  type t =
    | Observe
    | Stop_propagation
    | Prevent_default
    | Prevent_and_stop
  [@@deriving bin_io, equal, sexp_of]
end

module Subscription = struct
  type t =
    { kind : Kind.t
    ; phase : Phase.t
    ; policy : Policy.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    match t.kind with
    | Mouse_down | Mouse_up | Mouse_move | Key_down | Key_up | Scroll -> true
    | Click
    | Auxiliary_click
    | Mouse_enter
    | Mouse_leave
    | Mouse_down_outside
    | Focus
    | Blur -> Phase.equal t.phase Bubble && Policy.equal t.policy Observe
  ;;
end

module Focus = struct
  type t =
    | None
    | Click
    | Tab
  [@@deriving bin_io, equal, sexp_of]
end

let text_valid s ~max_bytes =
  String.length s <= max_bytes
  && (not (String.is_empty s))
  && Stdlib.String.is_valid_utf_8 s
  && not (String.contains s '\000')
;;

module Config = struct
  type t =
    { label : string
    ; disabled : bool
    ; focus : Focus.t
    ; subscriptions : Subscription.t list
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    text_valid t.label ~max_bytes:4096
    && (not (String.is_empty (String.strip t.label ~drop:Numeric_wire.Draft.whitespace)))
    && (not (List.is_empty t.subscriptions))
    && List.length t.subscriptions <= 13
    && List.for_all t.subscriptions ~f:Subscription.valid
    && List.is_sorted_strictly t.subscriptions ~compare:(fun a b ->
      Kind.compare a.kind b.kind)
    && ((not (Focus.equal t.focus None))
        || List.for_all t.subscriptions ~f:(fun s ->
          not (Kind.equal s.kind Focus || Kind.equal s.kind Blur)))
  ;;
end

module Position = struct
  type t =
    { x : float
    ; y : float
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = Float.is_finite t.x && Float.is_finite t.y
end

module Location = struct
  type t =
    { window : Position.t
    ; local : Position.t
    ; modifiers : Modifiers.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = Position.valid t.window && Position.valid t.local
end

module Mouse = struct
  type t =
    { location : Location.t
    ; button : Button.t
    ; click_count : int64
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Location.valid t.location
    && Int64.(t.click_count > 0L && t.click_count <= 0xffff_ffffL)
  ;;
end

module Motion = struct
  type t =
    { location : Location.t
    ; pressed_button : Button.t option
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Key = struct
  type t =
    { key : string
    ; character : string option
    ; modifiers : Modifiers.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    text_valid t.key ~max_bytes:256
    && Option.for_all t.character ~f:(fun s -> text_valid s ~max_bytes:256)
  ;;
end

module Delta = struct
  type t =
    | Pixels of Position.t
    | Lines of Position.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Pixels p | Lines p -> Position.valid p
  ;;
end

module Touch_phase = struct
  type t =
    | Started
    | Moved
    | Ended
    | Cancelled
  [@@deriving bin_io, equal, sexp_of]
end

module Scroll = struct
  type t =
    { location : Location.t
    ; delta : Delta.t
    ; phase : Touch_phase.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = Location.valid t.location && Delta.valid t.delta
end

module Event = struct
  type t =
    | Click of Mouse.t
    | Auxiliary_click of Mouse.t
    | Mouse_down of Mouse.t
    | Mouse_up of Mouse.t
    | Mouse_move of Motion.t
    | Mouse_enter
    | Mouse_leave
    | Mouse_down_outside of Mouse.t
    | Key_down of Key.t * bool
    | Key_up of Key.t
    | Focus
    | Blur
    | Scroll of Scroll.t
  [@@deriving bin_io, equal, sexp_of]

  let kind : t -> Kind.t = function
    | Click _ -> Click
    | Auxiliary_click _ -> Auxiliary_click
    | Mouse_down _ -> Mouse_down
    | Mouse_up _ -> Mouse_up
    | Mouse_move _ -> Mouse_move
    | Mouse_enter -> Mouse_enter
    | Mouse_leave -> Mouse_leave
    | Mouse_down_outside _ -> Mouse_down_outside
    | Key_down _ -> Key_down
    | Key_up _ -> Key_up
    | Focus -> Focus
    | Blur -> Blur
    | Scroll _ -> Scroll
  ;;

  let valid = function
    | Click mouse -> Mouse.valid mouse && Button.equal mouse.button Left
    | Auxiliary_click mouse -> Mouse.valid mouse && not (Button.equal mouse.button Left)
    | Mouse_down mouse | Mouse_up mouse | Mouse_down_outside mouse -> Mouse.valid mouse
    | Mouse_move motion -> Location.valid motion.location
    | Key_down (key, _) | Key_up key -> Key.valid key
    | Scroll scroll -> Scroll.valid scroll
    | Mouse_enter | Mouse_leave | Focus | Blur -> true
  ;;
end
