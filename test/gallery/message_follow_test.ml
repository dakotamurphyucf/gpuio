open Core
open Gpuio
module Follow = Gpuio_gallery_model.Message_follow
module Wire = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let key = Key.of_string_exn

let viewport ~following_tail ~at_end : Virtual_list.Viewport.t =
  { visible_first = 0
  ; visible_last = 1
  ; requested = []
  ; pinned = []
  ; anchor = None
  ; following_tail
  ; at_start = true
  ; at_end
  ; budget_exhausted = false
  }
;;

let%expect_test "unknown, following and short lists do not suggest unread content" =
  assert (not (Follow.is_away None));
  List.iter [ false; true ] ~f:(fun following_tail ->
    List.iter [ false; true ] ~f:(fun at_end ->
      let away = Follow.is_away (Some (viewport ~following_tail ~at_end)) in
      print_s [%sexp (following_tail : bool), (at_end : bool), (away : bool)]));
  [%expect
    {|
    (false false true)
    (false true false)
    (true false false)
    (true true false)
    |}]
;;

let rec find view name =
  let d = View.Expert.describe view in
  if Option.equal Key.equal d.key (Some (key name))
  then Some d
  else List.find_map d.children ~f:(fun child -> find child name)
;;

let fields d =
  Style.Expert.to_wire d.View.Expert.style ~theme:Theme.default
  |> ok
  |> List.concat_map ~f:(function
    | Wire.Style.Fields fields -> fields
    | _ -> [])
;;

let%expect_test
    "presentation updates keep the native list and remove hidden input immediately"
  =
  let list =
    View.virtual_list
      ~key:(key "messages")
      ~config:(Virtual_list.Config.create ~height:(Fixed 32.) () |> ok)
      [ key "row", View.text "A retained message" ]
    |> ok
  in
  let jump = View.button ~key:(key "jump") ~on_click:(fun () -> ()) "Follow latest" in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create window in
  let render ~visible ~jump_enabled ~motion ~fade =
    Follow.view
      ~viewport:(Some (viewport ~following_tail:(not visible) ~at_end:(not visible)))
      ~jump_enabled
      ~motion
      ~fade
      ~jump
      list
  in
  let prior = ref None in
  List.iter
    [ false, true, true, Some (Color.rgb_exn 0xffffff)
    ; true, true, true, Some (Color.rgb_exn 0xffffff)
    ; false, true, true, Some (Color.rgb_exn 0xffffff)
    ; true, false, true, Some (Color.rgb_exn 0xffffff)
    ; true, true, false, None
    ]
    ~f:(fun (visible, jump_enabled, motion, fade) ->
      let view = render ~visible ~jump_enabled ~motion ~fade in
      let shown = visible && jump_enabled in
      let gate = find view "jump-gate" |> Option.value_exn in
      assert (
        List.exists (fields gate) ~f:(function
          | Inert inert -> Bool.equal inert (not shown)
          | _ -> false));
      let wrapper = find view "jump-motion" |> Option.value_exn in
      assert (
        not
          (List.exists (fields wrapper) ~f:(function
             | Inert true -> true
             | _ -> false)));
      let config =
        (Option.value_exn wrapper.animation).config
        |> Animation.Expert.to_wire ~generation:1L
        |> ok
      in
      assert (Int64.equal config.duration_ms (if motion then 200L else 0L));
      assert (
        List.exists config.targets ~f:(fun target ->
          Wire.Animation.Property.equal target.property Opacity
          && Float.equal target.value (if shown then 1. else 0.)));
      let prepared =
        Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok
      in
      (match !prior, Reconciler.message prepared with
       | Some _, Some (Wire.Message.Apply tx) ->
         assert (
           not
             (List.exists tx.operations ~f:(function
                | Create _
                | Remove _
                | Set_list_config _
                | Set_list_order _
                | Set_list_rows _
                | Scroll_list _ -> true
                | _ -> false)))
       | _ -> ());
      Reconciler.accept reconciler prepared |> ok;
      prior := Some view);
  print_endline
    "Show/hide, disabling jump, fade removal and zero motion retain list and controls; \
     hidden input is inert";
  [%expect
    {| Show/hide, disabling jump, fade removal and zero motion retain list and controls; hidden input is inert |}]
;;
