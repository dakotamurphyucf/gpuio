open Core

module Kind : sig
  type t =
    | Skeleton
    | Shimmer
    | Spinner
  [@@deriving equal, sexp_of]
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Native indeterminate loading, without an OCaml timer or callback.
      [label] is nonblank UTF-8 without NUL, at most 4096 bytes. [animated] defaults
      to true; false renders a recognizable static indicator. Reduced motion
      also uses the static presentation. [period] defaults to 1200ms, must be
      100ms..60s and rounds up to a whole millisecond.

      Skeleton pulses a solid placeholder; Shimmer sweeps a highlight across one;
      Spinner cycles twelve radial strokes. Styles set size and foreground color.
      Defaults are 160x16 logical pixels (placeholder) or 20x20 (spinner).
      Visibility, unmount and window lifetime bound native frame requests.
      Inert subtrees retain a static indicator; hidden subtrees do not paint.
      No completion or progress fraction is invented. *)
  val create
    :  kind:Kind.t
    -> label:string
    -> ?animated:bool
    -> ?period:Time_ns.Span.t
    -> unit
    -> t Or_error.t
end

module Expert : sig
  val to_wire : Config.t -> Gpuio_protocol.Loading_wire.Config.t
end
