open Core

type t =
  { x : Chart_axis_wire.Tick_position.t list option
  ; y : Chart_axis_wire.Tick_position.t list option
  ; dashes : float list
  ; width : float
  ; color : int64 option
  }
[@@deriving bin_io, equal, sexp_of]

let positions xs =
  List.length xs <= 64 && List.for_all xs ~f:Chart_axis_wire.Tick_position.valid
;;

let valid t =
  Option.for_all t.x ~f:positions
  && Option.for_all t.y ~f:positions
  && List.length t.dashes <= 16
  && List.for_all t.dashes ~f:(fun n -> Chart_axis_wire.within n 0.5 128.)
  && Chart_axis_wire.within t.width 0.5 8.
  && Option.for_all t.color ~f:Chart_axis_wire.color
;;
