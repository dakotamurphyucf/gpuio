open Core
open Gpuio
module P = Gpuio_protocol
module W = P.Wire

let ok = Or_error.ok_exn
let id = Choice.Id.of_string

let%expect_test "tab viewport uses typed positive serials and independent Op100 bytes" =
  List.iter [ 0L; -1L ] ~f:(fun serial ->
    assert (Result.is_error (Tab_bar.Reveal_request.create (id "tab-7" |> ok) ~serial)));
  let reveal = Tab_bar.Reveal_request.create (id "tab-7" |> ok) ~serial:1L |> ok in
  let viewport = Tab_bar.Viewport.create ~reveal () in
  let config = Tab_bar.Expert.viewport_to_wire viewport in
  let node = P.Node_id.create ~slot:0L ~generation:1L |> ok in
  let bytes =
    Bin_prot.Utils.bin_dump W.Op.bin_writer_t (Set_tab_viewport (node, Some config))
    |> Bigstring.to_string
  in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  assert (String.equal hex "640001010101057461622d37");
  print_endline "positive serials and exact paired Op100 payload";
  [%expect {| positive serials and exact paired Op100 payload |}]
;;

let%expect_test "viewport changes preserve choice and part owners and reset explicitly" =
  let window = P.Window_id.create ~slot:0L ~generation:1L |> ok in
  let r = Reconciler.create window in
  let config =
    Choice.Config.create
      ~label:"Tabs"
      ~options:
        (Choice.Collection.create [ Choice.create ~id:(id "a" |> ok) ~label:"A" () |> ok ]
         |> ok)
      ~selected:None
      ()
    |> ok
  in
  let part =
    View.Tab_content.create ~suffix:(View.button ~on_click:(fun () -> ()) "Close") ()
    |> ok
  in
  let view viewport =
    View.tab_bar_with_content
      ?viewport
      ~config
      ~content:[ id "a" |> ok, part ]
      ~on_select:(fun _ -> ())
      ()
    |> ok
  in
  let commit viewport =
    let update = Reconciler.prepare r ~theme:Theme.default (Some (view viewport)) |> ok in
    Reconciler.accept r update |> ok;
    match Reconciler.message update with
    | Some (Apply tx) -> tx.operations
    | None -> []
    | Some _ -> assert false
  in
  ignore (commit None : W.Op.t list);
  List.iter
    [ Some Tab_bar.Viewport.default
    ; Some
        (Tab_bar.Viewport.create
           ~reveal:(Tab_bar.Reveal_request.create (id "removed" |> ok) ~serial:1L |> ok)
           ())
    ; None
    ]
    ~f:(fun viewport ->
      let operations = commit viewport in
      assert (List.length operations = 1);
      assert (
        List.for_all operations ~f:(function
          | W.Op.Set_tab_viewport _ -> true
          | _ -> false));
      assert (List.is_empty (commit viewport)));
  print_endline
    "one viewport operation per change; no remount or choice write; no-op and reset";
  [%expect
    {| one viewport operation per change; no remount or choice write; no-op and reset |}]
;;
