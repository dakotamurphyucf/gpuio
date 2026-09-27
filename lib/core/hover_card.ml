open Core
module Open_state = Tooltip.Open_state

module Config = struct
  type t = Tooltip.Config.t [@@deriving equal, sexp_of]

  let create
        ~label
        ?width
        ?placement
        ?open_state
        ?disabled
        ?(show_delay = Time_ns.Span.of_ms 600.)
        ?(hide_delay = Time_ns.Span.of_ms 300.)
        ()
    =
    Tooltip.Config.create
      ~label
      ?width
      ?placement
      ?open_state
      ?disabled
      ~hoverable:true
      ~show_delay
      ~hide_delay
      ~skip_delay:Time_ns.Span.zero
      ()
  ;;
end

module Expert = struct
  let tooltip (t : Config.t) = t
end
