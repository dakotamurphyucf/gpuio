open Core
module Wire = Gpuio_protocol.Wire

module Timeout = struct
  type t = Time_ns.Span.t option [@@deriving equal, sexp_of]

  let persistent = None

  let after duration =
    if Time_ns.Span.(duration > zero && duration <= of_sec 86400.)
    then Ok (Some duration)
    else Or_error.error_string "toast timeout must be positive and at most 24 hours"
  ;;
end

module Politeness = struct
  type t =
    | Polite
    | Assertive
  [@@deriving equal, sexp_of]
end

module Dismissal = struct
  type t =
    | Timeout
    | Close_button
    | Escape
    | Overflow
  [@@deriving equal, sexp_of]
end

let valid_label value limit =
  String.length value <= limit
  && Stdlib.String.is_valid_utf_8 value
  && (not (String.contains value '\000'))
  && not (String.is_empty (String.strip value))
;;

module Config = struct
  type t =
    { label : string
    ; timeout : Timeout.t
    ; politeness : Politeness.t
    ; close_label : string
    }
  [@@deriving equal, sexp_of]

  let create
        ~label
        ?(timeout = Some (Time_ns.Span.of_sec 5.))
        ?(politeness = Politeness.Polite)
        ?(close_label = "Dismiss notification")
        ()
    =
    if valid_label label 4096 && valid_label close_label 256
    then Ok { label; timeout; politeness; close_label }
    else Or_error.error_string "toast labels must be bounded, nonblank UTF-8 without NUL"
  ;;
end

module Corner = struct
  type t =
    | Top_left
    | Top_right
    | Bottom_left
    | Bottom_right
  [@@deriving equal, sexp_of]
end

module Stack = struct
  type t =
    { label : string
    ; corner : Corner.t
    ; width : float
    ; max_visible : int
    }
  [@@deriving equal, sexp_of]

  let create
        ?(label = "Notifications")
        ?(corner = Corner.Bottom_right)
        ?(width = 360.)
        ?(max_visible = 3)
        ()
    =
    if
      valid_label label 4096
      && Float.is_finite width
      && Float.(width > 0. && width <= 1_000_000.)
      && max_visible >= 1
      && max_visible <= 8
    then Ok { label; corner; width; max_visible }
    else
      Or_error.error_string
        "toast stack needs a bounded label, positive finite width and 1..8 visible items"
  ;;

  let default = create () |> Or_error.ok_exn
end

module Expert = struct
  let to_wire (t : Config.t) : Wire.Toast.t =
    { label = t.label
    ; close_label = t.close_label
    ; timeout_ns =
        Option.map t.timeout ~f:(fun span ->
          Time_ns.Span.to_int63_ns span |> Int63.to_int64)
    ; politeness =
        (match t.politeness with
         | Polite -> Polite
         | Assertive -> Assertive)
    }
  ;;

  let stack_to_wire (t : Stack.t) : Wire.Toast_stack.t =
    { label = t.label
    ; width = t.width
    ; max_visible = Int64.of_int t.max_visible
    ; corner =
        (match t.corner with
         | Top_left -> Top_left
         | Top_right -> Top_right
         | Bottom_left -> Bottom_left
         | Bottom_right -> Bottom_right)
    }
  ;;

  let dismissal : Wire.Toast_dismissal.t -> Dismissal.t = function
    | Timeout -> Timeout
    | Close_button -> Close_button
    | Escape -> Escape
    | Overflow -> Overflow
  ;;
end
