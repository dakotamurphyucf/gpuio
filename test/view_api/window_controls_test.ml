open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok

let client =
  Window.Decorations.Client { top = false; right = false; bottom = false; left = false }
;;

let snapshot
      ?(decorations = client)
      ?(minimize = true)
      ?(maximize = true)
      ?(maximized = false)
      ()
  : Window.Snapshot.t
  =
  { title = "Controls"
  ; appearance = Light
  ; x = 0.
  ; y = 0.
  ; width = 800.
  ; height = 600.
  ; content_width = 800.
  ; content_height = 600.
  ; active = true
  ; fullscreen = false
  ; maximized
  ; document = None
  ; presentation =
      { decorations
      ; controls = { fullscreen = true; minimize; maximize; window_menu = true }
      ; resizable = true
      }
  }
;;

let view backend snapshot =
  View.window_controls
    ~backend
    ~snapshot
    ~on_minimize:(fun () -> "minimize")
    ~on_zoom:(fun () -> "zoom")
    ~on_close:(fun () -> "request-close")
    ()
;;

let%expect_test "native and unsupported window controls are not duplicated" =
  List.iter [ Window.Backend.Macos; X11; Wayland ] ~f:(fun backend ->
    List.iter [ Window.Decorations.Server; client ] ~f:(fun decorations ->
      let root = view backend (snapshot ~decorations ()) |> View.Expert.describe in
      print_s
        [%sexp
          (backend : Window.Backend.t)
        , (List.map root.children ~f:(fun v -> (View.Expert.describe v).text)
           : string list)]));
  let root =
    view X11 (snapshot ~minimize:false ~maximize:false ()) |> View.Expert.describe
  in
  print_s
    [%sexp
      (List.map root.children ~f:(fun v -> (View.Expert.describe v).text) : string list)];
  [%expect
    {|
    (Macos ())
    (Macos ())
    (X11 ())
    (X11 (Minimize Maximize Close))
    (Wayland ())
    (Wayland (Minimize Maximize Close))
    (Close) |}]
;;

let%expect_test
    "restoring labels preserves buttons while revoked capabilities retire callbacks"
  =
  let r = Reconciler.create window in
  let commit snapshot =
    let update =
      Reconciler.prepare r ~theme:Theme.default (Some (view Wayland snapshot)) |> ok
    in
    Reconciler.accept r update |> ok;
    match Reconciler.message update with
    | Some (W.Message.Apply tx) -> tx.operations
    | None -> []
    | Some _ -> assert false
  in
  let initial = commit (snapshot ()) in
  let find label =
    List.find_map_exn initial ~f:(function
      | W.Op.Create (id, Button, text, Some handler) when String.equal text label ->
        Some (id, handler)
      | _ -> None)
  in
  let minimize, minimize_handler = find "Minimize" in
  let zoom, _ = find "Maximize" in
  let close, close_handler = find "Close" in
  let changes = commit (snapshot ~maximized:true ()) in
  assert (
    List.exists changes ~f:(function
      | W.Op.Set_text (id, "Restore") -> Gpuio_protocol.Node_id.equal id zoom
      | _ -> false));
  assert (
    not
      (List.exists changes ~f:(function
         | W.Op.Create _ | Remove _ -> true
         | _ -> false)));
  ignore (commit (snapshot ~minimize:false ~maximize:false ()) : W.Op.t list);
  let dispatch node handler =
    Reconciler.dispatch r (W.Event.Press (window, node, handler, Reconciler.revision r))
  in
  assert (Option.is_none (dispatch minimize minimize_handler));
  assert (Option.equal String.equal (dispatch close close_handler) (Some "request-close"));
  print_endline
    "capabilities retire stale actions; Restore and Close preserve native identity";
  [%expect
    {| capabilities retire stale actions; Restore and Close preserve native identity |}]
;;
