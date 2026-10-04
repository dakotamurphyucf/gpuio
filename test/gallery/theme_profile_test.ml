open Core
module P = Gpuio_gallery_model.Theme_profile
module S = Gpuio_gallery_model.Theme_selection
module A = Gpuio_gallery_model.Appearance

let ok = Or_error.ok_exn

let fixture =
  {|((version 1)(name "Aurora")(appearance Dark)(colors ((background #111320)(surface #1c2033)(foreground #eef1ff)(muted #aab5d6)(accent #bca6ff)(border #39425f)(on_solid #10151d)(success #89ddc9)(warning #ffd08a)(danger #ff9aa8))))|}
;;

let replace text pattern with_ = String.substr_replace_all text ~pattern ~with_

let%expect_test "strict profile parsing and bounds" =
  let profile = P.decode fixture |> ok in
  print_s [%sexp (P.name profile : string), (P.appearance profile : A.t)];
  assert (Gpuio.Color.equal (P.accent profile) (Gpuio.Color.rgb_exn 0xbca6ff));
  List.iter
    [ P.maximum_bytes - 1; P.maximum_bytes; P.maximum_bytes + 1 ]
    ~f:(fun size ->
      let input = fixture ^ String.make (size - String.length fixture) ' ' in
      printf "%d: %b\n" size (Result.is_ok (P.decode input)));
  let invalid =
    [ replace fixture "(version 1)" "(version 2)"
    ; replace fixture "(version 1)" "(version 1)(version 1)"
    ; replace fixture "(version 1)" "(version 1)(surprise true)"
    ; replace fixture "(muted #aab5d6)" ""
    ; replace fixture "(muted #aab5d6)" "(muted #aab5d6)(other #fff)"
    ; fixture ^ ")"
    ; fixture ^ fixture
    ; replace fixture "Aurora" "   "
    ; replace fixture "Aurora" (String.make 129 'a')
    ; fixture ^ "\000"
    ; replace fixture "Aurora" "\\000"
    ; replace fixture "Aurora" "\255"
    ; replace fixture "#bca6ff" "#gggggg"
    ; replace fixture "#bca6ff" "accent"
    ; replace fixture "Dark" "System"
    ; String.make 1000 '(' ^ String.make 1000 ')'
    ]
  in
  List.iteri invalid ~f:(fun index input ->
    if Result.is_ok (P.decode input) then failwithf "Invalid fixture %d accepted" index ());
  printf "%d malformed profiles rejected\n" (List.length invalid);
  [%expect
    {|
    (Aurora Dark)
    16383: true
    16384: true
    16385: false
    16 malformed profiles rejected
  |}]
;;

let%expect_test "last good profile, stale choices and independent windows" =
  let initial = S.initial () in
  let profile = P.decode fixture |> ok in
  let loaded = S.complete initial ~token:(S.token initial) (Ok profile) in
  assert (Option.is_some (S.profile loaded));
  let failed =
    S.complete loaded ~token:(S.token loaded) (Or_error.error_string "bad file")
  in
  assert (Option.equal P.equal (S.profile loaded) (S.profile failed));
  List.iter
    A.Preference.[ Explicit Dark; System ]
    ~f:(fun preference ->
      let newer = S.choose loaded preference in
      let late = S.complete newer ~token:(S.token loaded) (Ok profile) in
      assert (Option.is_none (S.profile late));
      assert (A.Preference.equal (S.preference late) preference));
  let other = S.initial () in
  assert (
    Option.is_none (S.profile (S.complete other ~token:(S.token initial) (Ok profile))));
  let reload = P.decode (replace fixture "#bca6ff" "#88ffcc") |> ok in
  let updated = S.complete loaded ~token:(S.token loaded) (Ok reload) in
  assert (
    Gpuio.Color.equal
      (P.accent (Option.value_exn (S.profile updated)))
      (Gpuio.Color.rgb_exn 0x88ffcc));
  print_endline "last good retained; stale choices ignored; same-mode reload applied";
  [%expect {| last good retained; stale choices ignored; same-mode reload applied |}]
;;

let%expect_test "loaded profile colors update styles without replacing an editor" =
  let open Gpuio in
  let module W = Gpuio_protocol.Wire in
  let reconciler =
    Reconciler.create (Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok)
  in
  let config = Text_input.Config.create ~mode:Single_line ~label:"Theme draft" () |> ok in
  let view =
    View.text_input
      ~controller:(Key.of_string_exn "theme-draft")
      ~config
      ~initial_text:"draft λ"
      ~style:(Style.create_exn [ Foreground (Color.token_exn "foreground") ])
      ~on_event:(fun _ -> ())
      ()
    |> ok
  in
  let commit profile =
    let theme =
      Theme.create
        [ "foreground", P.foreground profile
        ; "background", P.surface profile
        ; "accent", P.accent profile
        ; "muted", P.muted profile
        ]
      |> ok
    in
    let update = Reconciler.prepare reconciler ~theme (Some view) |> ok in
    Reconciler.accept reconciler update |> ok;
    match Reconciler.message update with
    | Some (W.Message.Apply tx) -> tx.operations
    | _ -> []
  in
  let first = P.decode fixture |> ok in
  let changed = P.decode (replace fixture "#eef1ff" "#ffffff") |> ok in
  ignore (commit first : W.Op.t list);
  List.iter [ changed; first ] ~f:(fun profile ->
    let ops = commit profile in
    assert (
      List.exists ops ~f:(function
        | W.Op.Set_style _ -> true
        | _ -> false));
    assert (
      not
        (List.exists ops ~f:(function
           | W.Op.Create _ | Remove _ | Set_text _ -> true
           | _ -> false))));
  print_endline
    "profile reload emits style updates, no editor remount or text replacement";
  [%expect
    {| profile reload emits style updates, no editor remount or text replacement |}]
;;
