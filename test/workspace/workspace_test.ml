open Core
module W = Gpuio.Workspace

let id value = W.Id.of_string value |> Or_error.ok_exn
let tab name draft = W.Tab.create ~id:(id name) ~label:name draft |> Or_error.ok_exn

let print t =
  print_s
    [%sexp
      (( List.map (W.tabs t) ~f:(fun tab -> W.Id.to_string (W.Tab.id tab), W.Tab.data tab)
       , Option.map (W.active t) ~f:(fun tab -> W.Id.to_string (W.Tab.id tab)) )
       : (string * string) list * string option)]
;;

let%expect_test "switch, reorder and close retain draft payloads with stable identity" =
  let original =
    W.create [ tab "one" "draft λ"; tab "two" "second"; tab "three" "third" ]
    |> Or_error.ok_exn
  in
  let selected = W.select original (id "two") |> Or_error.ok_exn in
  let reordered = W.move selected (id "two") ~index:0 |> Or_error.ok_exn in
  let edited = W.update reordered (id "one") ~f:(fun _ -> "new 👨‍👩‍👧‍👦") |> Or_error.ok_exn in
  print edited;
  print (W.next edited);
  print (W.previous edited);
  let remaining, removed = W.remove edited (id "two") |> Or_error.ok_exn in
  print remaining;
  print_s [%sexp (W.Tab.data removed : string)];
  print original;
  [%expect
    {|
   (((two second)
     (one
      "new \240\159\145\168\226\128\141\240\159\145\169\226\128\141\240\159\145\167\226\128\141\240\159\145\166")
     (three third))
    (two))
   (((two second)
     (one
      "new \240\159\145\168\226\128\141\240\159\145\169\226\128\141\240\159\145\167\226\128\141\240\159\145\166")
     (three third))
    (one))
   (((two second)
     (one
      "new \240\159\145\168\226\128\141\240\159\145\169\226\128\141\240\159\145\167\226\128\141\240\159\145\166")
     (three third))
    (three))
   (((one
      "new \240\159\145\168\226\128\141\240\159\145\169\226\128\141\240\159\145\167\226\128\141\240\159\145\166")
     (three third))
    (one))
   second
   (((one "draft \206\187") (two second) (three third)) (one))
   |}]
;;

let%expect_test "last-tab removal is empty and IDs/limits are validated" =
  let single = W.create [ tab "one" "draft" ] |> Or_error.ok_exn in
  let empty, _ = W.remove single (id "one") |> Or_error.ok_exn in
  print empty;
  print (W.next empty);
  List.iter
    [ W.create [ tab "same" "a"; tab "same" "b" ]
    ; W.create ~active:(id "absent") [ tab "one" "a" ]
    ; W.move single (id "one") ~index:(-1)
    ; W.create (List.init (W.max_tabs + 1) ~f:(fun i -> tab (Int.to_string i) ""))
    ]
    ~f:(fun result -> print_s [%sexp (Result.is_error result : bool)]);
  [%expect
    {|
   (() ())
   (() ())
   true
   true
   true
   true
   |}]
;;

let%expect_test
    "switching retained panels sends styles and choice updates without remounting"
  =
  let module V = Gpuio.View in
  let module R = Gpuio.Reconciler in
  let module Wire = Gpuio_protocol.Wire in
  let window =
    Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
  in
  let reconciler = R.create window in
  let state =
    W.create [ tab "one" "draft one"; tab "two" "draft two" ] |> Or_error.ok_exn
  in
  let view state =
    let selected = Option.map (W.active state) ~f:W.Tab.id in
    V.column
      (V.tab_bar
         ~key:(Gpuio.Key.of_string_exn "tabs")
         ~config:(W.choices state ~label:"Conversations" |> Or_error.ok_exn)
         ~on_select:Fn.id
         ()
       :: List.map (W.tabs state) ~f:(fun tab ->
         V.tab_panel
           ~key:(Gpuio.Key.of_string_exn (W.Id.to_string (W.Tab.id tab)))
           ~label:(W.Tab.label tab)
           ~active:(Option.equal W.Id.equal selected (Some (W.Tab.id tab)))
           [ V.text (W.Tab.data tab) ]))
  in
  let prepare state =
    R.prepare reconciler ~theme:Gpuio.Theme.default (Some (view state)) |> Or_error.ok_exn
  in
  let first = prepare state in
  R.accept reconciler first |> Or_error.ok_exn;
  let second = prepare (W.next state) in
  (match R.message second with
   | Some (Wire.Message.Apply tx) ->
     let remounts =
       List.count tx.operations ~f:(function
         | Wire.Op.Create _ | Remove _ -> true
         | _ -> false)
     in
     let styles =
       List.count tx.operations ~f:(function
         | Wire.Op.Set_style _ -> true
         | _ -> false)
     in
     let choices =
       List.count tx.operations ~f:(function
         | Wire.Op.Set_choice _ -> true
         | _ -> false)
     in
     print_s [%sexp (remounts : int), (styles : int), (choices : int)]
   | _ -> failwith "expected selection/visibility delta");
  [%expect {| (0 2 1) |}]
;;
