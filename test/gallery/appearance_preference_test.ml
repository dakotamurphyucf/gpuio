open Core
module A = Gpuio_gallery_model.Appearance
module N = Gpuio.Window.Appearance

let%expect_test
    "system follows native appearance while explicit palettes remain independent"
  =
  List.iter
    A.Preference.[ System; Explicit Light; Explicit Dark ]
    ~f:(fun preference ->
      let resolved =
        List.map
          [ None
          ; Some N.Light
          ; Some Vibrant_light
          ; Some Dark
          ; Some Vibrant_dark
          ; Some Light
          ]
          ~f:(fun native -> A.Preference.resolve preference ~native)
      in
      print_s [%sexp (preference : A.Preference.t), (resolved : A.t list)]);
  [%expect
    {|
    (System (Dark Light Light Dark Dark Light))
    ((Explicit Light) (Light Light Light Light Light Light))
    ((Explicit Dark) (Dark Dark Dark Dark Dark Dark))
  |}]
;;

let%expect_test "changing palette retains editor mount and emits no draft replacement" =
  let open Gpuio in
  let module W = Gpuio_protocol.Wire in
  let ok = Or_error.ok_exn in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create window in
  let controller = Key.of_string "appearance-editor" |> ok in
  let config = Text_input.Config.create ~mode:Single_line ~label:"Draft" () |> ok in
  let view =
    View.text_input
      ~style:(Style.create_exn [ Foreground (Color.token_exn "foreground") ])
      ~controller
      ~config
      ~initial_text:"draft λ"
      ~on_event:(fun _ -> ())
      ()
    |> ok
  in
  let commit color =
    let theme =
      Theme.create
        [ "foreground", Color.rgb_exn color
        ; "background", Color.rgb_exn 0x10151d
        ; "accent", Color.rgb_exn 0x89ddc9
        ; "muted", Color.rgb_exn 0x98a7bd
        ]
      |> ok
    in
    let update = Reconciler.prepare reconciler ~theme (Some view) |> ok in
    Reconciler.accept reconciler update |> ok;
    match Reconciler.message update with
    | None -> []
    | Some (W.Message.Apply tx) -> tx.operations
    | Some _ -> assert false
  in
  let initial = commit 0xffffff in
  assert (
    List.exists initial ~f:(function
      | W.Op.Create (_, Input, _, _) -> true
      | _ -> false));
  List.iter [ 0x222222; 0xffffff; 0x222222 ] ~f:(fun color ->
    let operations = commit color in
    assert (
      List.exists operations ~f:(function
        | W.Op.Set_style _ -> true
        | _ -> false));
    assert (
      not
        (List.exists operations ~f:(function
           | W.Op.Create _ | Remove _ | Set_text _ -> true
           | _ -> false))));
  print_endline "palette changes retain the editor owner and its native draft";
  [%expect {| palette changes retain the editor owner and its native draft |}]
;;
