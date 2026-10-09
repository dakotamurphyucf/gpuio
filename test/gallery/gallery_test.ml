open Core
module State = Gpuio_gallery_model.Numeric_state
module Rating = Gpuio.Rating

let%expect_test "rating preview preserves appearance and applies current range and policy"
  =
  let apply state actions = List.fold actions ~init:state ~f:State.apply in
  let toggle n = State.Action.Rate (Rating.Request.toggle n |> Or_error.ok_exn) in
  let value state = Rating.Config.value (State.rating state) in
  let customized =
    apply
      State.initial
      [ Toggle_rating_colors
      ; Cycle_rating_size
      ; Cycle_rating_maximum
      ; Toggle_rating_step_down
      ; toggle 3
      ]
  in
  assert (value customized = 2);
  let locked =
    apply customized [ Toggle_read_only; toggle 3; Rate Rating.Request.increase ]
  in
  assert (value locked = 2);
  assert (State.custom_rating_colors locked && State.step_down_rating locked);
  assert (Float.equal (Rating.Config.star_size (State.rating locked)) 36.);
  assert (Rating.Config.maximum (State.rating locked) = 10);
  let resized = apply locked [ Toggle_read_only; Cycle_rating_maximum; toggle 3 ] in
  assert (value resized = 1 && Rating.Config.maximum (State.rating resized) = 1);
  let disabled = apply resized [ Toggle_rating_disabled; Rate Rating.Request.clear ] in
  assert (value disabled = 1);
  let enabled = apply disabled [ Toggle_rating_disabled; toggle 1 ] in
  assert (value enabled = 0);
  print_endline
    "appearance/size survive read-only; stale range and disabled requests rejected; \
     step-down uses current state";
  [%expect
    {| appearance/size survive read-only; stale range and disabled requests rejected; step-down uses current state |}]
;;

let%expect_test "extension commands serialize properties and fence obsolete generations" =
  let module S = Gpuio_gallery_model.Extension_state in
  let apply state actions = List.fold actions ~init:state ~f:S.apply in
  let show state =
    print_s
      [%sexp
        (S.value state : int)
      , (S.step state : int)
      , (S.generation state : int64)
      , (S.command state : (int64 * int) option)
      , (S.disabled state : bool)
      , (S.visible state : bool)]
  in
  let pending =
    apply
      S.initial
      [ Send_command
      ; Set_property
      ; Toggle_step
      ; Send_command
      ; Toggle_disabled
      ; Observe (1L, Data 99)
      ; Observe (1L, Command_completed 99L)
      ]
  in
  show pending;
  let completed = S.apply pending (Observe (1L, Command_completed 1L)) in
  show completed;
  let reset =
    apply
      completed
      [ Reset; Observe (1L, Data 99); Toggle_disabled; Observe (2L, Data 8) ]
  in
  show reset;
  let departed =
    apply
      reset
      [ Toggle_visible
      ; Observe (2L, Data 99)
      ; Send_command
      ; Depart
      ; Observe (2L, Command_completed 1L)
      ; Observe (2L, Data 100)
      ]
  in
  show departed;
  [%expect
    {|
    (7 1 1 ((1 42)) true true)
    (42 1 1 () true true)
    (8 1 2 () false true)
    (8 1 3 () false false)
    |}]
;;

let%expect_test "canvas moves preserve other shapes and reject invalid whole scenes" =
  let module Study = Gpuio_gallery_model.Canvas_study in
  let module G = Gpuio.Canvas_geometry in
  let ok = Or_error.ok_exn in
  let orbit = Study.items Study.initial |> List.hd_exn |> Study.Item.id in
  let moved =
    Study.move Study.initial orbit (G.Transform.translate ~x:180. ~y:144. |> ok) |> ok
  in
  let positions model =
    List.map (Study.items model) ~f:(fun item ->
      let point = Study.Item.position item in
      Study.Item.name item, (G.Point.x point, G.Point.y point))
  in
  print_s [%sexp (positions moved : (string * (float * float)) list)];
  print_s
    [%sexp
      (Or_error.is_error
         (Study.move
            moved
            (Gpuio.Canvas_scene.Item_id.of_int64 999L |> ok)
            G.Transform.identity)
       : bool)
    , (Or_error.is_error
         (Study.move moved orbit (G.Transform.translate ~x:1_000_000. ~y:0. |> ok))
       : bool)
    , (Or_error.is_error (Study.move moved orbit (G.Transform.rotate ~radians:0.5 |> ok))
       : bool)];
  print_s [%sexp (positions Study.initial : (string * (float * float)) list)];
  [%expect
    {|
    ((Orbit (180 144)) (Prism (290 100)) (Tile (470 160)))
    (true true true)
    ((Orbit (110 130)) (Prism (290 100)) (Tile (470 160)))
    |}]
;;

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

let%expect_test "unavailable children preserve selection under delayed and bulk requests" =
  let module S = Gpuio_gallery_model.Selection_state in
  let apply state actions = List.fold actions ~init:state ~f:S.apply in
  let show state =
    print_s
      [%sexp
        (List.filter S.Format.all ~f:(S.selected state) : S.Format.t list)
      , (List.filter S.Format.all ~f:(S.format_enabled state) : S.Format.t list)
      , (List.filter S.Format.all ~f:(S.format_loading state) : S.Format.t list)
      , (S.master state : Gpuio.Check_state.t)]
  in
  let locked = apply S.initial [ Toggle_italic_enabled; Toggle_bold_loading ] in
  show (apply locked [ Toggle_format Bold; Toggle_format Italic ]);
  let partial = S.apply locked Toggle_all in
  show partial;
  show (S.apply partial Toggle_all);
  let paused = S.apply partial Toggle_enabled in
  show (apply paused [ Toggle_format Monospace; Toggle_all ]);
  let restored =
    apply paused [ Toggle_enabled; Toggle_italic_enabled; Toggle_bold_loading ]
  in
  show (S.apply restored Toggle_all);
  show (S.apply (S.apply restored Toggle_all) Toggle_all);
  [%expect
    {|
    ((Bold) (Bold Monospace) (Bold) Indeterminate)
    ((Bold Monospace) (Bold Monospace) (Bold) Indeterminate)
    ((Bold Monospace) (Bold Monospace) (Bold) Indeterminate)
    ((Bold Monospace) () (Bold) Indeterminate)
    ((Bold Italic Monospace) (Bold Italic Monospace) () Checked)
    (() (Bold Italic Monospace) () Unchecked)
    |}]
;;

let%expect_test "selection intents use current state and keep one alignment" =
  let module S = Gpuio_gallery_model.Selection_state in
  let apply state actions = List.fold actions ~init:state ~f:S.apply in
  let show state =
    print_s
      [%sexp
        (S.enabled state : bool)
      , (S.alignment state : S.Alignment.t)
      , (List.filter S.Format.all ~f:(S.selected state) : S.Format.t list)
      , (S.master state : Gpuio.Check_state.t)]
  in
  show S.initial;
  let rapid = apply S.initial [ Toggle_format Bold; Toggle_format Bold ] in
  show rapid;
  let all = S.apply rapid Toggle_all in
  show all;
  let none = S.apply all Toggle_all in
  show none;
  let exclusive = apply none [ Align Center; Align Right; Align Right ] in
  show exclusive;
  let locked =
    apply exclusive [ Toggle_enabled; Toggle_format Italic; Align Left; Toggle_all ]
  in
  show locked;
  let reopened = apply locked [ Toggle_enabled; Toggle_format Monospace ] in
  show reopened;
  [%expect
    {|
    (true Left (Bold) Indeterminate)
    (true Left (Bold) Indeterminate)
    (true Left (Bold Italic Monospace) Checked)
    (true Left () Unchecked)
    (true Right () Unchecked)
    (false Right () Unchecked)
    (true Right (Monospace) Indeterminate)
  |}]
;;

let%expect_test "restoring sample cards fences an exit callback already in flight" =
  let module S = Gpuio_gallery_model.Feedback_state.Samples in
  let old = List.hd_exn (S.items S.initial) in
  let restored = S.apply S.initial Show in
  let show model =
    print_s [%sexp (List.map (S.items model) ~f:S.Item.key : string list)]
  in
  show restored;
  let after_old_exit = S.apply restored (Dismiss old) in
  show after_old_exit;
  let current = List.hd_exn (S.items after_old_exit) in
  let dismissed = S.apply after_old_exit (Dismiss current) in
  show (S.apply dismissed (Dismiss current));
  show (S.apply dismissed Show);
  [%expect
    {|
    (layered-sample-1-1 layered-sample-1-2 layered-sample-1-3)
    (layered-sample-1-1 layered-sample-1-2 layered-sample-1-3)
    (layered-sample-1-2 layered-sample-1-3)
    (layered-sample-2-1 layered-sample-2-2 layered-sample-2-3)
    |}]
;;
