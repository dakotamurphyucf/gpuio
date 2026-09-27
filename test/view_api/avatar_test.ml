open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let owner = Asset.Expert.Owner.create ()

let handle owner slot =
  Asset.Expert.handle
    ~owner
    ~format:Png
    ~id:(Gpuio_protocol.Resource_id.create ~slot ~generation:1L |> ok)
;;

let config ?asset ?(fallback = "DM") ?(label = "Dakota") () =
  Avatar.Config.create
    ?asset
    ~fallback:(Avatar.Fallback.create fallback |> ok)
    ~description:(Image.Description.label label |> ok)
    ()
;;

let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok

let%expect_test "bounded explicit fallback and owned/foreign/no-source wire fixture" =
  List.iter
    [ ""; " "; "A\nB"; "A\tB"; "\127"; "\255"; String.make 129 'A' ]
    ~f:(fun s -> assert (Result.is_error (Avatar.Fallback.create s)));
  List.iter
    [ "👩🏽‍💻"; "名前"; String.make 128 'A' ]
    ~f:(fun s ->
      assert (String.equal (Avatar.Fallback.create s |> ok |> Avatar.Fallback.text) s));
  let configs =
    [ config ()
    ; config ~asset:(handle (Asset.Expert.Owner.create ()) 2L) ()
    ; config ~asset:(handle owner 2L) ()
    ]
  in
  let operations =
    List.concat_mapi configs ~f:(fun i c ->
      let node =
        Gpuio_protocol.Node_id.create ~slot:(Int64.of_int i) ~generation:1L |> ok
      in
      [ W.Op.Create (node, Avatar, "", None)
      ; Set_avatar (node, Avatar.Expert.to_wire c ~owner:(Some owner))
      ])
  in
  let bytes =
    W.Message.encode (Apply { window; base = 0L; revision = 1L; operations }) |> ok
  in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "avatar-request.hex") |> String.strip
    in
    assert (String.equal hex expected));
  print_endline
    "bounded explicit UTF-8 fallback; no source, foreign owner and local source fixture \
     agree";
  [%expect
    {| bounded explicit UTF-8 fallback; no source, foreign owner and local source fixture agree |}]
;;

let%expect_test "avatar preserves identity and fences image source observations" =
  let t = Reconciler.create ~asset_owner:owner window in
  let commit config on_change =
    let view = View.avatar ~on_change config in
    let update = Reconciler.prepare t ~theme:Theme.default (Some view) |> ok in
    Reconciler.accept t update |> ok;
    match Reconciler.message update with
    | Some (Apply { operations; _ }) -> operations
    | None -> []
    | Some _ -> assert false
  in
  let latest _ = `Latest
  and first _ = `First in
  let initial = commit (config ()) first in
  let node =
    List.find_map_exn initial ~f:(function
      | W.Op.Create (id, Avatar, "", None) -> Some id
      | _ -> None)
  in
  let binding ops =
    List.find_map_exn ops ~f:(function
      | W.Op.Bind (id, Some h) ->
        assert (Gpuio_protocol.Node_id.equal node id);
        Some h
      | _ -> None)
  in
  let image = handle owner 2L in
  let bound = commit (config ~asset:image ()) first |> binding in
  let event handler revision =
    W.Event.Image_state (window, node, handler, revision, W.Image.State.Loading)
  in
  assert (Option.is_some (Reconciler.dispatch t (event bound 2L)));
  assert (List.is_empty (commit (config ~asset:image ()) latest));
  (match Reconciler.dispatch t (event bound 2L) with
   | Some `Latest -> ()
   | Some `First | None -> assert false);
  (match commit (config ~asset:image ~fallback:"D" ~label:"Dakota avatar" ()) latest with
   | [ Set_avatar (id, _) ] -> assert (Gpuio_protocol.Node_id.equal id node)
   | _ -> assert false);
  let new_image = handle owner 3L in
  let replacement = commit (config ~asset:new_image ()) latest |> binding in
  assert (not (Gpuio_protocol.Handler_id.equal bound replacement));
  assert (Option.is_none (Reconciler.dispatch t (event bound 2L)));
  ignore (commit (config ()) latest : W.Op.t list);
  assert (Option.is_none (Reconciler.dispatch t (event replacement 4L)));
  let rebound = commit (config ~asset:new_image ()) latest |> binding in
  assert (not (Gpuio_protocol.Handler_id.equal replacement rebound));
  print_endline
    "stable leaf; latest callback; label/fallback updates; source replacement and \
     removal fence stale events";
  [%expect
    {| stable leaf; latest callback; label/fallback updates; source replacement and removal fence stale events |}]
;;
