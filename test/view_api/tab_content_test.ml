open Core
open Gpuio
module P = Gpuio_protocol
module W = P.Wire

let ok = Or_error.ok_exn
let id s = Choice.Id.of_string s |> ok

let config names =
  Choice.Config.create
    ~label:"Tabs"
    ~options:
      (List.map names ~f:(fun name -> Choice.create ~id:(id name) ~label:name () |> ok)
       |> Choice.Collection.create
       |> ok)
    ~selected:None
    ()
  |> ok
;;

let make ?max_width names content action =
  View.tab_bar_with_content
    ~key:(Key.of_string_exn "tabs")
    ?max_width
    ~config:(config names)
    ~content
    ~on_select:(fun _ -> action)
    ()
;;

let%expect_test "structured tab content validates labels, names, IDs and aggregate bounds"
  =
  let button = View.button ~on_click:(fun () -> ()) "Close" in
  assert (Result.is_error (View.Tab_content.create ~label:(Custom button) ()));
  let observed = View.with_hover button ~on_change:(fun _ -> ()) |> ok in
  assert (Result.is_error (View.Tab_content.create ~label:(Custom observed) ()));
  let part = View.Tab_content.create ~prefix:button ~suffix:button () |> ok in
  ignore (make [ "a" ] [ id "a", part ] () |> ok : unit View.t);
  List.iter
    [ Float.nan; Float.infinity; 0.; 1e6 +. 1. ]
    ~f:(fun max_width -> assert (Result.is_error (make ~max_width [ "a" ] [] ())));
  assert (Result.is_error (make [ "a" ] [ id "absent", part ] ()));
  assert (Result.is_error (make [ "a" ] [ id "a", part; id "a", part ] ()));
  assert (Result.is_error (make [ " " ] [] ()));
  let huge = View.column (List.init 4090 ~f:(fun _ -> View.text "x")) in
  let part = View.Tab_content.create ~suffix:huge () |> ok in
  assert (Result.is_error (make [ "a"; "b" ] [ id "a", part ] ()));
  let deep =
    List.fold (List.init 127 ~f:Fn.id) ~init:button ~f:(fun child _ ->
      View.column [ child ])
  in
  let part = View.Tab_content.create ~prefix:deep () |> ok in
  assert (Result.is_error (make [ "a" ] [ id "a", part ] ()));
  let node = P.Node_id.create ~slot:0L ~generation:1L |> ok in
  let bytes =
    Bin_prot.Utils.bin_dump
      W.Op.bin_writer_t
      (Set_tab_content
         (node, Some { max_width = Some 160.; labels = [ Default; Custom; Hidden ] }))
    |> Bigstring.to_string
  in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  assert (String.equal hex "6300010101000000000000644003000102");
  print_endline
    "passive labels, independent actions, names, IDs, bounds and paired Op99 bytes";
  [%expect
    {| passive labels, independent actions, names, IDs, bounds and paired Op99 bytes |}]
;;

let%expect_test "structured tab parts survive reorder, update callbacks and retire safely"
  =
  let window = P.Window_id.create ~slot:0L ~generation:1L |> ok in
  let r = Reconciler.create window in
  let commit view =
    let update = Reconciler.prepare r ~theme:Theme.default (Some view) |> ok in
    Reconciler.accept r update |> ok;
    match Reconciler.message update with
    | Some (Apply tx) -> tx.operations
    | None -> []
    | Some _ -> assert false
  in
  let view names action =
    make
      ~max_width:160.
      names
      [ ( id "a"
        , View.Tab_content.create
            ~suffix:
              (View.button
                 ~key:(Key.of_string_exn "close")
                 ~on_click:(fun () -> action)
                 "Close")
            ()
          |> ok )
      ]
      action
    |> ok
  in
  let first = commit (view [ "a"; "b" ] 1) in
  let close, handler =
    List.find_map_exn first ~f:(function
      | W.Op.Create (n, Button, "Close", Some h) -> Some (n, h)
      | _ -> None)
  in
  let owner =
    List.find_map_exn first ~f:(function
      | W.Op.Create (n, Tab_bar, _, _) -> Some n
      | _ -> None)
  in
  let reordered = commit (view [ "b"; "a" ] 2) in
  assert (
    not
      (List.exists reordered ~f:(function
         | W.Op.Create _ | Remove _ -> true
         | _ -> false)));
  assert (
    Option.equal
      Int.equal
      (Reconciler.dispatch r (W.Event.Press (window, close, handler, 1L)))
      (Some 2));
  assert (List.is_empty (commit (view [ "b"; "a" ] 2)));
  let reset =
    commit
      (View.tab_bar
         ~key:(Key.of_string_exn "tabs")
         ~config:(config [ "b"; "a" ])
         ~on_select:(fun _ -> 3)
         ())
  in
  assert (
    List.exists reset ~f:(function
      | W.Op.Set_tab_content (n, None) -> P.Node_id.equal n owner
      | _ -> false));
  assert (
    Option.is_none (Reconciler.dispatch r (W.Event.Press (window, close, handler, 1L))));
  print_endline
    "keyed parts retained; latest callback; no-op; reset and stale callback retirement";
  [%expect
    {| keyed parts retained; latest callback; no-op; reset and stale callback retirement |}]
;;

let%expect_test "public structured tab transactions replay in the native tree" =
  let window = P.Window_id.create ~slot:0L ~generation:1L |> ok in
  let r = Reconciler.create window in
  let content =
    View.Tab_content.create
      ~prefix:(View.text "•")
      ~label:(Custom (View.text "Decorative a"))
      ~suffix:(View.button ~on_click:(fun () -> ()) "Close")
      ()
    |> ok
  in
  let view names = make ~max_width:160. names [ id "a", content ] () |> ok in
  let actual =
    List.map
      [ Some (view [ "a"; "b" ])
      ; Some (view [ "b"; "a" ])
      ; Some
          (View.tab_bar
             ~key:(Key.of_string_exn "tabs")
             ~config:(config [ "b"; "a" ])
             ~on_select:(fun _ -> ())
             ())
      ; None
      ]
      ~f:(fun view ->
        let update = Reconciler.prepare r ~theme:Theme.default view |> ok in
        Reconciler.accept r update |> ok;
        let message = Reconciler.message update |> Option.value_exn in
        let encoded = W.Message.encode message |> ok in
        let hex =
          String.to_list encoded
          |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
          |> String.concat
        in
        hex)
  in
  let expected =
    Eio_main.run (fun env ->
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "tab-content-transactions.hex")
      |> String.split_lines)
  in
  assert (List.equal String.equal expected actual);
  print_endline "exact public create, reorder, reset and disposal transactions";
  [%expect {| exact public create, reorder, reset and disposal transactions |}]
;;
