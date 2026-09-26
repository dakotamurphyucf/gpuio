open Core
open Gpuio
module C = Color_input
module P = Color_picker
module W = Gpuio_protocol.Color_input_wire

let ok = Or_error.ok_exn
let color hex = Color_value.Value.Color (Color_value.Rgba.of_hex hex |> ok)
let value = color "#00FF00"

let config
      ?(allow_empty = true)
      ?(alpha_policy = Color_value.Alpha_policy.Allow_alpha)
      ?(read_only = false)
      ?(disabled = false)
      ()
  =
  C.Config.create
    ~labels:(C.Labels.english ~control:"Accent" |> ok)
    ~alpha_policy
    ~allow_empty
    ~read_only
    ~disabled
    ()
  |> ok
;;

let normal = config ()

let import ?(generation = 1L) wire =
  C.Expert.snapshot_of_wire
    ~window:(Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok)
    ~node:(Gpuio_protocol.Node_id.create ~slot:0L ~generation |> ok)
    wire
  |> ok
;;

let wire = Color_input_test.snapshot ()
let preview = import wire

let idle ?(generation = 1L) revision =
  import
    ~generation
    { wire with revision; committed = wire.value; interaction = None; draft = None }
;;

let opened ?(config = normal) ?(value = value) state =
  let state = P.open_popup state ~config ~value in
  state, P.session state ~config ~value |> Option.value_exn |> P.Session.id
;;

let show result =
  print_s
    [%sexp
      (Result.map result ~f:(function
         | Color_value.Value.Empty -> "empty"
         | Color c -> Color_value.Rgba.to_hex c)
       : (string, P.Error.t) Result.t)]
;;

let%expect_test
    "Apply accepts valid previews, rejecting composition, invalid drafts and live drags"
  =
  let state, session = opened P.empty in
  let _, result = P.confirm state ~config:normal ~value ~session preview in
  show result;
  let state = P.observe_native state ~session preview in
  let variants =
    [ { wire with draft = Some { text = "#ff000080"; composing = true; status = Valid } }
    ; { wire with draft = Some { text = "#12"; composing = false; status = Incomplete } }
    ; { wire with interaction = Some { id = 5L; kind = Drag Hue }; draft = None }
    ]
  in
  List.iter variants ~f:(fun wire ->
    let next, result = P.confirm state ~config:normal ~value ~session (import wire) in
    assert (Option.is_some (P.session next ~config:normal ~value));
    show result);
  show (P.candidate preview ~config:(config ~read_only:true ()));
  show (P.candidate preview ~config:(config ~disabled:true ()));
  show (P.candidate preview ~config:(config ~alpha_policy:Opaque_only ()));
  let closed, result = P.confirm state ~config:normal ~value ~session preview in
  show result;
  assert (Option.is_none (P.session closed ~config:normal ~value));
  [%expect
    {|
    (Error Not_ready)
    (Error Composing)
    (Error Invalid_draft)
    (Error Drag_in_progress)
    (Error Read_only)
    (Error Disabled)
    (Error Disallowed_value)
    (Ok #FF000080)
    |}]
;;

let%expect_test
    "session, lease and revision fences survive cancellation, remount and external \
     changes"
  =
  let state, first = opened P.empty in
  assert (P.equal state (P.open_popup state ~config:normal ~value));
  let state = P.observe_native state ~session:first (idle 8L) in
  assert (P.equal state (P.observe state ~session:first (idle 7L)));
  let _, stale = P.confirm state ~config:normal ~value ~session:first (idle 7L) in
  show stale;
  let remounted = P.observe_native state ~session:first (idle ~generation:2L 0L) in
  assert (P.equal remounted (P.observe remounted ~session:first (idle 9L)));
  let unchanged, stale =
    P.confirm remounted ~config:normal ~value ~session:first (idle 9L)
  in
  assert (P.equal unchanged remounted);
  show stale;
  let closed = P.cancel remounted ~session:first in
  let next, second = opened closed in
  assert (not (P.Session.Id.equal first second));
  assert (P.equal next (P.cancel next ~session:first));
  assert (P.equal next (P.observe_native next ~session:first preview));
  let unchanged, stale = P.confirm next ~config:normal ~value ~session:first preview in
  assert (P.equal unchanged next);
  show stale;
  let next = P.observe_native next ~session:second preview in
  let closed, stale =
    P.confirm next ~config:normal ~value:(color "#123456") ~session:second preview
  in
  show stale;
  assert (Option.is_none (P.draft closed));
  assert (
    Option.is_none
      (P.session
         (P.sync next ~config:(config ~disabled:true ()) ~value)
         ~config:normal
         ~value));
  [%expect
    {|
    (Error Stale_draft)
    (Error Stale_draft)
    (Error Stale_session)
    (Error Stale_session)
    |}]
;;

let%expect_test
    "historical values seed a valid draft without changing confirmed application values"
  =
  let historical = color "#12345680" in
  List.iter [ true; false ] ~f:(fun allow_empty ->
    let config = config ~allow_empty ~alpha_policy:Opaque_only () in
    let state, id = opened ~config ~value:historical P.empty in
    let session = P.session state ~config ~value:historical |> Option.value_exn in
    assert (Color_value.Value.equal (P.Session.original session) historical);
    assert (C.Config.allows config (P.Session.initial session));
    show (Ok (P.Session.initial session));
    let cancelled = P.cancel state ~session:id in
    let reopened, _ = opened ~config ~value:historical cancelled in
    assert (
      Color_value.Value.equal
        historical
        (P.session reopened ~config ~value:historical
         |> Option.value_exn
         |> P.Session.original)));
  let config = config ~allow_empty:false () in
  let state, _ = opened ~config ~value:Color_value.Value.Empty P.empty in
  show
    (Ok
       (P.session state ~config ~value:Color_value.Value.Empty
        |> Option.value_exn
        |> P.Session.initial));
  [%expect
    {|
    (Ok empty)
    (Ok #123456)
    (Ok #000000)
    |}]
;;
