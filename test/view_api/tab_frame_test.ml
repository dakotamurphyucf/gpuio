open Core
open Gpuio
module P = Gpuio_protocol
module W = P.Wire

let ok = Or_error.ok_exn
let id name = Choice.Id.of_string name |> ok
let style fields = Style.create_exn fields
let px = Length.px_exn

let config names =
  Choice.Config.create
    ~label:"Framed tabs"
    ~options:
      (List.map names ~f:(fun name -> Choice.create ~id:(id name) ~label:name () |> ok)
       |> Choice.Collection.create
       |> ok)
    ~selected:(Some (id "A"))
    ()
  |> ok
;;

let button name width =
  View.button
    ~style:(style [ Width (px width); Shrink 0. ])
    ~on_click:(fun () -> name)
    name
;;

type presentation =
  | Plain
  | Decorated
  | Structured

let framed ?reveal ?(prefix = true) ?(presentation = Plain) names =
  let key = Key.of_string_exn "frame" in
  let appearance =
    Tab_bar.Appearance.create ~tab_style:(style [ Width (px 100.) ]) () |> ok
  in
  let viewport = Tab_bar.Viewport.create ?reveal () in
  let config = config names in
  let on_select = Choice.Id.to_string in
  let tabs =
    match presentation with
    | Plain -> View.tab_bar ~key ~appearance ~viewport ~config ~on_select ()
    | Decorated ->
      View.tab_bar_with_labels
        ~key
        ~appearance
        ~viewport
        ~config
        ~on_select
        ~labels:[ id "A", View.text "Decorated A" ]
        ()
      |> ok
    | Structured ->
      let content =
        View.Tab_content.create
          ~label:(Custom (View.text "Decorated A"))
          ~suffix:(button "Close A" 22.)
          ()
        |> ok
      in
      View.tab_bar_with_content
        ~key
        ~appearance
        ~viewport
        ~config
        ~on_select
        ~content:[ id "A", content ]
        ()
      |> ok
  in
  View.tab_bar_frame
    ~style:(style [ Width (px 420.); Height (px 40.); Gap (px 10.) ])
    ?prefix:(if prefix then Some (button "Prefix" 30.) else None)
    ~suffix:(button "Suffix" 40.)
    ~trailing:(button "Trailing" 80.)
    tabs
  |> ok
;;

let%expect_test "tab frames require direct tabs and use disjoint structural keys" =
  assert (Result.is_error (View.tab_bar_frame (View.text "Not tabs")));
  assert (Result.is_error (View.tab_bar_frame (framed [ "A" ])));
  let rec nested depth view =
    if depth = 0 then view else nested (depth - 1) (View.column [ view ])
  in
  let tabs = View.tab_bar ~config:(config [ "A" ]) ~on_select:Choice.Id.to_string () in
  assert (
    Result.is_error (View.tab_bar_frame ~prefix:(nested 128 (View.text "deep")) tabs));
  let default = View.tab_bar_frame ~suffix:(button "Suffix" 40.) tabs |> ok in
  let viewport = List.nth_exn (View.Expert.describe default).children 1 in
  let viewport = View.Expert.describe viewport in
  assert (Option.is_some (Option.value_exn viewport.choice).tab_viewport);
  let trailing = List.last_exn viewport.children |> View.Expert.describe in
  assert (List.length trailing.children = 1);
  let window = P.Window_id.create ~slot:0L ~generation:1L |> ok in
  let r = Reconciler.create window in
  let view = framed [ "A"; "trailing"; "viewport"; "prefix"; "suffix" ] in
  let update = Reconciler.prepare r ~theme:Theme.default (Some view) |> ok in
  Reconciler.accept r update |> ok;
  let no_change = Reconciler.prepare r ~theme:Theme.default (Some view) |> ok in
  assert (Option.is_none (Reconciler.message no_change));
  let n = P.Node_id.create ~slot:0L ~generation:1L |> ok in
  let bytes =
    Bin_prot.Utils.bin_dump W.Op.bin_writer_t (Set_tab_trailing (n, true))
    |> Bigstring.to_string
    |> String.to_list
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  assert (String.equal bytes "65000101");
  print_endline
    "checked frame and depth; default viewport/spacing; disjoint keys; no-op; Op101";
  [%expect
    {| checked frame and depth; default viewport/spacing; disjoint keys; no-op; Op101 |}]
;;

let%expect_test "public tab frame transactions preserve the viewport and control owners" =
  let window = P.Window_id.create ~slot:0L ~generation:1L |> ok in
  let r = Reconciler.create window in
  let names = [ "A"; "B"; "C"; "D"; "E"; "F" ] in
  let actual =
    List.map
      [ Some (framed names)
      ; Some
          (framed
             ~reveal:(Tab_bar.Reveal_request.create (id "A") ~serial:1L |> ok)
             (List.rev names))
      ; Some (framed ~prefix:false (List.rev names))
      ; Some (framed ~prefix:false ~presentation:Decorated (List.rev names))
      ; Some (framed ~prefix:false ~presentation:Structured (List.rev names))
      ; None
      ]
      ~f:(fun view ->
        let update = Reconciler.prepare r ~theme:Theme.default view |> ok in
        Reconciler.accept r update |> ok;
        let message = Reconciler.message update |> Option.value_exn in
        let encoded = W.Message.encode message |> ok in
        String.to_list encoded
        |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
        |> String.concat)
  in
  let expected =
    Eio_main.run (fun env ->
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "tab-frame-transactions.hex")
      |> String.split_lines)
  in
  assert (List.equal String.equal expected actual);
  print_endline "public frame create, reorder/reveal, prefix removal and disposal";
  [%expect {| public frame create, reorder/reveal, prefix removal and disposal |}]
;;
