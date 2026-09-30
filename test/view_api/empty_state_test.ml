open Core
open Gpuio
module P = Presentation
module Empty = P.Empty_state
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let key = Key.of_string_exn
let describe = View.Expert.describe

let commit t view =
  let update = Reconciler.prepare t ~theme:Theme.default view |> ok in
  Reconciler.accept t update |> ok;
  match Reconciler.message update with
  | Some (Apply { operations; _ }) -> operations
  | None -> []
  | Some _ -> assert false
;;

let rec text view =
  let d = describe view in
  (if View.Expert.Kind.equal d.kind Text then [ d.text ] else [])
  @ List.concat_map d.children ~f:text
;;

let enabled mask bit = mask land (1 lsl bit) <> 0

let%expect_test "rich empty slots have deliberate order and no absent placeholders" =
  List.iter [ P.Appearance.light; P.Appearance.dark ] ~f:(fun p ->
    for mask = 0 to 63 do
      let present bit name = Option.some_if (enabled mask bit) (View.text name) in
      let header =
        Empty.header
          ?description:(present 2 "description")
          ?title:(present 1 "title")
          ?media:(present 0 "media")
          ()
      in
      let view =
        Empty.create
          p
          ?content:(present 4 "content")
          ?header:(Option.some_if (enabled mask 3) header)
          (Option.to_list (present 5 "extra"))
      in
      let expected =
        List.filter_opt
          [ Option.some_if (enabled mask 3 && enabled mask 0) "media"
          ; Option.some_if (enabled mask 3 && enabled mask 1) "title"
          ; Option.some_if (enabled mask 3 && enabled mask 2) "description"
          ; Option.some_if (enabled mask 4) "content"
          ; Option.some_if (enabled mask 5) "extra"
          ]
      in
      assert (List.equal String.equal (text view) expected);
      assert (List.length (describe header).children = Int.popcount (mask land 7));
      assert (
        List.length (describe view).children
        = Bool.to_int (enabled mask 3)
          + Bool.to_int (enabled mask 4)
          + Bool.to_int (enabled mask 5));
      let semantics =
        (describe view).accessibility |> Option.value_exn |> Accessibility.Expert.to_wire
      in
      assert (Gpuio_protocol.Accessibility_wire.Live.equal semantics.live Off);
      let t = Reconciler.create window in
      ignore (commit t (Some view) : W.Op.t list);
      assert (List.is_empty (commit t (Some view)))
    done);
  print_endline
    "64 combinations in light/dark; media/title/description/content/extras order; no \
     live region; unchanged views idle";
  [%expect
    {| 64 combinations in light/dark; media/title/description/content/extras order; no live region; unchanged views idle |}]
;;

module Action = struct
  type t =
    { name : string
    ; revision : int
    }
  [@@deriving equal, sexp_of]
end

let%expect_test "rich controls survive neighboring slots, appearance and media changes" =
  let t = Reconciler.create window in
  let previous = ref [] in
  let retired = ref [] in
  List.iteri [ P.Appearance.light; P.Appearance.dark ] ~f:(fun theme p ->
    List.iter [ Empty.Media_variant.Unframed; Icon ] ~f:(fun variant ->
      for mask = 0 to 15 do
        let revision = (theme * 16) + mask in
        let control name =
          View.button ~key:(key name) ~on_click:(fun () -> { Action.name; revision }) name
        in
        let wanted =
          [ "content" ]
          @ List.filter_mapi
              [ "media"; "title"; "description"; "header" ]
              ~f:(fun bit name -> Option.some_if (enabled mask bit) name)
        in
        let view =
          Empty.create
            p
            ~header:
              (Empty.header
                 ?media:
                   (Option.some_if
                      (enabled mask 0)
                      (Empty.media p ~variant [ control "media" ]))
                 ?title:
                   (Option.some_if (enabled mask 1) (Empty.title [ control "title" ]))
                 ?description:
                   (Option.some_if
                      (enabled mask 2)
                      (Empty.description p [ control "description" ]))
                 ())
            ~content:
              (Empty.content
                 [ View.checkbox
                     ~key:(key "content")
                     ~state:Checked
                     ~on_toggle:(fun () -> { Action.name = "content"; revision })
                     "content"
                 ])
            (* A user key matching a named slot is isolated in the extras wrapper. *)
            (Option.to_list (Option.some_if (enabled mask 3) (control "header")))
        in
        let operations = commit t (Some view) in
        let survivors, removed =
          List.partition_tf !previous ~f:(fun (_, _, name) ->
            List.mem wanted name ~equal:String.equal)
        in
        retired := removed @ !retired;
        List.iter survivors ~f:(fun (node, handler, name) ->
          assert (
            Option.equal
              Action.equal
              (Reconciler.dispatch t (W.Event.Press (window, node, handler, 1L)))
              (Some { Action.name; revision }));
          List.iter operations ~f:(function
            | W.Op.Remove other -> assert (not (Gpuio_protocol.Node_id.equal node other))
            | Set_control (other, _) ->
              assert (not (Gpuio_protocol.Node_id.equal node other))
            | _ -> ()));
        List.iter !retired ~f:(fun (node, handler, _) ->
          assert (
            Option.is_none
              (Reconciler.dispatch t (W.Event.Press (window, node, handler, 1L)))));
        let added =
          List.filter_map operations ~f:(function
            | W.Op.Create (node, (Button | Checkbox), _, Some handler) ->
              let action =
                Reconciler.dispatch t (W.Event.Press (window, node, handler, 1L))
                |> Option.value_exn
              in
              assert (
                not
                  (List.exists survivors ~f:(fun (_, _, name) ->
                     String.equal name action.name)));
              Some (node, handler, action.name)
            | _ -> None)
        in
        previous := survivors @ added;
        assert (List.length !previous = List.length wanted);
        assert (List.is_empty (commit t (Some view)))
      done));
  ignore (commit t None : W.Op.t list);
  List.iter (!previous @ !retired) ~f:(fun (node, handler, _) ->
    assert (
      Option.is_none (Reconciler.dispatch t (W.Event.Press (window, node, handler, 1L)))));
  print_endline
    "checked content and rich actions retained; latest callbacks; optional actions \
     retired; user keys isolated; full unmount fenced";
  [%expect
    {| checked content and rich actions retained; latest callbacks; optional actions retired; user keys isolated; full unmount fenced |}]
;;

let fields styles =
  List.concat_map styles ~f:(function
    | W.Style.Fields fields -> fields
    | State _ -> []
    | _ -> failwith "unexpected legacy style encoding")
;;

let view_fields view =
  Style.Expert.to_wire (describe view).style ~theme:Theme.default |> ok |> fields
;;

let%expect_test "media and slot style refinements reset without replacing content" =
  let t = Reconciler.create window in
  let appearance = P.Appearance.light in
  let original = Empty.media appearance ~variant:Icon [ View.text "世界 👩🏽‍💻" ] in
  let custom = Style.create_exn [ Width (Length.px_exn 48.); Font_size 20. ] in
  let refined = Empty.media appearance ~variant:Icon ~style:custom [ View.text "世界 👩🏽‍💻" ] in
  let has view field = List.mem (view_fields view) field ~equal:W.Field.equal in
  assert (has original (Width (Px 32.)));
  assert (has refined (Width (Px 48.)) && not (has refined (Width (Px 32.))));
  ignore (commit t (Some refined) : W.Op.t list);
  let reset = Empty.media appearance [ View.text "世界 👩🏽‍💻" ] in
  let operations = commit t (Some reset) in
  List.iter operations ~f:(function
    | W.Op.Create _ | Remove _ -> failwith "media variant replaced content"
    | _ -> ());
  let styles =
    List.find_map_exn operations ~f:(function
      | W.Op.Set_style (_, styles) -> Some styles
      | _ -> None)
    |> fields
  in
  assert (
    not
      (List.exists styles ~f:(function
         | Width _ | Height _ | Background _ | Font_size _ -> true
         | _ -> false)));
  let header =
    Empty.header
      ~style:(Style.create_exn [ Max_width (Length.px_exn 200.); Align_items Start ])
      ()
  in
  assert (has header (Max_width (Px 200.)));
  assert (not (has header (Max_width (Px 384.))));
  List.iter
    [ ""; "العربية · 日本語 · 👩🏽‍💻"; String.make 10000 'x' ]
    ~f:(fun value ->
      let view =
        Empty.create
          appearance
          ~header:
            (Empty.header
               ~title:(Empty.title [ View.text value ])
               ~description:(Empty.description appearance [ View.text value ])
               ())
          []
      in
      assert (List.equal String.equal (text view) [ value; value ]);
      ignore (commit (Reconciler.create window) (Some view) : W.Op.t list));
  print_endline
    "custom width overrides defaults; unframed reset removes frame/font; slots refine \
     independently; localized and long text preserved";
  [%expect
    {| custom width overrides defaults; unframed reset removes frame/font; slots refine independently; localized and long text preserved |}]
;;
