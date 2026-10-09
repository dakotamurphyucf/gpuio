open Core
open Gpuio
module P = Gpuio_protocol
module W = P.Wire

let ok = Or_error.ok_exn
let id s = Choice.Id.of_string s |> ok

let hex bytes =
  String.to_list bytes
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let%expect_test "checked motion duration and independent Op103 bytes" =
  List.iter [ -1.; 60_001. ] ~f:(fun ms ->
    assert (
      Result.is_error (Tab_bar.Motion.create ~color_duration:(Time_ns.Span.of_ms ms) ())));
  List.iter
    [ 0., 0L; 0.01, 1L; 60_000., 60_000L ]
    ~f:(fun (ms, expected) ->
      let motion =
        Tab_bar.Motion.create ~color_duration:(Time_ns.Span.of_ms ms) ()
        |> ok
        |> Tab_bar.Expert.motion_to_wire
      in
      assert (Int64.equal motion.color_duration_ms expected));
  let node = P.Node_id.create ~slot:0L ~generation:1L |> ok in
  let config = Tab_bar.Expert.motion_to_wire Tab_bar.Motion.default in
  let payload =
    Bin_prot.Utils.bin_dump W.Op.bin_writer_t (Set_tab_motion (node, Some config))
    |> Bigstring.to_string
    |> hex
  in
  assert (
    String.equal
      payload
      "6700010100000000000079400000000000004440000000000000f03f7b14ae47e17a843ffed007fec800");
  print_endline "bounded rounded duration; exact independent Op103 payload";
  [%expect {| bounded rounded duration; exact independent Op103 payload |}]
;;

let%expect_test "all tab constructors retain owners and update only motion metadata" =
  let config =
    Choice.Config.create
      ~label:"Motion tabs"
      ~options:
        (Choice.Collection.create [ Choice.create ~id:(id "a") ~label:"A" () |> ok ] |> ok)
      ~selected:(Some (id "a"))
      ()
    |> ok
  in
  let on_select _ = () in
  let views =
    [ (fun motion -> View.tab_bar ?motion ~config ~on_select ())
    ; (fun motion ->
        View.tab_bar_with_labels ?motion ~config ~labels:[] ~on_select () |> ok)
    ; (fun motion ->
        View.tab_bar_with_content ?motion ~config ~content:[] ~on_select () |> ok)
    ]
  in
  List.iter views ~f:(fun view ->
    let r = Reconciler.create (P.Window_id.create ~slot:0L ~generation:1L |> ok) in
    let commit motion =
      let update = Reconciler.prepare r ~theme:Theme.default (Some (view motion)) |> ok in
      Reconciler.accept r update |> ok;
      match Reconciler.message update with
      | Some (Apply tx) -> tx.operations
      | None -> []
      | Some _ -> assert false
    in
    ignore (commit None : W.Op.t list);
    List.iter
      [ Some Tab_bar.Motion.default
      ; Some (Tab_bar.Motion.create ~color_duration:Time_ns.Span.zero () |> ok)
      ; None
      ]
      ~f:(fun motion ->
        (match commit motion with
         | [ W.Op.Set_tab_motion (_, actual) ] ->
           assert (
             Option.equal
               W.Tab_motion.equal
               actual
               (Option.map motion ~f:Tab_bar.Expert.motion_to_wire))
         | _ -> assert false);
        assert (List.is_empty (commit motion))));
  print_endline
    "plain, decorated and structured tabs: one metadata change, no remount, no-op and \
     reset";
  [%expect
    {| plain, decorated and structured tabs: one metadata change, no remount, no-op and reset |}]
;;
