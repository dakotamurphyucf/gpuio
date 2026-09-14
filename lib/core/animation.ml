open Core
module W = Gpuio_protocol.Wire.Animation

module Property = struct
  type t =
    | Width
    | Height
    | Top
    | Right
    | Bottom
    | Left
    | Opacity
    | Radius
    | Top_left_radius
    | Top_right_radius
    | Bottom_left_radius
    | Bottom_right_radius
  [@@deriving equal, sexp_of]
end

module Target = struct
  type t = W.Target.t list [@@deriving equal, sexp_of]

  let expand : Property.t -> W.Property.t list = function
    | Width -> [ Width ]
    | Height -> [ Height ]
    | Top -> [ Top ]
    | Right -> [ Right ]
    | Bottom -> [ Bottom ]
    | Left -> [ Left ]
    | Opacity -> [ Opacity ]
    | Radius ->
      [ Top_left_radius; Top_right_radius; Bottom_left_radius; Bottom_right_radius ]
    | Top_left_radius -> [ Top_left_radius ]
    | Top_right_radius -> [ Top_right_radius ]
    | Bottom_left_radius -> [ Bottom_left_radius ]
    | Bottom_right_radius -> [ Bottom_right_radius ]
  ;;

  let valid ({ property; value } : W.Target.t) =
    Float.is_finite value
    &&
    match property with
    | Opacity -> Float.(value >= 0. && value <= 1.)
    | Top | Right | Bottom | Left -> Float.(value >= -1_000_000. && value <= 1_000_000.)
    | Width
    | Height
    | Top_left_radius
    | Top_right_radius
    | Bottom_left_radius
    | Bottom_right_radius -> Float.(value >= 0. && value <= 1_000_000.)
  ;;

  let create properties =
    let fields =
      List.concat_map properties ~f:(fun (property, value) ->
        List.map (expand property) ~f:(fun property -> ({ property; value } : W.Target.t)))
      |> List.sort ~compare:(fun a b -> W.Property.compare a.property b.property)
    in
    if
      List.is_empty fields
      || List.length fields > 11
      || not (List.for_all fields ~f:valid)
    then
      Or_error.error_string
        "animation targets must be nonempty, finite and within property bounds"
    else if
      Option.is_some
        (List.find_consecutive_duplicate fields ~equal:(fun a b ->
           W.Property.equal a.property b.property))
    then
      Or_error.error_string "animation properties must be unique after expanding radius"
    else Ok fields
  ;;
end

module Easing = struct
  type t = W.Easing.t [@@deriving equal, sexp_of]

  let linear = W.Easing.Linear
  let ease = W.Easing.Ease
  let ease_in = W.Easing.Ease_in
  let ease_out = W.Easing.Ease_out
  let ease_in_out = W.Easing.Ease_in_out

  let cubic_bezier ~x1 ~y1 ~x2 ~y2 =
    if
      List.for_all [ x1; x2 ] ~f:(fun x ->
        Float.is_finite x && Float.(x >= 0. && x <= 1.))
      && List.for_all [ y1; y2 ] ~f:Float.is_finite
    then Ok (W.Easing.Cubic_bezier (x1, y1, x2, y2))
    else Or_error.error_string "invalid cubic Bezier control points"
  ;;
end

module Repeat = struct
  type t = W.Repeat.t =
    | Once
    | Loop
    | Alternate
  [@@deriving equal, sexp_of]
end

module Preference = struct
  type t = W.Preference.t =
    | System
    | Reduce
    | Full
  [@@deriving equal, sexp_of]
end

module Config = struct
  type t =
    { targets : Target.t
    ; initial : Target.t option
    ; duration_ms : int64
    ; delay_ms : int64
    ; easing : Easing.t
    ; repeat : Repeat.t
    }
  [@@deriving equal, sexp_of]

  let milliseconds span =
    let value = Time_ns.Span.to_ms span in
    if Float.(value < 0. || value > 86_400_000.)
    then Or_error.error_string "animation time must be between zero and one day"
    else Ok (Float.iround_up_exn value |> Int64.of_int)
  ;;

  let create
        ?initial
        ?(duration = Time_ns.Span.of_ms 200.)
        ?(delay = Time_ns.Span.zero)
        ?(easing = Easing.linear)
        ?(repeat = Repeat.Once)
        ~target
        ()
    =
    let open Or_error.Let_syntax in
    let%bind duration_ms = milliseconds duration in
    let%bind delay_ms = milliseconds delay in
    if
      Option.exists initial ~f:(fun initial ->
        not
          (List.equal
             W.Property.equal
             (List.map initial ~f:(fun field -> field.W.Target.property))
             (List.map target ~f:(fun field -> field.W.Target.property))))
    then Or_error.error_string "initial and target properties must match"
    else if
      (not (Repeat.equal repeat Once))
      && (Option.is_none initial || Int64.equal duration_ms 0L)
    then
      Or_error.error_string "repetition requires initial values and a positive duration"
    else Ok { targets = target; initial; duration_ms; delay_ms; easing; repeat }
  ;;
end

module Expert = struct
  let to_wire
        ({ targets; initial; duration_ms; delay_ms; easing; repeat } : Config.t)
        ~generation
    =
    if Int64.(generation <= 0L)
    then Or_error.error_string "animation generation must be positive"
    else
      Ok
        ({ generation; targets; initial; duration_ms; delay_ms; easing; repeat }
         : W.Config.t)
  ;;
end
