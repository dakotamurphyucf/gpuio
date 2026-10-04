open Core
open Gpuio
module P = Gpuio_protocol
module W = P.Wire

let ok = Or_error.ok_exn
let id n = Choice.Id.of_string (if n = 0 then " " else Int.to_string n) |> ok
let style = Style.create_exn
let px = Length.px_exn

let framed ?(menu = true) ?(menu_icons = []) ?(changed = false) ?(disabled = false) () =
  let items =
    List.init 40 ~f:(fun n ->
      Choice.create
        ~id:(id n)
        ~label:(sprintf "%s %02d" (if changed then "Renamed" else "Document") n)
        ~disabled:(n = 1 || (changed && n = 39))
        ()
      |> ok)
  in
  let config =
    Choice.Config.create
      ~label:"Workspace tabs"
      ~options:(Choice.Collection.create (if changed then List.rev items else items) |> ok)
      ~selected:(Some (id 0))
      ~disabled
      ()
    |> ok
  in
  View.tab_bar
    ~key:(Key.of_string_exn "workspace")
    ~appearance:(Tab_bar.Appearance.create ~tab_style:(style [ Width (px 100.) ]) () |> ok)
    ~config
    ~on_select:Choice.Id.to_string
    ()
  |> View.tab_bar_frame
       ?menu:
         (if menu then Some (Tab_bar.Menu.create ~icons:menu_icons () |> ok) else None)
       ~style:(style [ Width (px 420.); Height (px 40.) ])
       ~suffix:(View.button ~on_click:(fun () -> "suffix") "Suffix")
  |> ok
;;

let steps =
  [ Some (framed ~menu:false ())
  ; Some (framed ())
  ; Some (framed ~changed:true ())
  ; Some (framed ~changed:true ~disabled:true ())
  ; Some (framed ~changed:true ())
  ; Some (framed ~changed:true ~menu:false ())
  ; None
  ]
;;

let%expect_test "all-tabs menu validates names and routes against current choices" =
  List.iter
    [ ""; "bad\000name"; String.make 1025 'x'; "\255" ]
    ~f:(fun label -> assert (Result.is_error (Tab_bar.Menu.create ~label ())));
  let window = P.Window_id.create ~slot:0L ~generation:1L |> ok in
  let r = Reconciler.create window in
  let apply view =
    let update = Reconciler.prepare r ~theme:Theme.default view |> ok in
    Reconciler.accept r update |> ok;
    Reconciler.message update
  in
  let message = apply (Some (framed ())) |> Option.value_exn in
  let node, handler =
    match message with
    | Apply tx ->
      List.find_map_exn tx.operations ~f:(function
        | Create (node, Select, _, Some handler) -> Some (node, handler)
        | _ -> None)
    | _ -> assert false
  in
  let event n = W.Event.Choice (window, node, handler, 1L, Choice.Id.to_string (id n)) in
  assert (Option.equal String.equal (Reconciler.dispatch r (event 0)) (Some " "));
  assert (Option.is_none (Reconciler.dispatch r (event 1)));
  assert (Option.is_none (apply (Some (framed ()))));
  ignore (apply (Some (framed ~changed:true ())) : W.Message.t option);
  assert (Option.is_none (Reconciler.dispatch r (event 39)));
  assert (Option.equal String.equal (Reconciler.dispatch r (event 2)) (Some "2"));
  ignore (apply (Some (framed ~disabled:true ())) : W.Message.t option);
  assert (Option.is_none (Reconciler.dispatch r (event 2)));
  ignore (apply (Some (framed ~menu:false ())) : W.Message.t option);
  assert (Option.is_none (Reconciler.dispatch r (event 0)));
  let bytes =
    Bin_prot.Utils.bin_dump W.Op.bin_writer_t (Set_choice_menu (node, true))
    |> Bigstring.to_string
  in
  assert (Char.to_int bytes.[0] = 102);
  print_endline
    "validated menu; whitespace IDs; no-op; current disabled/removal fencing; Op102";
  [%expect
    {| validated menu; whitespace IDs; no-op; current disabled/removal fencing; Op102 |}]
;;

let%expect_test "public all-tabs menu transactions" =
  let window = P.Window_id.create ~slot:0L ~generation:1L |> ok in
  let r = Reconciler.create window in
  let actual =
    List.map steps ~f:(fun view ->
      let update = Reconciler.prepare r ~theme:Theme.default view |> ok in
      Reconciler.accept r update |> ok;
      let encoded =
        Reconciler.message update |> Option.value_exn |> W.Message.encode |> ok
      in
      String.to_list encoded
      |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
      |> String.concat)
  in
  let expected =
    Eio_main.run (fun env ->
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "tab-menu-transactions.hex")
      |> String.split_lines)
  in
  assert (List.equal String.equal expected actual);
  print_endline
    "public menu insertion, reorder/rename/disable, re-enable, removal and disposal";
  [%expect
    {| public menu insertion, reorder/rename/disable, re-enable, removal and disposal |}]
;;
