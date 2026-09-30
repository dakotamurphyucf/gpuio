open Core
open Gpuio
module C = Carousel
module W = Gpuio_protocol.Carousel_wire

let ok = Or_error.ok_exn
let id name = C.Id.of_string name |> ok
let item ?(data = "") name = C.Item.create ~id:(id name) ~label:name data |> ok
let apply t request = C.apply_request t request |> ok

let selected t =
  C.selected t |> Option.map ~f:(fun item -> C.Id.to_string (C.Item.id item))
;;

let config t = C.Expert.to_wire t ~axis:Horizontal

let auto_request t =
  let wire = config t in
  C.Expert.request_of_wire
    (W.Request.Auto_next { revision = wire.revision; from = "a"; target = "b" })
  |> ok
;;

let%expect_test "relative requests, boundaries, looping and disabled policy" =
  let t = C.create [ item "a"; item "b"; item "c" ] |> ok in
  let states =
    List.folding_map
      C.Request.[ previous; next; next; next; first; last ]
      ~init:t
      ~f:(fun t request ->
        let t = apply t request in
        t, selected t)
  in
  print_s [%sexp (states : string option list)];
  let looping = C.with_looping t true |> ok |> fun t -> apply t C.Request.previous in
  print_s [%sexp (selected looping : string option)];
  assert (W.Direction.equal (config looping).direction Previous);
  assert (W.Direction.equal (config (apply looping C.Request.next)).direction Next);
  let disabled = C.with_disabled t true |> ok in
  let explicit = C.select disabled (id "b") |> ok in
  print_s
    [%sexp
      (selected (apply disabled C.Request.next) : string option)
    , (selected explicit : string option)
    , (C.can_next explicit : bool)
    , (C.can_previous explicit : bool)];
  print_s [%sexp (selected (apply t (C.Request.select (id "missing"))) : string option)];
  assert (Result.is_error (C.select t (id "missing")));
  [%expect
    {|
    ((a) (b) (c) (c) (a) (c))
    (c)
    ((a) (b) false false)
    (a)
    |}]
;;

let%expect_test "reorder preserves ID; shrink selects nearest surviving position" =
  let t = C.create ~selected:(id "b") [ item "a"; item "b"; item "c" ] |> ok in
  let t = C.with_items t [ item "c"; item ~data:"draft" "b"; item "a" ] |> ok in
  print_s
    [%sexp
      (selected t : string option)
    , (C.selected t |> Option.map ~f:C.Item.data : string option)];
  let t = C.with_items t [ item "c"; item "a" ] |> ok in
  print_s [%sexp (selected t : string option)];
  let t = C.with_items t [] |> ok in
  print_s [%sexp (selected t : string option), (C.can_next t : bool)];
  let t = C.with_items t [ item "b" ] |> ok |> fun t -> C.with_looping t true |> ok in
  print_s
    [%sexp (selected t : string option), (C.can_next t : bool), (C.can_previous t : bool)];
  [%expect
    {|
    ((b) (draft))
    (a)
    (() false)
    ((b) false false)
    |}]
;;

let%expect_test "automatic proposals cannot revive after a complete loop or restart" =
  let t =
    C.create
      ~looping:true
      ~auto_advance:(C.Auto_advance.create () |> ok)
      [ item "a"; item "b"; item "c" ]
    |> ok
  in
  let request = auto_request t in
  let b = apply t request in
  print_s
    [%sexp (selected b : string option), (selected (apply b request) : string option)];
  let a = apply (apply b C.Request.next) C.Request.next in
  print_s [%sexp (selected (apply a request) : string option)];
  let request = auto_request a in
  let restarted = C.restart_auto_advance a |> ok in
  print_s [%sexp (selected (apply restarted request) : string option)];
  let refreshed =
    C.with_items a [ item ~data:"new data" "a"; item "b"; item "c" ] |> ok
  in
  print_s [%sexp (selected (apply refreshed request) : string option)];
  let reordered = C.with_items a [ item "a"; item "c"; item "b" ] |> ok in
  print_s [%sexp (selected (apply reordered request) : string option)];
  let wrong_successor =
    C.Expert.request_of_wire
      (W.Request.Auto_next
         { revision = (config reordered).revision; from = "a"; target = "b" })
    |> ok
  in
  print_s [%sexp (selected (apply reordered wrong_successor) : string option)];
  let stopped = C.with_auto_advance a None |> ok in
  print_s [%sexp (selected (apply stopped request) : string option)];
  [%expect
    {|
    ((b) (b))
    (a)
    (a)
    (b)
    (a)
    (a)
    (a)
    |}]
;;

let%expect_test "validated items, metadata budgets and automatic timing" =
  List.iter
    [ ""; "\255"; "x\000y"; String.make 257 'x' ]
    ~f:(fun name -> assert (Result.is_error (C.Id.of_string name)));
  List.iter
    [ ""; "\255"; "x\000y"; String.make 4097 'x' ]
    ~f:(fun label -> assert (Result.is_error (C.Item.create ~id:(id "a") ~label ())));
  assert (Result.is_error (C.create [ item "a"; item "a" ]));
  assert (Result.is_error (C.create ~selected:(id "a") []));
  let many count = List.init count ~f:(fun i -> item (Int.to_string i)) in
  assert (Result.is_ok (C.create (many 128)));
  assert (Result.is_error (C.create (many 129)));
  let large count =
    List.init count ~f:(fun i ->
      C.Item.create ~id:(id (Int.to_string i)) ~label:(String.make 4000 'x') () |> ok)
  in
  assert (Result.is_ok (C.create (large 65)));
  assert (Result.is_error (C.create (large 66)));
  List.iter
    [ Time_ns.Span.zero; Time_ns.Span.of_ms 999.; Time_ns.Span.of_sec 3601. ]
    ~f:(fun interval -> assert (Result.is_error (C.Auto_advance.create ~interval ())));
  let rounded =
    C.Auto_advance.create ~interval:(Time_ns.Span.of_ns 1_000_000_001.) () |> ok
  in
  print_s [%sexp (C.Auto_advance.interval rounded |> Time_ns.Span.to_int63_ns : Int63.t)];
  let many = C.create ~looping:true (many 128) |> ok in
  let final =
    List.fold (List.range 0 10_000) ~init:many ~f:(fun t _ ->
      let t = apply t C.Request.next in
      assert (W.Config.valid (config t));
      t)
  in
  print_s [%sexp (selected final : string option)];
  [%expect
    {|
    1001000000
    (16)
    |}]
;;

let%expect_test "independent carousel config/request fixtures" =
  let wire : W.Config.t =
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
  let request = W.Request.Auto_next { revision = 7L; from = "β"; target = "c" } in
  Eio_main.run (fun env ->
    let load file =
      let hex = Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / file) |> String.strip in
      String.init
        (String.length hex / 2)
        ~f:(fun i ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(i * 2) ~len:2)))
    in
    assert (
      String.equal
        (load "carousel-config.hex")
        (Bin_prot.Utils.bin_dump W.Config.bin_writer_t wire |> Bigstring.to_string));
    assert (
      String.equal
        (load "carousel-request.hex")
        (Bin_prot.Utils.bin_dump W.Request.bin_writer_t request |> Bigstring.to_string)));
  assert (W.Config.valid wire);
  List.iter
    [ { wire with revision = -1L }
    ; { wire with ids = [ "a"; "a" ] }
    ; { wire with selected = None }
    ; { wire with selected = Some 3L }
    ; { wire with auto_advance_ms = Some 999L }
    ]
    ~f:(fun invalid -> assert (not (W.Config.valid invalid)));
  List.iter
    [ W.Request.Auto_next { revision = -1L; from = "a"; target = "b" }
    ; Auto_next { revision = 0L; from = "a"; target = "a" }
    ; Select ""
    ]
    ~f:(fun invalid -> assert (Result.is_error (C.Expert.request_of_wire invalid)));
  print_s [%sexp (C.Expert.request_of_wire request |> ok : C.Request.t)];
  [%expect {| (Auto_next (revision 7) (from "\206\178") (target c)) |}]
;;

let%expect_test "navigation family capability uses the shared 64-bit handshake" =
  let module Wire = Gpuio_protocol.Wire in
  assert (Int64.equal (Int64.bit_and Wire.capabilities 274877906944L) 274877906944L);
  let bytes =
    Wire.Message.encode (Hello (Wire.version, Wire.capabilities)) |> Or_error.ok_exn
  in
  String.iter bytes ~f:(fun byte -> printf "%02x" (Char.to_int byte));
  print_endline "";
  [%expect {| 0001fcffffffffffffff00 |}]
;;
