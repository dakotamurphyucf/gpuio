open Core
module Combo = Gpuio.Combobox
module Choice = Gpuio.Choice
module Input = Gpuio.Text_input

let id value = Choice.Id.of_string value |> Or_error.ok_exn

let options =
  List.map
    [ "first", false; "second", true ]
    ~f:(fun (key, disabled) ->
      Choice.create ~id:(id key) ~label:key ~disabled () |> Or_error.ok_exn)
  |> Choice.Collection.create
  |> Or_error.ok_exn
;;

let config ?disabled ?filter ?(selected = Some (id "second")) () =
  Combo.Config.create ~label:"Model" ~options ~selected ?disabled ?filter ()
  |> Or_error.ok_exn
;;

let%expect_test "one combobox configuration keeps input and choices coherent" =
  List.iter [ false; true ] ~f:(fun disabled ->
    let config = config ~disabled () in
    let editor = Combo.Expert.editor_config config |> Input.Expert.config in
    let choices = Combo.Config.choices config in
    print_s
      [%sexp
        (editor.mode : Input.Mode.t)
      , (String.equal editor.label (Choice.Config.label choices) : bool)
      , (editor.disabled : bool)
      , (Choice.Config.is_disabled choices : bool)
      , (editor.submit_on_enter : bool)
      , (Combo.Config.filter config : Combo.Filter.t)]);
  let unfiltered = config ~filter:Unfiltered () in
  print_s [%sexp (Combo.Config.filter unfiltered : Combo.Filter.t)];
  print_s
    [%sexp
      (Combo.Config.create ~label:"" ~options ~selected:None () |> Result.is_error : bool)];
  print_s
    [%sexp
      (Combo.Config.create ~label:"Model" ~options ~selected:(Some (id "gone")) ()
       |> Result.is_error
       : bool)];
  [%expect
    {|
    (Single_line true false false false Substring)
    (Single_line true true true false Substring)
    Unfiltered
    true
    true
    |}]
;;

let snapshot ?(text = "query") ?(composing = false) () =
  let window =
    Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
  in
  let node = Gpuio_protocol.Node_id.create ~slot:2L ~generation:3L |> Or_error.ok_exn in
  let revision = Input.Revision.of_int64 42L |> Or_error.ok_exn in
  let selection =
    Input.Selection.create ~anchor:(String.length text) ~head:0 |> Or_error.ok_exn
  in
  Input.Expert.snapshot
    ~window
    ~node
    ~revision
    ~text
    ~selection
    ~composition:
      (Option.some_if
         composing
         (Input.Selection.create ~anchor:0 ~head:(String.length text) |> Or_error.ok_exn))
    ~focused:true
  |> Or_error.ok_exn
;;

let%expect_test
    "selection intent preserves the exact query and rejects invalid activation"
  =
  let observed = snapshot ~text:"é界" () in
  let chosen =
    Combo.Expert.selection (config ()) ~id:(id "first") ~snapshot:observed
    |> Or_error.ok_exn
  in
  print_s
    [%sexp
      (Choice.Id.to_string (Combo.Selection.id chosen) : string)
    , (Input.Snapshot.equal (Combo.Selection.snapshot chosen) observed : bool)
    , (Input.Revision.to_int64 (Input.Snapshot.revision (Combo.Selection.snapshot chosen))
       : int64)
    , (Option.map
         (Choice.Config.selected (Combo.Config.choices (config ())))
         ~f:Choice.Id.to_string
       : string option)];
  let check config key snapshot =
    print_s
      [%sexp
        (Combo.Expert.selection config ~id:(id key) ~snapshot
         |> Result.map ~f:(fun _ -> ())
         : unit Or_error.t)]
  in
  check (config ()) "second" observed;
  check (config ()) "gone" observed;
  check (config ~disabled:true ()) "first" observed;
  check (config ()) "first" (snapshot ~composing:true ());
  check (config ()) "first" (snapshot ~text:"two\nlines" ());
  [%expect
    {|
    (first true 42 (second))
    (Error "combobox choice is absent or disabled")
    (Error "combobox choice is absent or disabled")
    (Error "combobox choice is absent or disabled")
    (Error "cannot select a combobox choice during composition")
    (Error "single-line input cannot contain newlines")
    |}]
;;
