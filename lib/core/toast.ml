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

module Placement = struct
  module Anchor = Gpuio_protocol.Toast_placement_wire.Anchor

  type t = Gpuio_protocol.Toast_placement_wire.t [@@deriving equal, sexp_of]

  let create ~anchor ?(top = 16.) ?(right = 16.) ?(bottom = 16.) ?(left = 16.) () =
    let t : t = { anchor; top; right; bottom; left } in
    if Gpuio_protocol.Toast_placement_wire.valid t
    then Ok t
    else
      Or_error.error_string
        "toast placement insets must be finite and in 0..16384 logical pixels"
  ;;
end

module Stack = struct
  module Motion = struct
    type t = Gpuio_protocol.Toast_motion_wire.t [@@deriving equal, sexp_of]

    let default_spring =
      Animation.Spring.create
        ~stiffness:400.
        ~damping:40.
        ~mass:1.
        ~epsilon:0.01
        ~max_duration:(Time_ns.Span.of_sec 2.)
        ()
      |> Or_error.ok_exn
    ;;

    let create
          ?(spring = default_spring)
          ?(enter = Time_ns.Span.of_ms 400.)
          ?(exit = Time_ns.Span.of_ms 200.)
          ?(offset = 96.)
          ()
      =
      let enter = Time_ns.Span.to_ms enter in
      let exit = Time_ns.Span.to_ms exit in
      if
        (not
           (List.for_all [ enter; exit ] ~f:(fun n ->
              Float.is_finite n && Float.(n >= 0. && n <= 60_000.))))
        || not (Float.is_finite offset && Float.(offset >= 0. && offset <= 16384.))
      then
        Or_error.error_string
          "toast motion needs durations in 0..60 seconds and finite offset in 0..16384"
      else
        Ok
          { Gpuio_protocol.Toast_motion_wire.spring =
              Animation.Expert.spring_to_wire spring
          ; enter_ms = Float.iround_up_exn enter |> Int64.of_int
          ; exit_ms = Float.iround_up_exn exit |> Int64.of_int
          ; offset
          }
    ;;

    let default = create () |> Or_error.ok_exn
  end

  module Layering = struct
    type t = Gpuio_protocol.Toast_layering_wire.t [@@deriving equal, sexp_of]

    let create ?(peek = 14.) ?(gap = 14.) ?(width_step = 0.05) ?(visible = 3) () =
      let t : t = { peek; gap; width_step; visible = Int64.of_int visible } in
      if Gpuio_protocol.Toast_layering_wire.valid t
      then Ok t
      else
        Or_error.error_string
          "toast layering needs finite peek/gap in 0..16384, width step in 0..0.1 and \
           1..8 layers"
    ;;

    let default = create () |> Or_error.ok_exn
  end

  type t =
    { label : string
    ; corner : Corner.t
    ; width : float
    ; max_visible : int
    ; placement : Placement.t option
    ; layering : Layering.t option
    ; motion : Motion.t option
    }
  [@@deriving equal, sexp_of]

  let create
        ?(label = "Notifications")
        ?corner
        ?placement
        ?layering
        ?motion
        ?(width = 360.)
        ?(max_visible = 3)
        ()
    =
    if Option.is_some corner && Option.is_some placement
    then Or_error.error_string "toast stack accepts either corner or placement, not both"
    else if
      valid_label label 4096
      && Float.is_finite width
      && Float.(width > 0. && width <= 1_000_000.)
      && max_visible >= 1
      && max_visible <= 8
    then
      Ok
        { label
        ; corner = Option.value corner ~default:Corner.Bottom_right
        ; width
        ; max_visible
        ; placement
        ; layering
        ; motion
        }
    else
      Or_error.error_string
        "toast stack needs a bounded label, positive finite width and 1..8 visible items"
  ;;

  let default = create () |> Or_error.ok_exn
end

module Expert = struct
  let motion (t : Stack.t) = t.motion
  let layering (t : Stack.t) = t.layering
  let placement (t : Stack.t) = t.placement

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
