open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok

let config =
  Number_input.Config.create
    ~domain:(Numeric.Domain.create ~min:0. ~max:100. ~step:1. |> ok)
    ~label:"Amount"
    ()
  |> ok
;;

let base () =
  View.number_input
    ~controller:(Key.of_string_exn "amount")
    ~config
    ~initial:(Number_input.Value.of_float 12. |> ok)
    ~initial_draft:(Number_input.Draft.of_string "1e-" |> ok)
    ~on_event:(fun _ -> ())
    ()
;;

let commit r theme view =
  let update = Reconciler.prepare r ~theme (Some view) |> ok in
  Reconciler.accept r update |> ok;
  match Reconciler.message update with
  | Some (W.Message.Apply tx) -> tx.operations
  | None -> []
  | Some _ -> assert false
;;

let%expect_test "numeric frame validates parts and retains the editor across role changes"
  =
  let invalid style =
    Result.is_error (Number_input.Appearance.create ~frame_style:style ())
  in
  assert (invalid (Style.create_exn [ Width (Length.px_exn 1.) ]));
  assert (invalid (Style.with_state_exn Style.empty Hovered [ Opacity 0.5 ]));
  List.iter [ Float.nan; Float.infinity; -1.; 257. ] ~f:(fun n ->
    assert (Result.is_error (Number_input.Appearance.create ~button_width:n ()));
    assert (Result.is_error (Number_input.Appearance.create ~gap:n ())));
  let action = View.button ~on_click:Fn.id "Currency" in
  assert (Result.is_error (View.number_frame ~increment:action (base ())));
  assert (Result.is_error (View.number_frame (View.text "not an editor")));
  let appearance =
    Number_input.Appearance.create
      ~frame_style:(Style.create_exn [ Foreground (Color.token_exn "amount") ])
      ()
    |> ok
  in
  let theme color =
    Theme.create
      [ "amount", Color.rgb_exn color
      ; "accent", Color.rgb_exn 0x336699
      ; "foreground", Color.rgb_exn 0xeeeeee
      ]
    |> ok
  in
  let view ?leading ?trailing () =
    View.number_frame ~appearance ?leading ?trailing (base ()) |> ok
  in
  let r = Reconciler.create window in
  let initial = commit r (theme 0x112233) (base ()) in
  let owner =
    List.find_map_exn initial ~f:(function
      | W.Op.Create (id, Number_input, _, _) -> Some id
      | _ -> None)
  in
  let changed =
    commit r (theme 0x112233) (view ~leading:action ~trailing:(View.text "USD") ())
  in
  assert (
    List.exists changed ~f:(function
      | W.Op.Set_number_presentation (id, Some _) -> Gpuio_protocol.Node_id.equal id owner
      | _ -> false));
  assert (
    not
      (List.exists changed ~f:(function
         | W.Op.Set_number_input _ | Set_number_input_draft _ -> true
         | _ -> false)));
  let button =
    List.find_map_exn changed ~f:(function
      | W.Op.Create (id, Button, "Currency", _) -> Some id
      | _ -> None)
  in
  let changes = commit r (theme 0x112233) (view ~leading:action ()) in
  assert (
    not
      (List.exists changes ~f:(function
         | W.Op.Remove id ->
           Gpuio_protocol.Node_id.equal id owner || Gpuio_protocol.Node_id.equal id button
         | _ -> false)));
  (match commit r (theme 0x445566) (view ~leading:action ()) with
   | [ W.Op.Set_number_presentation (id, Some _) ] ->
     assert (Gpuio_protocol.Node_id.equal id owner)
   | _ -> assert false);
  let revision = Reconciler.revision r in
  assert (Result.is_error (Reconciler.prepare r ~theme:Theme.default (Some (view ()))));
  assert (Int64.equal revision (Reconciler.revision r));
  let removed = commit r Theme.default (base ()) in
  assert (
    List.exists removed ~f:(function
      | W.Op.Set_number_presentation (id, None) -> Gpuio_protocol.Node_id.equal id owner
      | _ -> false));
  assert (
    not
      (List.exists removed ~f:(function
         | W.Op.Remove id -> Gpuio_protocol.Node_id.equal id owner
         | _ -> false)));
  print_endline
    "native editor and surviving role identity retained; theme failure atomic; step \
     content passive";
  [%expect
    {| native editor and surviving role identity retained; theme failure atomic; step content passive |}]
;;

let%expect_test "numeric presentation matches independently constructed Op82 bytes" =
  let color n = W.Color.Rgba n in
  let config : W.Number_presentation.t =
    { gap = 4.
    ; button_width = 30.
    ; button_min_height = 22.
    ; stacked_button_min_height = 16.
    ; editor_padding = 3.
    ; border_width = Some 1.
    ; frame_style =
        [ Fields [ Foreground (color 0x11223344L) ]
        ; State (1L, [ Border_color (color 0x66778899L) ])
        ]
    ; editor_style = [ Fields [ Font_size 14. ] ]
    ; decrement_style = [ State (2L, [ Opacity 0.5 ]) ]
    ; increment_style = [ State (3L, [ Foreground (color 0xaabbccddL) ]) ]
    }
  in
  let message =
    W.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_number_presentation (node, Some config)
          ; Set_number_presentation (node, None)
          ]
      }
  in
  let bytes =
    Bin_prot.Utils.bin_dump W.Message.bin_writer_t message |> Bigstring.to_string
  in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  let expected =
    Eio_main.run (fun env ->
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "number-presentation.hex")
      |> String.strip)
  in
  assert (String.equal hex expected);
  print_endline "numeric geometry, themed part states, and reset match independent bytes";
  [%expect {| numeric geometry, themed part states, and reset match independent bytes |}]
;;
