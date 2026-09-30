open Core
module W = Gpuio_protocol.Text_shimmer_wire

module Spread = struct
  type t = W.Spread.t [@@deriving equal, sexp_of]

  let relative value =
    let t = W.Spread.Relative value in
    if W.Spread.valid t
    then Ok t
    else
      Or_error.error_string "text shimmer relative spread must be finite and in 0.05..1"
  ;;

  let pixels value =
    let t = W.Spread.Pixels value in
    if W.Spread.valid t
    then Ok t
    else
      Or_error.error_string "text shimmer pixel spread must be finite and in 1..1000000"
  ;;

  let default = W.Spread.Relative 0.3
end

module Direction = W.Direction
module Repeat = W.Repeat

module Config = struct
  type t = W.Config.t [@@deriving equal, sexp_of]

  let of_wire t =
    if W.Config.valid t then Ok t else Or_error.error_string "invalid text shimmer config"
  ;;

  let create
        ?(duration = Time_ns.Span.of_sec 2.)
        ?(spread = Spread.default)
        ?(direction = Direction.Left_to_right)
        ?(repeat = Repeat.Loop)
        ?(animated = true)
        ?highlight
        ?(theme = Theme.default)
        ()
    =
    let duration_ms = Time_ns.Span.to_ms duration in
    if Float.(duration_ms < 1. || duration_ms > 60_000.)
    then Or_error.error_string "text shimmer duration must be between 1ms and 60s"
    else
      let open Or_error.Let_syntax in
      let%bind highlight =
        match highlight with
        | None -> Ok None
        | Some color -> Theme.resolve theme color |> Or_error.map ~f:Option.some
      in
      of_wire
        { duration_ms = Float.iround_up_exn duration_ms
        ; spread
        ; direction
        ; repeat
        ; animated
        ; highlight
        }
  ;;

  let default = create () |> Or_error.ok_exn
end

module Expert = struct
  let to_wire t = t
  let of_wire = Config.of_wire
end
