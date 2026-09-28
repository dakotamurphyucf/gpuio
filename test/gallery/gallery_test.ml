open Core
module State = Gpuio_gallery_model.Numeric_state
module Rating = Gpuio.Rating

let%expect_test "read-only fences delayed requests and relative bursts saturate" =
  let apply state actions = List.fold actions ~init:state ~f:State.apply in
  let show state =
    print_s
      [%sexp
        (State.is_read_only state : bool)
      , (Rating.Config.value (State.rating state) : int)]
  in
  let locked =
    apply
      State.initial
      [ Toggle_read_only; Rate Rating.Request.increase; Rate Rating.Request.clear ]
  in
  show locked;
  let unlocked =
    apply
      locked
      [ Toggle_read_only
      ; Rate Rating.Request.increase
      ; Rate Rating.Request.increase
      ; Rate Rating.Request.increase
      ]
  in
  show unlocked;
  let cleared =
    State.apply unlocked (Rate (Rating.Request.toggle 5 |> Or_error.ok_exn))
  in
  show cleared;
  [%expect
    {|
    (true 3)
    (false 5)
    (false 0)
  |}]
;;

let%expect_test "commands use current availability; old dismissals preserve a replacement"
  =
  let module F = Gpuio_gallery_model.Feedback_state in
  let apply state actions = List.fold actions ~init:state ~f:F.apply in
  let show state =
    print_s
      [%sexp
        (F.stage state : F.Stage.t)
      , (F.is_enabled state : bool)
      , (F.notification state : int option)]
  in
  let disabled = apply F.initial [ Toggle_enabled; Advance ] in
  show disabled;
  let working = apply disabled [ Toggle_enabled; Advance; Notify ] in
  show working;
  let replacement = apply working [ Notify; Dismiss 1; Advance; Advance ] in
  show replacement;
  show (apply replacement [ Dismiss 2; Dismiss 2; Advance ]);
  show (apply replacement [ Leave; Notify; Dismiss 2 ]);
  [%expect
    {|
    (Idle false ())
    (Working true (1))
    (Complete true (2))
    (Idle true ())
    (Complete true (3))
  |}]
;;
