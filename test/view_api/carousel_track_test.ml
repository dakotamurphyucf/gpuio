open Core
open Gpuio
module C = Carousel_track
module W = Gpuio_protocol.Carousel_track_wire

let ok = Or_error.ok_exn
let id name = C.Id.of_string name |> ok
let item name = C.Item.create ~id:(id name) ~label:name () |> ok
let values = List.map [ "a"; "β"; "c" ] ~f:item

let selected t =
  C.selected t |> Option.map ~f:(fun item -> C.Id.to_string (C.Item.id item))
;;

let apply t request = C.apply_request t request |> ok
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok
let handler = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:1L |> ok

let wire_request ?(handler = handler) wire =
  C.Expert.request_of_wire ~window ~node ~handler wire |> ok
;;

let layout ?handler ?(lineage = 0L) ?(epoch = 0L) ?(looping = W.Loop.Finite) canonical =
  wire_request ?handler (Layout { lineage; epoch; stops = Some { canonical; looping } })
;;

let unavailable ?(epoch = 0L) ?(lineage = 0L) () =
  wire_request (Layout { lineage; epoch; stops = None })
;;

let config = C.Expert.to_wire

let measured ?(looping = false) canonical =
  let t = C.create ~looping values |> ok in
  apply t (layout ~looping:(if looping then Jump else Finite) canonical)
;;

let%expect_test
    "measured navigation preserves queued relative intents and logical selection"
  =
  let initial = C.create values |> ok in
  assert (not (C.has_layout initial || C.can_next initial || C.can_previous initial));
  assert (Option.equal String.equal (selected (apply initial C.Request.next)) (Some "a"));
  let t = apply initial (layout [ 0L; 1L; 1L ]) in
  let states =
    List.folding_map
      C.Request.[ next; next; last; previous; first ]
      ~init:t
      ~f:(fun t action ->
        let t = apply t action in
        t, selected t)
  in
  print_s [%sexp (states : string option list)];
  let t = measured ~looping:true [ 0L; 1L; 2L ] in
  let states =
    List.folding_map
      C.Request.[ next; next; next; next; previous ]
      ~init:t
      ~f:(fun t action ->
        let t = apply t action in
        t, selected t)
  in
  print_s [%sexp (states : string option list)];
  let disabled = C.with_disabled initial true |> ok in
  assert (
    Option.equal String.equal (selected (C.select disabled (id "c") |> ok)) (Some "c"));
  assert (Result.is_error (C.select initial (id "missing")));
  [%expect
    {|
    (("\206\178") ("\206\178") (c) (a) (a))
    (("\206\178") (c) (a) ("\206\178") (a))
    |}]
;;

let%expect_test
    "layout epochs, unavailable geometry and collection lineage are independent of \
     selection"
  =
  let t = measured [ 0L; 1L; 1L ] in
  let t = apply t (layout ~epoch:2L [ 0L; 1L; 2L ]) in
  let t = apply t (layout ~epoch:1L [ 0L; 1L; 1L ]) in
  let t = apply (apply t C.Request.next) C.Request.next in
  assert (Option.equal String.equal (selected t) (Some "c"));
  assert (Int64.equal (config t).lineage 0L);
  let same =
    C.with_items t (List.map values ~f:(fun item -> C.Item.with_data item ())) |> ok
  in
  assert (C.has_layout same);
  assert (W.Config.equal (config same) (config t));
  let t = C.with_disabled t true |> ok in
  let t = apply t (unavailable ~epoch:3L ()) in
  assert (not (C.has_layout t));
  let t = apply t (layout ~epoch:2L [ 0L; 1L; 2L ]) in
  assert (not (C.has_layout t));
  let t = apply t (layout ~epoch:4L [ 0L; 1L; 2L ]) in
  assert (C.has_layout t && not (C.can_previous t));
  let t = C.with_disabled t false |> ok in
  assert (C.can_previous t);
  let t = C.with_items t [ item "c"; item "a"; item "β" ] |> ok in
  assert (not (C.has_layout t));
  assert (Int64.equal (config t).lineage 1L);
  assert (Option.equal String.equal (selected t) (Some "c"));
  let t = apply t (layout ~epoch:5L [ 0L; 1L; 2L ]) in
  assert (not (C.has_layout t));
  let t = apply t (layout ~lineage:1L ~epoch:6L [ 0L; 1L; 2L ]) in
  let t = C.with_axis t Vertical |> ok in
  assert (Int64.equal (config t).lineage 2L && not (C.has_layout t));
  let t = C.with_looping t true |> ok in
  assert (Int64.equal (config t).lineage 3L);
  let t =
    apply t (layout ~lineage:3L ~epoch:Int64.max_value ~looping:Continuous [ 0L; 1L; 2L ])
  in
  let t = apply t (unavailable ~lineage:3L ~epoch:0L ()) in
  assert (C.has_layout t);
  let fresh = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:2L |> ok in
  let t =
    apply t (layout ~handler:fresh ~lineage:3L ~epoch:0L ~looping:Finite [ 0L; 0L; 0L ])
  in
  assert (C.has_layout t && not (C.can_previous t || C.can_next t));
  print_endline
    "latest geometry, unavailable epochs and remounted source accepted without resetting \
     selection";
  [%expect
    {| latest geometry, unavailable epochs and remounted source accepted without resetting selection |}]
;;

let%expect_test
    "automatic movement fences source revision geometry and measured successor"
  =
  let t = measured ~looping:true [ 0L; 0L; 2L ] in
  let t = C.with_auto_advance t (Some (C.Auto_advance.create () |> ok)) |> ok in
  let proposal ?(epoch = 0L) ?(handler = handler) ?(target = "c") t =
    wire_request
      ~handler
      (Auto_next
         { revision = (config t).carousel.revision
         ; geometry_epoch = epoch
         ; from = "a"
         ; target
         })
  in
  assert (
    Option.equal String.equal (selected (apply t (proposal ~target:"β" t))) (Some "a"));
  let other = Gpuio_protocol.Handler_id.create ~slot:1L ~generation:1L |> ok in
  assert (
    Option.equal String.equal (selected (apply t (proposal ~handler:other t))) (Some "a"));
  let action = proposal t in
  let next = apply t action in
  assert (Option.equal String.equal (selected next) (Some "c"));
  assert (
    Gpuio_protocol.Carousel_wire.Direction.equal (config next).carousel.direction Next);
  assert (W.Config.equal (config next) (config (apply next action)));
  let newer = apply t (layout ~epoch:1L ~looping:Jump [ 0L; 0L; 2L ]) in
  assert (Option.equal String.equal (selected (apply newer action)) (Some "a"));
  assert (
    Option.equal
      String.equal
      (selected (apply newer (proposal ~epoch:1L newer)))
      (Some "c"));
  let restarted = C.restart_auto_advance t |> ok in
  assert (Option.equal String.equal (selected (apply restarted action)) (Some "a"));
  let unavailable = apply t (unavailable ~epoch:2L ()) in
  assert (Option.equal String.equal (selected (apply unavailable action)) (Some "a"));
  print_endline "automatic target can skip duplicate stops; stale proposals never advance";
  [%expect {| automatic target can skip duplicate stops; stale proposals never advance |}]
;;

let hex bytes =
  String.to_list bytes
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let%expect_test "independent track payload fixtures and invalid geometry domains" =
  let check env name writer value =
    let encoded = Bin_prot.Utils.bin_dump writer value |> Bigstring.to_string |> hex in
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / (name ^ ".hex")) |> String.strip
    in
    assert (String.equal encoded expected)
  in
  let carousel : Gpuio_protocol.Carousel_wire.Config.t =
    { revision = 7L
    ; ids = [ "a"; "β"; "c" ]
    ; selected = Some 1L
    ; looping = true
    ; disabled = false
    ; axis = Vertical
    ; auto_advance_ms = Some 5000L
    ; direction = Next
    }
  in
  Eio_main.run (fun env ->
    check env "carousel-track-config" W.Config.bin_writer_t { carousel; lineage = 3L };
    check
      env
      "carousel-track-layout"
      W.Request.bin_writer_t
      (Layout
         { lineage = 3L
         ; epoch = 9L
         ; stops = Some { canonical = [ 0L; 1L; 1L ]; looping = Jump }
         });
    check
      env
      "carousel-track-unavailable"
      W.Request.bin_writer_t
      (Layout { lineage = 3L; epoch = 10L; stops = None });
    check
      env
      "carousel-track-auto"
      W.Request.bin_writer_t
      (Auto_next { revision = 7L; geometry_epoch = 9L; from = "β"; target = "c" }));
  List.iter
    [ [ 1L ]; [ -1L ]; [ 0L; 2L ]; [ 0L; 0L; 1L ]; List.init 129 ~f:Int64.of_int ]
    ~f:(fun canonical -> assert (not (W.Stops.valid { canonical; looping = Finite })));
  assert (not (W.Stops.valid { canonical = [ 0L; 0L ]; looping = Continuous }));
  assert (W.Stops.valid { canonical = [ 0L; 0L; 2L; 2L ]; looping = Jump });
  let t = C.create values |> ok in
  assert (not (C.has_layout (apply t (layout [ 0L; 1L ]))));
  assert (not (C.has_layout (apply t (layout ~looping:Jump [ 0L; 1L; 2L ]))));
  assert (
    Result.is_error
      (C.Expert.request_of_wire
         ~window
         ~node
         ~handler
         (Layout { lineage = 0L; epoch = -1L; stops = None })));
  let empty = C.create [] |> ok |> fun t -> apply t (layout []) in
  assert (C.has_layout empty && not (C.can_next empty));
  let before = config t in
  let changed_axis =
    { before with carousel = { before.carousel with axis = Vertical } }
  in
  assert (not (W.Config.can_replace changed_axis before));
  assert (W.Config.can_replace (config (C.with_axis t Vertical |> ok)) before);
  print_endline "paired payload fixtures and bounded canonical-stop validation pass";
  [%expect {| paired payload fixtures and bounded canonical-stop validation pass |}]
;;
