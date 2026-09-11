open Core
open Gpuio
open Gpuio_protocol

let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
let prepare t view = Reconciler.prepare t ~theme:Theme.default view |> Or_error.ok_exn

let ops update =
  match Reconciler.message update with
  | Some (Wire.Message.Apply tx) -> tx.operations
  | None -> []
  | Some _ -> assert false
;;

let accept t update = Reconciler.accept t update |> Or_error.ok_exn

let commit t view =
  let update = prepare t view in
  accept t update;
  update
;;

let button key action =
  View.button ~key:(Key.of_string_exn key) ~on_click:(fun () -> action) "Click"
;;

let binding update =
  List.find_map_exn (ops update) ~f:(function
    | Wire.Op.Create (node, Button, _, Some handler) -> Some (node, handler)
    | _ -> None)
;;

let event (node, handler) revision = Wire.Event.Press (window, node, handler, revision)

let%expect_test
    "callbacks refresh without a native diff; removed and closed bindings cannot fire"
  =
  let t = Reconciler.create window in
  let initial = commit t (Some (button "action" "first")) in
  let original = binding initial in
  let update = prepare t (Some (button "action" "latest")) in
  assert (List.is_empty (ops update));
  print_s [%sexp (Reconciler.dispatch t (event original 1L) : string option)];
  accept t update;
  print_s [%sexp (Reconciler.dispatch t (event original 1L) : string option)];
  ignore (commit t None : string Reconciler.update);
  assert (Option.is_none (Reconciler.dispatch t (event original 1L)));
  let replacement = commit t (Some (button "action" "replacement")) in
  let next = binding replacement in
  assert (not (Node_id.equal (fst original) (fst next)));
  assert (Option.is_none (Reconciler.dispatch t (event original 1L)));
  print_s [%sexp (Reconciler.dispatch t (event next 3L) : string option)];
  Reconciler.close t;
  assert (Option.is_none (Reconciler.dispatch t (event next 3L)));
  assert (Result.is_error (Reconciler.prepare t ~theme:Theme.default None));
  [%expect
    {| (first)
 (latest)
 (replacement) |}]
;;

let%expect_test
    "key reordering preserves identities and replacement invalidates the old event"
  =
  let t = Reconciler.create window in
  let a = button "a" "a"
  and b = button "b" "b" in
  let initial = commit t (Some (View.column [ a; b ])) in
  let original = binding initial in
  let reordered = commit t (Some (View.column [ b; a ])) in
  assert (
    List.for_all (ops reordered) ~f:(function
      | Wire.Op.Splice _ -> true
      | _ -> false));
  print_s [%sexp (Reconciler.dispatch t (event original 1L) : string option)];
  let replaced = commit t (Some (View.column [ b; button "c" "c" ])) in
  assert (
    List.exists (ops replaced) ~f:(function
      | Wire.Op.Remove node -> Node_id.equal node (fst original)
      | _ -> false));
  assert (Option.is_none (Reconciler.dispatch t (event original 1L)));
  print_s [%sexp (Reconciler.dispatch t (event (binding replaced) 3L) : string option)];
  [%expect
    {| (a)
 (c) |}]
;;

let%expect_test
    "discarded, invalid, stale and foreign plans do not consume identity or commit state"
  =
  let t = Reconciler.create window in
  let view = View.column [ button "x" "x"; button "x" "duplicate" ] in
  assert (Result.is_error (Reconciler.prepare t ~theme:Theme.default (Some view)));
  let discarded = prepare t (Some (button "x" "discard")) in
  let accepted = prepare t (Some (button "x" "accepted")) in
  assert (Node_id.equal (fst (binding discarded)) (fst (binding accepted)));
  let other = Reconciler.create window in
  assert (Result.is_error (Reconciler.accept other accepted));
  accept t accepted;
  assert (Result.is_error (Reconciler.accept t discarded));
  assert (Result.is_error (Reconciler.accept t accepted));
  print_s [%sexp (Reconciler.dispatch t (event (binding accepted) 1L) : string option)];
  [%expect {| (accepted) |}]
;;

let%expect_test "only the changed child range crosses the bridge" =
  let t = Reconciler.create window in
  let row i = View.text ~key:(Key.of_int i) (String.make 256 'x') in
  let children = List.init 500 ~f:row in
  ignore (commit t (Some (View.column children)) : unit Reconciler.update);
  let update = prepare t (Some (View.column (children @ [ row 500 ]))) in
  let inserted =
    List.filter_map (ops update) ~f:(function
      | Wire.Op.Splice (_, offset, removed, insert) ->
        Some (offset, removed, List.length insert)
      | _ -> None)
  in
  print_s [%sexp (inserted : (int64 * int64 * int) list)];
  assert (List.length (ops update) <= 3);
  let message = Reconciler.message update |> Option.value_exn in
  assert (Wire.Message.bin_size_t message < 1024);
  [%expect {| ((500 0 1)) |}]
;;

let%expect_test
    "style composition expands shorthand, resets overrides and resolves themes"
  =
  let base =
    Style.create_exn
      [ Padding (Length.px_exn 8.)
      ; Width (Length.px_exn 100.)
      ; Foreground (Color.token_exn "foreground")
      ]
  in
  let override =
    Style.create_exn [ Padding_left (Length.px_exn 20.) ] |> fun t -> Style.unset t Width
  in
  let merged = Style.merge [ base; override ] in
  let wire = Style.Expert.to_wire merged ~theme:Theme.default |> Or_error.ok_exn in
  let fields =
    List.concat_map wire ~f:(function
      | Wire.Style.Fields fields -> fields
      | _ -> [])
  in
  assert (
    not
      (List.exists fields ~f:(function
         | Wire.Field.Width _ -> true
         | _ -> false)));
  let padding =
    List.filter_map fields ~f:(function
      | Wire.Field.Padding_top (Px n)
      | Padding_right (Px n)
      | Padding_bottom (Px n)
      | Padding_left (Px n) -> Some n
      | _ -> None)
  in
  print_s [%sexp (padding : float list)];
  assert (Result.is_error (Style.create [ Opacity Float.nan ]));
  assert (Result.is_error (Style.create [ Width (Length.px_exn (-1.)) ]));
  assert (Result.is_error (Style.create [ Padding Length.auto ]));
  assert (Result.is_ok (Style.create [ Margin (Length.px_exn (-5.)) ]));
  assert (
    Result.is_error
      (Style.Expert.to_wire
         (Style.create_exn [ Foreground (Color.token_exn "missing") ])
         ~theme:Theme.default));
  [%expect {| (8 8 8 20) |}]
;;

let%expect_test "theme updates change resolved properties even on shared views" =
  let t = Reconciler.create window in
  let view =
    View.text
      ~style:(Style.create_exn [ Foreground (Color.token_exn "foreground") ])
      "same"
  in
  ignore (commit t (Some view) : unit Reconciler.update);
  let theme = Theme.create [ "foreground", Color.rgb_exn 0xabcdef ] |> Or_error.ok_exn in
  let update = Reconciler.prepare t ~theme (Some view) |> Or_error.ok_exn in
  assert (
    List.for_all (ops update) ~f:(function
      | Wire.Op.Set_style _ -> true
      | _ -> false));
  assert (List.length (ops update) = 1);
  accept t update;
  let unchanged = Reconciler.prepare t ~theme (Some view) |> Or_error.ok_exn in
  assert (List.is_empty (ops unchanged));
  print_endline "THEME_REFRESH_PASS";
  [%expect {| THEME_REFRESH_PASS |}]
;;

let%expect_test "interaction refinements have explicit scope and accessible overrides" =
  assert (Result.is_error (Style.with_state Style.empty Hovered [ Pointer_events false ]));
  assert (Result.is_error (Style.with_state Style.empty Pressed [ User_select true ]));
  let style =
    Style.create_exn [ User_select true; Selection_color (Color.rgb_exn 0x123456) ]
  in
  let style =
    Style.with_state_exn style Focused [ Foreground (Color.rgb_exn 0xffffff) ]
  in
  let style =
    Style.with_state_exn style Hovered [ Foreground (Color.rgb_exn 0xcccccc) ]
  in
  let style =
    Style.with_state_exn style Pressed [ Foreground (Color.rgb_exn 0x999999) ]
  in
  let states =
    Style.Expert.to_wire style ~theme:Theme.default
    |> Or_error.ok_exn
    |> List.filter_map ~f:(function
      | Wire.Style.State (state, _) -> Some state
      | _ -> None)
  in
  print_s [%sexp (states : int64 list)];
  let view = View.button ~accessible_name:"Send prompt" ~on_click:(fun () -> ()) "→" in
  let update = prepare (Reconciler.create window) (Some view) in
  assert (
    List.exists (ops update) ~f:(function
      | Set_style (_, style) ->
        List.exists style ~f:(function
          | Fields fields ->
            List.exists fields ~f:(function
              | Accessible_name name -> String.equal name "Send prompt"
              | _ -> false)
          | _ -> false)
      | _ -> false));
  [%expect {| (1 2 3) |}]
;;
