open Core
open Gpuio
open Gpuio_protocol

let config ?(outside = false) () =
  Overlay.Config.create ~label:"Settings" ~dismiss_on_outside_pointer:outside ()
  |> Or_error.ok_exn
;;

let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn

let%expect_test "overlay configuration protects wire and layout invariants" =
  List.iter
    [ "", 480.; "label", Float.nan; "label", -1.; "bad\000label", 80.; "label", 16385. ]
    ~f:(fun (label, width) ->
      print_s [%sexp (Overlay.Config.create ~label ~width () |> Result.is_error : bool)]);
  [%expect
    {|
    true
    true
    true
    true
    true
    |}]
;;

let%expect_test
    "dismissal uses latest callback and policy; close/reopen rejects old generation"
  =
  let reconciler = Reconciler.create window in
  let render ?(outside = false) callback content =
    let view =
      View.dialog
        ~key:(Key.of_string_exn "dialog")
        ~config:(config ~outside ())
        ~on_dismiss:callback
        content
    in
    let update =
      Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> Or_error.ok_exn
    in
    Reconciler.accept reconciler update |> Or_error.ok_exn;
    match Reconciler.message update with
    | Some (Wire.Message.Apply tx) -> tx.operations
    | _ -> []
  in
  let content = Some (View.text "Content") in
  let operations = render (fun _ -> "first") content in
  let node, handler =
    List.find_map_exn operations ~f:(function
      | Wire.Op.Create (node, Focus_scope, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let event reason = Wire.Event.Overlay_dismissed (window, node, handler, 1L, reason) in
  let show reason =
    print_s [%sexp (Reconciler.dispatch reconciler (event reason) : string option)]
  in
  show Escape;
  show Outside_pointer;
  print_s [%sexp (render (fun _ -> "latest") content : Wire.Op.t list)];
  show Escape;
  ignore (render ~outside:true (fun _ -> "outside") content : Wire.Op.t list);
  show Outside_pointer;
  ignore (render (fun _ -> "closed") None : Wire.Op.t list);
  show Escape;
  ignore (render (fun _ -> "reopened") content : Wire.Op.t list);
  show Escape;
  [%expect
    {|
    (first)
    ()
    ()
    (latest)
    (outside)
    ()
    ()
    |}]
;;

let%expect_test "popover closure preserves its anchor" =
  let reconciler = Reconciler.create window in
  let anchor = View.button ~on_click:(fun () -> ()) "Open" in
  let commit content =
    let view =
      View.popover ~config:(config ()) ~on_dismiss:(fun _ -> ()) ~anchor content
    in
    let update =
      Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> Or_error.ok_exn
    in
    Reconciler.accept reconciler update |> Or_error.ok_exn;
    match Reconciler.message update with
    | Some (Wire.Message.Apply tx) -> tx.operations
    | _ -> []
  in
  let initial = commit None in
  let anchor =
    List.find_map_exn initial ~f:(function
      | Wire.Op.Create (id, Button, _, _) -> Some id
      | _ -> None)
  in
  ignore (commit (Some (View.text "Details")) : Wire.Op.t list);
  let closed = commit None in
  print_s
    [%sexp
      (List.exists closed ~f:(function
         | Wire.Op.Remove id -> Node_id.equal id anchor
         | _ -> false)
       : bool)];
  [%expect {| false |}]
;;

let%expect_test "overlay request and dismissal agree with independent Rust fixtures" =
  let node = Node_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let handler = Handler_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn in
  let config =
    Overlay.Config.create ~label:"Settings" ~width:220. () |> Or_error.ok_exn
  in
  let request =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (node, Focus_scope, "", Some handler)
          ; Set_focus_scope
              (node, { trap = true; auto_focus = true; restore_focus = true })
          ; Set_overlay (node, Some (Overlay.Expert.to_wire config ~kind:Dialog))
          ; Set_root (Some node)
          ]
      }
  in
  let events = [ Wire.Event.Overlay_dismissed (window, node, handler, 1L, Escape) ] in
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
        (load "overlay-v1-request.hex")
        (Wire.Message.encode request |> Or_error.ok_exn));
    let bytes = load "overlay-v1-events.hex" in
    assert (
      String.equal
        bytes
        (Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] events
         |> Bigstring.to_string));
    assert (List.equal Wire.Event.equal events (Wire.Event.decode bytes |> Or_error.ok_exn)));
  [%expect {| |}]
;;
