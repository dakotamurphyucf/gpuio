open Core
open Gpuio
module P = Presentation
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let describe = View.Expert.describe
let style = Style.create_exn
let px = Length.px_exn

let commit t view =
  let update = Reconciler.prepare t ~theme:Theme.default view |> ok in
  Reconciler.accept t update |> ok;
  match Reconciler.message update with
  | Some (Apply { operations; _ }) -> operations
  | None -> []
  | Some _ -> assert false
;;

let fields view =
  Style.Expert.to_wire (describe view).style ~theme:Theme.default
  |> ok
  |> List.concat_map ~f:(function
    | W.Style.Fields fields -> fields
    | State _ -> []
    | _ -> failwith "unexpected legacy encoding")
;;

let%expect_test "separator axis, label and paint refinements preserve retained identity" =
  let t = Reconciler.create window in
  let first = ref true in
  let line_id = ref None in
  let cases = ref 0 in
  List.iter [ P.Appearance.light; P.Appearance.dark ] ~f:(fun appearance ->
    List.iter [ P.Axis.Horizontal; Vertical ] ~f:(fun axis ->
      List.iter [ Style.Border_style.Solid; Dashed ] ~f:(fun pattern ->
        List.iter [ None; Some ""; Some "世界 · مرحبا"; None ] ~f:(fun label ->
          let view = P.Separator.create appearance ~axis ~pattern ?label () in
          let description = describe view in
          let line = List.hd_exn description.children in
          let line_fields = fields line in
          let has field = List.mem line_fields field ~equal:W.Field.equal in
          (match axis with
           | Horizontal ->
             assert (has (Width (Percent 100.)) && has (Height (Px 1.)));
             assert (has (Border_top_width 1.))
           | Vertical ->
             assert (has (Height (Percent 100.)) && has (Width (Px 1.)));
             assert (has (Border_left_width 1.)));
          assert (
            List.length description.children = 1 + Bool.to_int (Option.is_some label));
          Option.iter label ~f:(fun text ->
            assert (
              String.equal (describe (List.nth_exn description.children 1)).text text));
          let operations = commit t (Some view) in
          if !first
          then (
            first := false;
            (* The root and the line have different keys and retained identities. *)
            let created =
              List.filter_map operations ~f:(function
                | W.Op.Create (node, _, _, _) -> Some node
                | _ -> None)
            in
            assert (List.length created = 2);
            line_id := Some (List.nth_exn created 1))
          else
            List.iter operations ~f:(function
              | W.Op.Remove node ->
                assert (
                  not (Gpuio_protocol.Node_id.equal node (Option.value_exn !line_id)))
              | _ -> ());
          assert (List.is_empty (commit t (Some view)));
          incr cases))));
  ignore (commit t None : W.Op.t list);
  print_s
    [%sexp { cases = (!cases : int); line_retained = true; repeat_commit_idle = true }];
  [%expect {| ((cases 32) (line_retained true) (repeat_commit_idle true)) |}]
;;

let%expect_test "root, line and label styles refine independently and reset" =
  let appearance = P.Appearance.light in
  let make ?style ?line_style ?label_style () =
    P.Separator.create
      appearance
      ?style
      ?line_style
      ?label_style
      ~pattern:Dashed
      ~label:"Continue"
      ()
  in
  let custom =
    make
      ~style:(style [ Width (px 200.) ])
      ~line_style:(style [ Border_style Solid; Border_color (Color.rgb_exn 0xff0000) ])
      ~label_style:(style [ Font_size 18.; Padding_top (px 10.) ])
      ()
  in
  let has view field = List.mem (fields view) field ~equal:W.Field.equal in
  let line, label =
    match (describe custom).children with
    | [ line; label ] -> line, label
    | _ -> assert false
  in
  assert (has custom (Width (Px 200.)));
  assert (has line (Width (Percent 100.)) && has line (Border_style 0L));
  assert (has label (Font_size 18.) && has label (Padding_top (Px 10.)));
  let t = Reconciler.create window in
  ignore (commit t (Some custom) : W.Op.t list);
  let reset = make () in
  let operations = commit t (Some reset) in
  List.iter operations ~f:(function
    | W.Op.Create _ | Remove _ -> failwith "style reset remounted separator"
    | _ -> ());
  let line, label =
    match (describe reset).children with
    | [ line; label ] -> line, label
    | _ -> assert false
  in
  assert (has reset (Width (Percent 100.)));
  assert (has line (Border_style 1L));
  assert (has label (Font_size 12.) && has label (Padding_top (Px 4.)));
  let legacy = P.separator appearance ~style:(style [ Height (px 3.) ]) () in
  assert (List.is_empty (describe legacy).children && has legacy (Height (Px 3.)));
  print_endline
    "independent style overrides/reset; legacy helper retains its single rectangle";
  [%expect
    {| independent style overrides/reset; legacy helper retains its single rectangle |}]
;;
