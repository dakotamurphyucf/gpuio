open Core

module Open_state = struct
  type t =
    | Managed of { initially_open : bool }
    | Controlled of bool
  [@@deriving equal, sexp_of]
end

module Config = struct
  type t =
    { label : string
    ; width : float
    ; placement : Placement.t
    ; open_state : Open_state.t
    ; disabled : bool
    ; hoverable : bool
    ; show_delay : Time_ns.Span.t
    ; hide_delay : Time_ns.Span.t
    ; skip_delay : Time_ns.Span.t
    }
  [@@deriving equal, sexp_of]

  let default_placement =
    Placement.create ~side:Top ~align:Center ~offset:6. () |> Or_error.ok_exn
  ;;

  let create
        ~label
        ?(width = 320.)
        ?(placement = default_placement)
        ?(open_state = Open_state.Managed { initially_open = false })
        ?(disabled = false)
        ?(hoverable = true)
        ?(show_delay = Time_ns.Span.of_ms 250.)
        ?(hide_delay = Time_ns.Span.of_ms 80.)
        ?(skip_delay = Time_ns.Span.of_ms 300.)
        ()
    =
    let%bind.Or_error _ = Overlay.Config.create ~label ~width () in
    if
      List.for_all [ show_delay; hide_delay; skip_delay ] ~f:(fun delay ->
        Time_ns.Span.(delay >= zero && delay <= of_sec 60.))
    then
      Ok
        { label
        ; width
        ; placement
        ; open_state
        ; disabled
        ; hoverable
        ; show_delay
        ; hide_delay
        ; skip_delay
        }
    else Or_error.error_string "tooltip delays must be in 0..60 seconds"
  ;;
end

module Expert = struct
  let placement (t : Config.t) = t.placement
  let is_disabled (t : Config.t) = t.disabled

  let to_wire (t : Config.t) : Gpuio_protocol.Wire.Tooltip.t =
    { label = t.label
    ; width = t.width
    ; open_state =
        (match t.open_state with
         | Managed { initially_open } -> Managed initially_open
         | Controlled open_ -> Controlled open_)
    ; disabled = t.disabled
    ; hoverable = t.hoverable
    ; show_delay_ns = Time_ns.Span.to_int63_ns t.show_delay |> Int63.to_int64
    ; hide_delay_ns = Time_ns.Span.to_int63_ns t.hide_delay |> Int63.to_int64
    ; skip_delay_ns = Time_ns.Span.to_int63_ns t.skip_delay |> Int63.to_int64
    }
  ;;
end
