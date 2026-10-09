open Core
open Gpuio
module V = Gpuio_bonsai.View

type t =
  | Default
  | Quarter_turn
  | Mirror
  | Stretch
  | Offset
  | Combined
  | Collapsed
[@@deriving equal]

let label = function
  | Default -> "Default icon"
  | Quarter_turn -> "Rotate icon"
  | Mirror -> "Mirror icon"
  | Stretch -> "Stretch icon"
  | Offset -> "Offset icon"
  | Combined -> "Combine transforms"
  | Collapsed -> "Collapse icon"
;;

let transform preset =
  let validated value = Some (Or_error.ok_exn value) in
  match preset with
  | Default -> None
  | Quarter_turn -> validated (Icon.Transform.rotate_degrees 90.)
  | Mirror -> validated (Icon.Transform.create ~scale_x:(-1.) ())
  | Stretch -> validated (Icon.Transform.create ~scale_x:1.4 ~scale_y:0.7 ())
  | Offset -> validated (Icon.Transform.create ~translate_x:3. ~translate_y:(-2.) ())
  | Combined ->
    validated
      (Icon.Transform.create
         ~scale_x:(-0.8)
         ~scale_y:0.8
         ~rotation_degrees:30.
         ~translate_x:2.
         ())
  | Collapsed -> validated (Icon.Transform.create ~scale_x:0. ())
;;

let controls p ~selected ~on_select =
  V.row
    ~style:(Style.create_exn [ Gap (Length.px_exn 6.); Wrap Wrap ])
    (List.map
       [ Default; Quarter_turn; Mirror; Stretch; Offset; Combined; Collapsed ]
       ~f:(fun preset ->
         Palette.button
           p
           ~selected:(equal selected preset)
           (label preset)
           (on_select preset)))
;;
