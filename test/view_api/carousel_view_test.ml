open Core
open Gpuio
module C = Carousel
module W = Gpuio_protocol.Wire
module N = Gpuio_protocol.Node_id
module H = Gpuio_protocol.Handler_id

let ok = Or_error.ok_exn
let id s = C.Id.of_string s |> ok
let item s = C.Item.create ~id:(id s) ~label:s () |> ok
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let describe = View.Expert.describe

let view ?(hidden = Content_policy.Retain) ?(on_request = Fn.id) model =
  View.carousel
    model
    ~key:(Key.of_string_exn "gallery")
    ~hidden
    ~label:"Gallery"
    ~on_request
    ~content:(fun item -> [ View.text (C.Item.label item) ])
    ()
;;

let commit t view =
  let prepared = Reconciler.prepare t ~theme:Theme.default (Some view) |> ok in
  Reconciler.accept t prepared |> ok;
  match Reconciler.message prepared with
  | Some (W.Message.Apply tx) -> tx.operations
  | None -> []
  | Some _ -> assert false
;;

let%expect_test "keyed page retention, unmount, bounded controls and maximum IDs" =
  let model = C.create [ item "first"; item "pages"; item (String.make 256 'x') ] |> ok in
  let panels hidden =
    (describe (List.hd_exn (describe (view ~hidden model)).children)).children
  in
  List.iter [ Content_policy.Retain; Unmount ] ~f:(fun hidden ->
    print_s
      [%sexp
        (List.map (panels hidden) ~f:(fun panel -> List.length (describe panel).children)
         : int list)]);
  let r = Reconciler.create window in
  let initial = commit r (view model) in
  let next = C.apply_request model C.Request.next |> ok in
  let update = commit r (view next) in
  assert (
    not
      (List.exists update ~f:(function
         | W.Op.Remove _ | Create _ -> true
         | _ -> false)));
  assert (
    List.exists initial ~f:(function
      | W.Op.Create (_, Carousel, _, Some _) -> true
      | _ -> false));
  let max = C.create (List.init 128 ~f:(fun i -> item (Int.to_string i))) |> ok in
  let root = describe (view max) in
  let controls = describe (List.nth_exn root.children 1) in
  let page_controls = describe (List.nth_exn controls.children 2) in
  assert (List.length page_controls.children <= 13);
  let _ = commit (Reconciler.create window) (view max) in
  [%expect
    {|
    (1 1 1)
    (1 0 0)
    |}]
;;

let%expect_test "request delivery, replacement callback, stale model and teardown" =
  let model =
    C.create ~auto_advance:(C.Auto_advance.create () |> ok) [ item "a"; item "b" ] |> ok
  in
  let r = Reconciler.create window in
  let initial = commit r (view model) in
  let node, handler =
    List.find_map_exn initial ~f:(function
      | W.Op.Create (node, Carousel, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let dispatch request =
    Reconciler.dispatch
      r
      (W.Event.Carousel_requested (window, node, handler, 1L, request))
  in
  let requests =
    List.map
      W.Carousel.Request.[ Next; Previous; Next ]
      ~f:(fun request -> dispatch request |> Option.value_exn)
  in
  let selected =
    List.fold requests ~init:model ~f:(fun t request -> C.apply_request t request |> ok)
  in
  assert (
    String.equal (C.Id.to_string (C.Item.id (Option.value_exn (C.selected selected)))) "b");
  let _ = commit r (view selected) in
  assert (
    Option.is_none (dispatch (Auto_next { revision = 0L; from = "a"; target = "b" })));
  assert (Result.is_error (Reconciler.prepare r ~theme:Theme.default (Some (view model))));
  let _ = commit r (view ~on_request:(fun _ -> C.Request.first) selected) in
  assert (Option.equal C.Request.equal (dispatch Previous) (Some C.Request.first));
  let bad_handler = H.create ~slot:0L ~generation:2L |> ok in
  assert (
    Option.is_none
      (Reconciler.dispatch
         r
         (W.Event.Carousel_requested (window, node, bad_handler, 1L, Next))));
  let clear = Reconciler.prepare r ~theme:Theme.default None |> ok in
  Reconciler.accept r clear |> ok;
  assert (Option.is_none (dispatch Next));
  print_endline
    "ordered intents; stale automatic/model/handler rejection; callback replacement; \
     retirement";
  [%expect
    {| ordered intents; stale automatic/model/handler rejection; callback replacement; retirement |}]
;;

let%expect_test "paired appended carousel transaction and event envelopes" =
  let node = N.create ~slot:0L ~generation:1L |> ok in
  let handler = H.create ~slot:0L ~generation:1L |> ok in
  let config : W.Carousel.Config.t =
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
  let tx =
    W.Message.Apply
      { window; base = 0L; revision = 1L; operations = [ Set_carousel (node, config) ] }
  in
  let event =
    W.Event.Carousel_requested
      (window, node, handler, 1L, Auto_next { revision = 7L; from = "β"; target = "c" })
  in
  Eio_main.run (fun env ->
    let assert_hex name bytes =
      let hex = Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / name) |> String.strip in
      let actual =
        Bigstring.to_string bytes
        |> String.to_list
        |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
        |> String.concat
      in
      assert (String.equal hex actual)
    in
    assert_hex
      "carousel-transaction.hex"
      (Bin_prot.Utils.bin_dump W.Message.bin_writer_t tx);
    assert_hex "carousel-event.hex" (Bin_prot.Utils.bin_dump W.Event.bin_writer_t event));
  let encoded =
    Bin_prot.Utils.bin_dump W.Event.bin_writer_t event |> Bigstring.to_string
  in
  assert (List.equal W.Event.equal (W.Event.decode ("\001" ^ encoded) |> ok) [ event ]);
  let invalid = W.Event.Carousel_requested (window, node, handler, -1L, Next) in
  let encoded =
    Bin_prot.Utils.bin_dump W.Event.bin_writer_t invalid |> Bigstring.to_string
  in
  assert (Result.is_error (W.Event.decode ("\001" ^ encoded)));
  [%expect {| |}]
;;
