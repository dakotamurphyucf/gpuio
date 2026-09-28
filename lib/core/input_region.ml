open Core
module Wire = Gpuio_protocol.Input_wire
module Button = Pointer.Button
module Modifiers = Pointer.Modifiers
module Kind = Wire.Kind
module Phase = Wire.Phase
module Policy = Wire.Policy
module Focus = Wire.Focus

module Subscription = struct
  type t = Wire.Subscription.t [@@deriving equal, sexp_of]

  let create kind ?(phase = Phase.Bubble) ?(policy = Policy.Observe) () =
    let t = Wire.Subscription.{ kind; phase; policy } in
    if Wire.Subscription.valid t
    then Ok t
    else Or_error.error_string "derived input notifications require Bubble/Observe"
  ;;

  let kind (t : t) = t.kind
end

module Config = struct
  type t = Wire.Config.t [@@deriving equal, sexp_of]

  let create ~label ?(disabled = false) ?(focus = Focus.None) subscriptions =
    let subscriptions =
      List.sort subscriptions ~compare:(fun a b ->
        Kind.compare (Subscription.kind a) (Subscription.kind b))
    in
    let t = Wire.Config.{ label; disabled; focus; subscriptions } in
    if Wire.Config.valid t
    then Ok t
    else
      Or_error.error_string
        "input region requires a bounded label, unique subscriptions and focusable \
         Focus/Blur target"
  ;;

  let subscriptions (t : t) = t.subscriptions
  let is_disabled (t : t) = t.disabled
end

let button_of_wire : Wire.Button.t -> Button.t = function
  | Left -> Left
  | Right -> Right
  | Middle -> Middle
  | Back -> Back
  | Forward -> Forward
;;

let modifiers_of_wire (t : Wire.Modifiers.t) =
  Modifiers.
    { shift = t.shift
    ; control = t.control
    ; alt = t.alt
    ; command = t.command
    ; function_ = t.function_
    }
;;

module Position = struct
  type t =
    { x : float
    ; y : float
    }
  [@@deriving equal, sexp_of]

  let of_wire (t : Wire.Position.t) = { x = t.x; y = t.y }
end

module Location = struct
  type t =
    { window : Position.t
    ; local : Position.t
    ; modifiers : Modifiers.t
    }
  [@@deriving equal, sexp_of]

  let of_wire (t : Wire.Location.t) =
    { window = Position.of_wire t.window
    ; local = Position.of_wire t.local
    ; modifiers = modifiers_of_wire t.modifiers
    }
  ;;
end

module Motion = struct
  type t =
    { location : Location.t
    ; pressed_button : Button.t option
    }
  [@@deriving equal, sexp_of]

  let of_wire (t : Wire.Motion.t) =
    { location = Location.of_wire t.location
    ; pressed_button = Option.map t.pressed_button ~f:button_of_wire
    }
  ;;
end

module Key = struct
  type t =
    { key : string
    ; character : string option
    ; modifiers : Modifiers.t
    }
  [@@deriving equal, sexp_of]

  let of_wire (t : Wire.Key.t) =
    { key = t.key; character = t.character; modifiers = modifiers_of_wire t.modifiers }
  ;;
end

module Delta = struct
  type t =
    | Pixels of Position.t
    | Lines of Position.t
  [@@deriving equal, sexp_of]

  let of_wire : Wire.Delta.t -> t = function
    | Pixels p -> Pixels (Position.of_wire p)
    | Lines p -> Lines (Position.of_wire p)
  ;;
end

module Touch_phase = Wire.Touch_phase

module Scroll = struct
  type t =
    { location : Location.t
    ; delta : Delta.t
    ; phase : Touch_phase.t
    }
  [@@deriving equal, sexp_of]

  let of_wire (t : Wire.Scroll.t) =
    { location = Location.of_wire t.location
    ; delta = Delta.of_wire t.delta
    ; phase = t.phase
    }
  ;;
end

module Mouse = struct
  type t =
    { location : Location.t
    ; button : Button.t
    ; click_count : int
    }
  [@@deriving equal, sexp_of]

  let of_wire (t : Wire.Mouse.t) =
    { location = Location.of_wire t.location
    ; button = button_of_wire t.button
    ; click_count = Int64.to_int_exn t.click_count
    }
  ;;
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
  [@@deriving equal, sexp_of]

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
end

module Expert = struct
  let to_wire (t : Config.t) = t

  let event_of_wire t =
    if not (Wire.Event.valid t)
    then Or_error.error_string "invalid native input observation"
    else
      Ok
        (match (t : Wire.Event.t) with
         | Click t -> Event.Click (Mouse.of_wire t)
         | Auxiliary_click t -> Auxiliary_click (Mouse.of_wire t)
         | Mouse_down t -> Mouse_down (Mouse.of_wire t)
         | Mouse_up t -> Mouse_up (Mouse.of_wire t)
         | Mouse_move t -> Mouse_move (Motion.of_wire t)
         | Mouse_enter -> Mouse_enter
         | Mouse_leave -> Mouse_leave
         | Mouse_down_outside t -> Mouse_down_outside (Mouse.of_wire t)
         | Key_down (t, repeat) -> Key_down (Key.of_wire t, repeat)
         | Key_up t -> Key_up (Key.of_wire t)
         | Focus -> Focus
         | Blur -> Blur
         | Scroll t -> Scroll (Scroll.of_wire t))
  ;;
end
