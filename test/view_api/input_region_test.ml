open Core
open Gpuio
module I = Input_region
module W = Gpuio_protocol.Input_wire

let ok = Or_error.ok_exn

let kinds : I.Kind.t list =
  [ Click
  ; Auxiliary_click
  ; Mouse_down
  ; Mouse_up
  ; Mouse_move
  ; Mouse_enter
  ; Mouse_leave
  ; Mouse_down_outside
  ; Key_down
  ; Key_up
  ; Focus
  ; Blur
  ; Scroll
  ]
;;

let subscriptions = List.map kinds ~f:(fun kind -> I.Subscription.create kind () |> ok)
let config = I.Config.create ~label:"Input café" ~focus:Tab subscriptions |> ok

let location : W.Location.t =
  { window = { x = 40.5; y = 70. }
  ; local = { x = -2.; y = 7.25 }
  ; modifiers =
      { shift = true; control = false; alt = true; command = false; function_ = true }
  }
;;

let mouse : W.Mouse.t = { location; button = Left; click_count = 2L }
let key : W.Key.t = { key = "é"; character = Some "é"; modifiers = location.modifiers }

let events : W.Event.t list =
  [ W.Event.Click mouse
  ; Auxiliary_click { mouse with button = Right }
  ; Mouse_down mouse
  ; Mouse_up mouse
  ; Mouse_move { location; pressed_button = None }
  ; Mouse_enter
  ; Mouse_leave
  ; Mouse_down_outside mouse
  ; Key_down (key, true)
  ; Key_up key
  ; Focus
  ; Blur
  ; Scroll { location; delta = Pixels { x = 0.5; y = -12. }; phase = Moved }
  ]
  @ List.concat_map [ W.Button.Left; Right; Middle; Back; Forward ] ~f:(fun button ->
    [ W.Event.Mouse_move { location; pressed_button = Some button }
    ; Mouse_down { mouse with button; click_count = 0xffff_ffffL }
    ])
  @ List.map [ W.Touch_phase.Started; Moved; Ended; Cancelled ] ~f:(fun phase ->
    W.Event.Scroll { location; delta = Lines { x = -1.; y = 0. }; phase })
  @ [ W.Event.Key_down
        ( { key = "space"
          ; character = None
          ; modifiers =
              { shift = false
              ; control = false
              ; alt = false
              ; command = false
              ; function_ = false
              }
          }
        , false )
    ]
;;

let%expect_test "canonical subscriptions validate policies, labels and focus" =
  assert (
    I.Config.equal
      config
      (I.Config.create ~label:"Input café" ~focus:Tab (List.rev subscriptions) |> ok));
  assert (not (I.Config.is_disabled config));
  assert (List.equal I.Subscription.equal subscriptions (I.Config.subscriptions config));
  List.iter kinds ~f:(fun kind ->
    let derived =
      match kind with
      | Click
      | Auxiliary_click
      | Mouse_enter
      | Mouse_leave
      | Mouse_down_outside
      | Focus
      | Blur -> true
      | Mouse_down | Mouse_up | Mouse_move | Key_down | Key_up | Scroll -> false
    in
    List.iter [ I.Phase.Capture; Bubble ] ~f:(fun phase ->
      List.iter
        [ I.Policy.Observe; Stop_propagation; Prevent_default; Prevent_and_stop ]
        ~f:(fun policy ->
          let result = I.Subscription.create kind ~phase ~policy () in
          assert (
            Bool.equal
              (Result.is_ok result)
              ((not derived)
               || (I.Phase.equal phase Bubble && I.Policy.equal policy Observe))))));
  List.iter
    [ []; [ List.hd_exn subscriptions; List.hd_exn subscriptions ] ]
    ~f:(fun subscriptions ->
      assert (Result.is_error (I.Config.create ~label:"Example" subscriptions)));
  assert (Result.is_error (I.Config.create ~label:"Example" subscriptions));
  List.iter
    [ ""; " \t\011"; "bad\000name"; "\255"; String.make 4097 'a' ]
    ~f:(fun label ->
      assert (Result.is_error (I.Config.create ~label ~focus:Click subscriptions)));
  List.iter
    [ "\194\160"; String.make 4096 'a' ]
    ~f:(fun label ->
      assert (Result.is_ok (I.Config.create ~label ~focus:Tab subscriptions)));
  let raw =
    I.Subscription.create Key_down ~phase:Capture ~policy:Prevent_and_stop () |> ok
  in
  assert (Result.is_ok (I.Config.create ~label:"Descendant keys" [ raw ]));
  print_s [%sexp (List.length kinds * 2 * 4 : int), (I.Subscription.kind raw : I.Kind.t)];
  [%expect {| (104 Key_down) |}]
;;

let hex bytes =
  Bigstring.to_string bytes
  |> String.to_list
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let%expect_test "independent Rust bytes preserve payload units, phases and button values" =
  let actual =
    hex (Bin_prot.Utils.bin_dump W.Config.bin_writer_t (I.Expert.to_wire config))
    :: List.map events ~f:(fun event ->
      assert (W.Event.valid event);
      ignore (I.Expert.event_of_wire event |> ok : I.Event.t);
      hex (Bin_prot.Utils.bin_dump W.Event.bin_writer_t event))
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "input-values.hex") |> String.strip
    in
    assert (String.equal (String.concat ~sep:"\n" actual) expected);
    let lines = String.split_lines expected in
    List.iter2_exn (List.tl_exn lines) events ~f:(fun line expected ->
      let bytes =
        String.init
          (String.length line / 2)
          ~f:(fun i ->
            Char.of_int_exn (Int.of_string ("0x" ^ String.sub line ~pos:(i * 2) ~len:2)))
        |> Bigstring.of_string
      in
      let pos_ref = ref 0 in
      let observed = W.Event.bin_read_t bytes ~pos_ref in
      assert (!pos_ref = Bigstring.length bytes);
      assert (W.Event.equal expected observed);
      ignore (I.Expert.event_of_wire observed |> ok : I.Event.t)));
  let observed = I.Expert.event_of_wire (Mouse_down mouse) |> ok in
  (match observed with
   | Mouse_down { location; button; click_count } ->
     print_s
       [%sexp (location.local.x : float), (button : I.Button.t), (click_count : int)]
   | Click _
   | Auxiliary_click _
   | Mouse_up _
   | Mouse_move _
   | Mouse_enter
   | Mouse_leave
   | Mouse_down_outside _
   | Key_down _
   | Key_up _
   | Focus
   | Blur
   | Scroll _ -> assert false);
  print_s [%sexp (List.length events : int)];
  [%expect
    {|
    (-2 Left 2)
    28
    |}]
;;

let%expect_test "malformed native samples cannot enter public input types" =
  let reject event = assert (Result.is_error (I.Expert.event_of_wire event)) in
  List.iter [ -1L; 0L; 0x1_0000_0000L ] ~f:(fun click_count ->
    reject (Mouse_down { mouse with click_count }));
  reject (Auxiliary_click mouse);
  reject (Click { mouse with button = Right });
  List.iter [ Float.nan; Float.infinity; Float.neg_infinity ] ~f:(fun value ->
    reject
      (Mouse_move
         { location = { location with local = { location.local with x = value } }
         ; pressed_button = None
         });
    reject (Scroll { location; delta = Lines { x = value; y = 0. }; phase = Ended }));
  List.iter
    [ ""; "\255"; "a\000b"; String.make 257 'a' ]
    ~f:(fun text ->
      reject (Key_up { key with key = text });
      reject (Key_down ({ key with character = Some text }, false)));
  let unknown =
    W.Subscription.{ kind = Focus; phase = Capture; policy = Stop_propagation }
  in
  assert (not (W.Subscription.valid unknown));
  let raw = I.Expert.to_wire config in
  assert (not (W.Config.valid { raw with subscriptions = List.rev raw.subscriptions }));
  print_endline
    "finite coordinates, real click counts/buttons, bounded UTF-8, legal policies and \
     canonical subscriptions";
  [%expect
    {| finite coordinates, real click counts/buttons, bounded UTF-8, legal policies and canonical subscriptions |}]
;;

let%expect_test "independent native phase/policy/focus/disabled encodings" =
  let configurations =
    List.concat_map [ I.Phase.Capture; Bubble ] ~f:(fun phase ->
      List.map
        [ I.Policy.Observe; Stop_propagation; Prevent_default; Prevent_and_stop ]
        ~f:(fun policy ->
          let subscription = I.Subscription.create Key_down ~phase ~policy () |> ok in
          I.Config.create ~label:"Keys" ~disabled:true [ subscription ] |> ok))
    @ [ I.Config.create
          ~label:"Focus"
          ~focus:Click
          [ I.Subscription.create Focus () |> ok ]
        |> ok
      ]
  in
  let actual =
    List.map configurations ~f:(fun config ->
      hex (Bin_prot.Utils.bin_dump W.Config.bin_writer_t (I.Expert.to_wire config)))
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "input-policies.hex") |> String.strip
    in
    assert (String.equal (String.concat ~sep:"\n" actual) expected));
  print_s [%sexp (List.length configurations : int)];
  [%expect {| 9 |}]
;;

module Wire = Gpuio_protocol.Wire

let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok
let handler generation = Gpuio_protocol.Handler_id.create ~slot:0L ~generation |> ok

let wire_event ?(generation = 1L) ?(revision = 1L) event =
  Wire.Event.Input_observed (window, node, handler generation, revision, event)
;;

let%expect_test "mounted request and event envelopes match independent native bytes" =
  let request =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (node, Input_region, "", Some (handler 1L))
          ; Set_input_region (node, I.Expert.to_wire config)
          ; Set_root (Some node)
          ]
      }
  in
  let envelopes = List.map events ~f:(fun event -> wire_event event) in
  let bytes =
    Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] envelopes
    |> Bigstring.to_string
  in
  assert (List.equal Wire.Event.equal envelopes (Wire.Event.decode bytes |> ok));
  Eio_main.run (fun env ->
    List.iter
      [ "input-request.hex", Wire.Message.encode request |> ok
      ; "input-events.hex", bytes
      ]
      ~f:(fun (name, actual) ->
        assert (
          String.equal
            (hex (Bigstring.of_string actual))
            (Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / name) |> String.strip))));
  for length = 0 to String.length bytes - 1 do
    assert (Result.is_error (Wire.Event.decode (String.prefix bytes length)))
  done;
  assert (Result.is_error (Wire.Event.decode (bytes ^ "\000")));
  let invalid =
    [ wire_event (Mouse_up { mouse with click_count = 0L })
    ; wire_event ~revision:(-1L) Focus
    ]
  in
  List.iter invalid ~f:(fun event ->
    let bytes =
      Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] [ event ]
      |> Bigstring.to_string
    in
    assert (Result.is_error (Wire.Event.decode bytes)));
  print_endline
    "paired mounted request, all observations, strict consumption and validation";
  [%expect
    {| paired mounted request, all observations, strict consumption and validation |}]
;;

let%expect_test "config retires old observation handlers while child identity survives" =
  let r = Reconciler.create window in
  let view config prefix =
    View.input_region
      ~config
      ~on_event:(fun event ->
        prefix ^ Sexp.to_string (I.Kind.sexp_of_t (I.Event.kind event)))
      [ View.text ~key:(Key.of_string_exn "retained") "Child" ]
  in
  let prepare config prefix =
    Reconciler.prepare r ~theme:Theme.default (Some (view config prefix)) |> ok
  in
  let first = prepare config "old:" in
  Reconciler.accept r first |> ok;
  assert (
    Option.equal
      String.equal
      (Reconciler.dispatch r (wire_event Focus))
      (Some "old:Focus"));
  let closure = prepare config "new:" in
  assert (Option.is_none (Reconciler.message closure));
  Reconciler.accept r closure |> ok;
  assert (
    Option.equal
      String.equal
      (Reconciler.dispatch r (wire_event Focus))
      (Some "new:Focus"));
  let changed_config = I.Config.create ~label:"Renamed" ~focus:Tab subscriptions |> ok in
  let changed = prepare changed_config "changed:" in
  (match Reconciler.message changed with
   | Some
       (Apply
          { operations = [ Bind (bound, Some fresh); Set_input_region (configured, _) ]
          ; _
          }) ->
     assert (
       Gpuio_protocol.Node_id.equal bound node
       && Gpuio_protocol.Node_id.equal configured node);
     assert (Gpuio_protocol.Handler_id.equal fresh (handler 2L))
   | _ -> assert false);
  (* Pending transactions leave the accepted handler in force. *)
  assert (Option.is_some (Reconciler.dispatch r (wire_event Focus)));
  Reconciler.accept r changed |> ok;
  assert (Option.is_none (Reconciler.dispatch r (wire_event Focus)));
  assert (
    Option.equal
      String.equal
      (Reconciler.dispatch r (wire_event ~generation:2L ~revision:2L Focus))
      (Some "changed:Focus"));
  let disabled =
    I.Config.create ~label:"Renamed" ~disabled:true ~focus:Tab subscriptions |> ok
  in
  Reconciler.accept r (prepare disabled "disabled:") |> ok;
  assert (
    Option.is_none (Reconciler.dispatch r (wire_event ~generation:3L ~revision:3L Focus)));
  let enabled =
    I.Config.create ~label:"Only keys" [ I.Subscription.create Key_down () |> ok ] |> ok
  in
  Reconciler.accept r (prepare enabled "keys:") |> ok;
  assert (
    Option.is_none (Reconciler.dispatch r (wire_event ~generation:4L ~revision:4L Focus)));
  assert (
    Option.is_some
      (Reconciler.dispatch
         r
         (wire_event ~generation:4L ~revision:4L (Key_down (key, false)))));
  Reconciler.accept r (Reconciler.prepare r ~theme:Theme.default None |> ok) |> ok;
  assert (
    Option.is_none
      (Reconciler.dispatch
         r
         (wire_event ~generation:4L ~revision:4L (Key_down (key, false)))));
  print_endline
    "latest accepted closure; config-bound generations; unchanged child; \
     disabled/unsubscribed/removed observations rejected";
  [%expect
    {| latest accepted closure; config-bound generations; unchanged child; disabled/unsubscribed/removed observations rejected |}]
;;
