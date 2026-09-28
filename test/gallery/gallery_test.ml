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
