open Core

(** Configuration for a highlight moving over laid-out text glyphs, distinct from
    [Loading.Kind.Shimmer]'s rectangular placeholder. Apply to ordinary text with
    [View.with_text_shimmer]; configuration alone does not animate a view. *)
module Spread : sig
  type t [@@deriving equal, sexp_of]

  (** Highlight half-width as a fraction of the text bounds, in [0.05, 1.].
      Nonfinite or out-of-range inputs are rejected, not silently clamped. *)
  val relative : float -> t Or_error.t

  (** Highlight half-width in logical pixels, in [1., 1_000_000.]. *)
  val pixels : float -> t Or_error.t

  (** Relative half-width 0.3. *)
  val default : t
end

module Direction : sig
  (** Physical sweep direction, independent of the text's writing direction. *)
  type t =
    | Left_to_right
    | Right_to_left
  [@@deriving equal, sexp_of]
end

module Repeat : sig
  type t =
    | Once
    | Loop
  [@@deriving equal, sexp_of]
end

module Appearance : sig
  type t [@@deriving equal, sexp_of]

  (** Application-owned theme, independent of the platform's light/dark setting.
      Colors resolve against [theme] now. Recreate on application theme changes.
      [dark] selects foreground (true) or background (false) as the default
      highlight target, and the corresponding native layer opacity. *)
  val create
    :  dark:bool
    -> foreground:Color.t
    -> background:Color.t
    -> ?theme:Theme.t
    -> unit
    -> t Or_error.t
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Defaults to a two-second, left-to-right loop with relative spread 0.3.
      [duration] is one sweep, in [1ms, 60s], rounded up to whole milliseconds.
      [animated=false] requests ordinary static text, as does reduced motion.

      An explicit [highlight] is resolved against [theme] now, preserving alpha;
      an undefined token is an error. Recreate on theme changes when using tokens.
      Omitting it requests the appearance/inherited-text default. [appearance]
      overrides the system light/dark palette for application-owned themes.
      No theme token
      is required when [highlight] is omitted. The effect owns no OCaml timer,
      task or callback. *)
  val create
    :  ?duration:Time_ns.Span.t
    -> ?spread:Spread.t
    -> ?direction:Direction.t
    -> ?repeat:Repeat.t
    -> ?animated:bool
    -> ?highlight:Color.t
    -> ?appearance:Appearance.t
    -> ?theme:Theme.t
    -> unit
    -> t Or_error.t

  val default : t
end

module Expert : sig
  val to_wire : Config.t -> Gpuio_protocol.Text_shimmer_wire.Config.t
  val of_wire : Gpuio_protocol.Text_shimmer_wire.Config.t -> Config.t Or_error.t
end
