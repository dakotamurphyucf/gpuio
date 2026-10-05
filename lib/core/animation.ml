open Core
module W = Gpuio_protocol.Wire.Animation
module P = Gpuio_protocol.Wire.Animation_program

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
    | Opacity_factor
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
    | Opacity_factor -> [ Opacity_factor ]
  ;;

  let valid ({ property; value } : W.Target.t) =
    Float.is_finite value
    &&
    match property with
    | Opacity | Opacity_factor -> Float.(value >= 0. && value <= 1.)
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
      List.exists fields ~f:(fun t -> W.Property.equal t.property Opacity)
      && List.exists fields ~f:(fun t -> W.Property.equal t.property Opacity_factor)
    then Or_error.error_string "opacity and opacity factor cannot share a target"
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
  module Linear_stop = struct
    type t =
      { input : float option
      ; output : float
      }
    [@@deriving equal, sexp_of]

    let create ?input ~output () =
      if
        Float.is_finite output
        && Option.for_all input ~f:(fun value ->
          Float.is_finite value && Float.(value >= 0. && value <= 1.))
      then Ok { input; output }
      else Or_error.error_string "linear stop requires a finite output and input in [0,1]"
    ;;
  end

  module Step_position = struct
    type t = W.Step_position.t =
      | Jump_start
      | Jump_end
      | Jump_none
      | Jump_both
    [@@deriving equal, sexp_of]
  end

  type t = W.Easing.t [@@deriving equal, sexp_of]

  let linear = W.Easing.Linear
  let ease = W.Easing.Ease
  let ease_in = W.Easing.Ease_in
  let ease_out = W.Easing.Ease_out
  let ease_in_out = W.Easing.Ease_in_out
  let ease_in_cubic = W.Easing.Cubic_bezier (1. /. 3., 0., 2. /. 3., 0.)
  let ease_out_cubic = W.Easing.Cubic_bezier (1. /. 3., 1., 2. /. 3., 1.)
  let ease_in_out_cubic = W.Easing.Ease_in_out_cubic

  let steps ~count ~position =
    let count = Int64.of_int count in
    if W.Easing.valid_steps ~count ~position
    then Ok (W.Easing.Steps (count, position))
    else
      Or_error.error_string
        "step count must be in [1,4294967295]; Jump_none requires at least 2"
  ;;

  let linear_stops stops =
    let length = List.length stops in
    if length < 2 || length > 256
    then Or_error.error_string "linear easing requires 2..256 stops"
    else (
      let stops = Array.of_list stops in
      let set_input index input =
        stops.(index) <- { (stops.(index)) with Linear_stop.input = Some input }
      in
      if Option.is_none stops.(0).input then set_input 0 0.;
      if Option.is_none stops.(length - 1).input then set_input (length - 1) 1.;
      let rec resolve anchor =
        if anchor = length - 1
        then Ok ()
        else (
          let next = ref (anchor + 1) in
          while Option.is_none stops.(!next).input do
            Int.incr next
          done;
          let from = Option.value_exn stops.(anchor).input in
          let until = Option.value_exn stops.(!next).input in
          if Float.(until < from)
          then Or_error.error_string "linear easing positions must be nondecreasing"
          else (
            for index = anchor + 1 to !next - 1 do
              set_input
                index
                (from
                 +. ((until -. from)
                     *. Float.of_int (index - anchor)
                     /. Float.of_int (!next - anchor)))
            done;
            resolve !next))
      in
      Or_error.map (resolve 0) ~f:(fun () ->
        W.Easing.Linear_stops
          (Array.to_list stops
           |> List.map ~f:(fun stop -> Option.value_exn stop.input, stop.output))))
  ;;

  let cubic_bezier ~x1 ~y1 ~x2 ~y2 =
    if
      List.for_all [ x1; x2 ] ~f:(fun x ->
        Float.is_finite x && Float.(x >= 0. && x <= 1.))
      && List.for_all [ y1; y2 ] ~f:Float.is_finite
    then Ok (W.Easing.Cubic_bezier (x1, y1, x2, y2))
    else Or_error.error_string "invalid cubic Bezier control points"
  ;;
end

module Spring = struct
  type t = W.Spring.t [@@deriving equal, sexp_of]

  let create
        ?(epsilon = 0.001)
        ?(max_duration = Time_ns.Span.of_sec 10.)
        ~stiffness
        ~damping
        ~mass
        ()
    =
    let bounded value ~lower ~upper =
      Float.is_finite value && Float.(value >= lower && value <= upper)
    in
    let milliseconds = Time_ns.Span.to_ms max_duration in
    if
      not
        (bounded stiffness ~lower:0.01 ~upper:10_000.
         && bounded damping ~lower:0. ~upper:1_000.
         && bounded mass ~lower:0.01 ~upper:1_000.
         && bounded epsilon ~lower:0.0001 ~upper:1.
         && Float.(milliseconds > 0. && milliseconds <= 60_000.))
    then Or_error.error_string "spring parameters or maximum duration are out of bounds"
    else
      Ok
        ({ stiffness
         ; damping
         ; mass
         ; epsilon
         ; max_duration_ms = Float.iround_up_exn milliseconds |> Int64.of_int
         }
         : W.Spring.t)
  ;;
end

module Iteration_count = struct
  type t = W.Iteration_count.t [@@deriving equal, sexp_of]

  let base = 4_294_967_296L
  let zero : t = { high = 0L; low = 0L }
  let one : t = { high = 0L; low = 1L }

  let of_int64 value =
    if Int64.(value < 0L)
    then Or_error.error_string "animation iteration count must be unsigned"
    else Ok W.Iteration_count.{ high = Int64.(value / base); low = Int64.(value % base) }
  ;;

  let of_int value = of_int64 (Int64.of_int value)

  let of_string text =
    let digits = String.lstrip text ~drop:(Char.equal '0') in
    if
      String.is_empty text
      || (not (String.for_all text ~f:Char.is_digit))
      || String.length digits > 20
      || (String.length digits = 20 && String.compare digits "18446744073709551615" > 0)
    then Or_error.error_string "animation iteration count must be decimal in [0, 2^64-1]"
    else
      Ok
        (String.fold digits ~init:zero ~f:(fun { W.Iteration_count.high; low } digit ->
           let digit = Int64.of_int (Char.to_int digit - Char.to_int '0') in
           let low = Int64.((low * 10L) + digit) in
           { W.Iteration_count.high = Int64.((high * 10L) + (low / base))
           ; low = Int64.(low % base)
           }))
  ;;

  let to_string t =
    let rec loop { W.Iteration_count.high; low } digits =
      if Int64.equal high 0L && Int64.equal low 0L
      then digits
      else (
        let combined = Int64.((high % 10L * base) + low) in
        let digit =
          Char.of_int_exn (Char.to_int '0' + Int64.to_int_exn Int64.(combined % 10L))
        in
        loop
          { W.Iteration_count.high = Int64.(high / 10L); low = Int64.(combined / 10L) }
          (digit :: digits))
    in
    if equal t zero then "0" else String.of_char_list (loop t [])
  ;;
end

module Direction = struct
  type t = W.Direction.t =
    | Normal
    | Reverse
    | Alternate
    | Alternate_reverse
  [@@deriving equal, sexp_of]
end

module Repeat = struct
  type t = W.Repeat.t =
    | Once
    | Loop
    | Alternate
    | Finite of Iteration_count.t * Direction.t
    | Infinite of Direction.t
  [@@deriving equal, sexp_of]

  let is_infinite = function
    | Loop | Alternate | Infinite _ -> true
    | Once | Finite _ -> false
  ;;
end

let milliseconds span =
  let value = Time_ns.Span.to_ms span in
  if Float.(value < 0. || value > 86_400_000.)
  then Or_error.error_string "animation time must be between zero and one day"
  else Ok (Float.iround_up_exn value |> Int64.of_int)
;;

let initial_delay_milliseconds span =
  let value = Time_ns.Span.to_ms span in
  let magnitude = Float.abs value in
  if Float.(magnitude > 86_400_000.)
  then Or_error.error_string "animation initial delay magnitude must be at most one day"
  else (
    let magnitude = Float.iround_up_exn magnitude |> Int64.of_int in
    Ok (if Float.(value < 0.) then Int64.neg magnitude else magnitude))
;;

module Timing = struct
  type t = P.Timing.t [@@deriving equal, sexp_of]

  let tween ?(easing = Easing.linear) duration =
    Or_error.map (milliseconds duration) ~f:(fun duration ->
      P.Timing.Tween (duration, easing))
  ;;

  let spring parameters = P.Timing.Spring parameters

  let maximum_duration = function
    | P.Timing.Tween (duration, _) -> duration
    | Spring parameters -> parameters.W.Spring.max_duration_ms
  ;;
end

module Stage = struct
  type t = P.Stage.t [@@deriving equal, sexp_of]

  let create ?(delay = Time_ns.Span.zero) ~timing ~target () =
    Or_error.map (milliseconds delay) ~f:(fun delay_ms ->
      ({ targets = target; timing; delay_ms } : P.Stage.t))
  ;;
end

module Clock = struct
  type t = P.Clock.t [@@deriving equal, sexp_of]

  let independent = P.Clock.Independent
  let application = P.Clock.Application

  let group name =
    if
      String.is_empty name
      || String.length name > 128
      || String.contains name '\000'
      || not (Stdlib.String.is_valid_utf_8 name)
    then
      Or_error.error_string "animation group name must be 1..128 UTF-8 bytes without NUL"
    else Ok (P.Clock.Group name)
  ;;
end

module Playback = struct
  type t = P.Playback.t =
    | Running
    | Paused
    | Cancelled
  [@@deriving equal, sexp_of]
end

module Run_id = struct
  type t = int64 [@@deriving equal, compare, sexp_of]

  let to_int64 t = t
end

module Program = struct
  module Stage_result = struct
    type t = P.Stage_result.t =
      | Played
      | Reduced_motion
    [@@deriving equal, sexp_of]
  end

  module Cancel_reason = struct
    type t = P.Cancel_reason.t =
      | Replaced
      | Removed
      | Window_closed
      | Requested
    [@@deriving equal, sexp_of]
  end

  module Observation = struct
    type t =
      | Stage_completed of int * Stage_result.t
      | Finished
      | Cancelled of Cancel_reason.t
    [@@deriving equal, sexp_of]
  end

  module Event = struct
    type t =
      { run_id : Run_id.t
      ; observations : Observation.t list
      }
    [@@deriving equal, sexp_of]
  end

  type t =
    { program : P.Program.t
    ; playback : Playback.t
    ; restart : int64
    }
  [@@deriving equal, sexp_of]

  let create
        ?initial
        ?(delay = Time_ns.Span.zero)
        ?(repeat = Repeat.Once)
        ?(clock = Clock.independent)
        stages
    =
    let open Or_error.Let_syntax in
    let%bind delay_ms = initial_delay_milliseconds delay in
    let matches a b =
      List.equal
        W.Property.equal
        (List.map a ~f:(fun item -> item.W.Target.property))
        (List.map b ~f:(fun item -> item.W.Target.property))
    in
    match stages with
    | [] -> Or_error.error_string "animation program must contain 1..32 stages"
    | first :: rest ->
      let period =
        List.fold stages ~init:0L ~f:(fun sum (stage : Stage.t) ->
          Int64.(sum + stage.delay_ms + Timing.maximum_duration stage.timing))
      in
      let shared = not (P.Clock.equal clock Independent) in
      if List.length stages > 32
      then Or_error.error_string "animation program must contain 1..32 stages"
      else if
        (not
           (List.for_all rest ~f:(fun (stage : Stage.t) ->
              matches first.targets stage.targets)))
        || Option.exists initial ~f:(fun values -> not (matches first.targets values))
      then Or_error.error_string "every animation stage must name the same properties"
      else if
        ((not (List.is_empty rest)) || not (Repeat.equal repeat Once))
        && Option.is_none initial
      then Or_error.error_string "sequences and repeats require initial values"
      else if Int64.(period > 86_400_000L)
      then Or_error.error_string "animation cycle exceeds one day"
      else if Repeat.is_infinite repeat && Int64.(period <= 0L)
      then Or_error.error_string "repeating animation needs a positive cycle duration"
      else if
        shared
        && ((not (Repeat.is_infinite repeat))
            || (not (Int64.equal delay_ms 0L))
            || List.exists stages ~f:(fun (stage : Stage.t) ->
              match stage.timing with
              | Spring _ -> true
              | Tween _ -> false))
      then
        Or_error.error_string
          "shared clocks require timed repetition without initial delay"
      else (
        let program : P.Program.t = { initial; stages; delay_ms; repeat; clock } in
        (* Reserve the maximum generation/restart encoding and playback tag. *)
        if P.Program.bin_size_t program + 19 > 16_384
        then Or_error.error_string "animation program exceeds 16384 encoded bytes"
        else Ok { program; playback = Running; restart = 0L })
  ;;

  let with_playback t playback = { t with playback }

  let with_repeat t repeat =
    let open Or_error.Let_syntax in
    let%map next =
      create
        ?initial:t.program.initial
        ~delay:(Time_ns.Span.of_ms (Int64.to_float t.program.delay_ms))
        ~repeat
        ~clock:t.program.clock
        t.program.stages
    in
    { next with playback = t.playback; restart = t.restart }
  ;;

  let with_initial_delay t delay =
    let open Or_error.Let_syntax in
    let%bind delay_ms = initial_delay_milliseconds delay in
    if (not (P.Clock.equal t.program.clock Independent)) && not (Int64.equal delay_ms 0L)
    then
      Or_error.error_string "shared clocks require timed repetition without initial delay"
    else (
      let program = { t.program with delay_ms } in
      if P.Program.bin_size_t program + 19 > 16_384
      then Or_error.error_string "animation program exceeds 16384 encoded bytes"
      else Ok { t with program })
  ;;

  let restart t =
    if Int64.equal t.restart Int64.max_value
    then Or_error.error_string "animation restart token exhausted"
    else Ok { t with restart = Int64.succ t.restart; playback = Running }
  ;;

  let reverse t =
    match t.program.initial with
    | None -> Or_error.error_string "reversing an animation requires initial values"
    | Some initial ->
      let final, reversed =
        List.fold
          t.program.stages
          ~init:(initial, [])
          ~f:(fun (previous, reversed) (stage : Stage.t) ->
            stage.targets, { stage with targets = previous } :: reversed)
      in
      Ok { t with program = { t.program with initial = Some final; stages = reversed } }
  ;;
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
    let%bind delay_ms = initial_delay_milliseconds delay in
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
      && (Option.is_none initial
          || (Repeat.is_infinite repeat && Int64.equal duration_ms 0L))
    then
      Or_error.error_string "repetition requires initial values and a positive duration"
    else Ok { targets = target; initial; duration_ms; delay_ms; easing; repeat }
  ;;
end

module Cancel_reason = struct
  type t = W.Cancel_reason.t =
    | Replaced
    | Removed
    | Window_closed
  [@@deriving equal, sexp_of]
end

module Outcome = struct
  type t = W.Outcome.t =
    | Finished
    | Cancelled of Cancel_reason.t
  [@@deriving equal, sexp_of]
end

module Event = struct
  type t =
    { run_id : Run_id.t
    ; outcome : Outcome.t
    }
  [@@deriving equal, sexp_of]
end

module Expert = struct
  let easing_to_wire (easing : Easing.t) = easing

  let program_event_of_wire signals =
    if not (P.Signal.valid_batch signals)
    then Or_error.error_string "invalid animation observation batch"
    else (
      let observations =
        List.map signals ~f:(fun (signal : P.Signal.t) ->
          match signal.observation with
          | Stage_completed (index, result) ->
            Program.Observation.Stage_completed (Int64.to_int_exn index, result)
          | Finished -> Finished
          | Cancelled reason -> Cancelled reason)
      in
      Ok ({ run_id = (List.hd_exn signals).generation; observations } : Program.Event.t))
  ;;

  let spring_to_wire (spring : Spring.t) = spring

  let program_to_wire (config : Program.t) ~generation =
    if Int64.(generation <= 0L)
    then Or_error.error_string "animation generation must be positive"
    else
      Ok
        ({ generation
         ; program = config.program
         ; playback = config.playback
         ; restart = config.restart
         }
         : P.Config.t)
  ;;

  let event_of_wire ({ generation; outcome } : W.Endpoint.t) =
    if Int64.(generation <= 0L)
    then Or_error.error_string "invalid animation run"
    else Ok ({ run_id = generation; outcome } : Event.t)
  ;;

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
