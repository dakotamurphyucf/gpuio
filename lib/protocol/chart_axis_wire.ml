open Core

let within n a b = Float.is_finite n && Float.(n >= a && n <= b)
let color n = Int64.(n >= 0L && n <= 0xffff_ffffL)

module Label_side = struct
  type t =
    | Auto
    | Before
    | After
  [@@deriving bin_io, equal, sexp_of]
end

module Label_align = struct
  type t =
    | Auto
    | Left
    | Center
    | Right
  [@@deriving bin_io, equal, sexp_of]
end

module Tick_position = struct
  type t =
    | Value of float
    | Category of int64
    | Fraction of float
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Value n -> within n (-1e100) 1e100
    | Category n -> Int64.(n > 0L)
    | Fraction n -> within n 0. 1.
  ;;
end

module Tick = struct
  type t =
    { position : Tick_position.t
    ; text : string
    ; color : int64 option
    ; font_size : float option
    ; align : Label_align.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Tick_position.valid t.position
    && String.length t.text <= 256
    && Stdlib.String.is_valid_utf_8 t.text
    && (not
          (String.exists t.text ~f:(fun c -> Char.to_int c < 32 || Char.equal c '\127')))
    && Option.for_all t.color ~f:color
    && Option.for_all t.font_size ~f:(fun n -> within n 8. 32.)
  ;;
end

type t =
  { line : bool
  ; labels : bool
  ; position : float option
  ; ticks : Tick.t list option
  ; tick_count : int64 option
  ; label_side : Label_side.t
  ; label_align : Label_align.t
  ; label_gap : float option
  ; label_width : float option
  ; font_size : float
  ; line_width : float
  ; line_color : int64 option
  ; label_color : int64 option
  }
[@@deriving bin_io, equal, sexp_of]

let valid t =
  Option.for_all t.position ~f:(fun n -> within n 0. 1.)
  && Option.for_all t.ticks ~f:(fun xs ->
    List.length xs <= 64 && List.for_all xs ~f:Tick.valid)
  && Option.for_all t.tick_count ~f:(fun n -> Int64.(n >= 2L && n <= 64L))
  && Option.for_all t.label_gap ~f:(fun n -> within n 0. 64.)
  && Option.for_all t.label_width ~f:(fun n -> within n 8. 256.)
  && within t.font_size 8. 32.
  && within t.line_width 0.5 8.
  && Option.for_all t.line_color ~f:color
  && Option.for_all t.label_color ~f:color
;;
