open Core
module M = Gpuio_gallery_model.Settings_state
module S = Gpuio.Settings
module N = Gpuio.Number_input

let ok = Or_error.ok_exn

let apply t actions =
  List.fold actions ~init:t ~f:(fun t action -> M.apply t action |> ok)
;;

let page name = S.Page_id.of_string name |> ok
let query text = S.Query.of_string text |> ok

let%expect_test
    "queued settings intents, disabled custom resets, filters and stable drafts"
  =
  let changed =
    apply
      M.initial
      [ Toggle_notifications
      ; Toggle_notifications
      ; Custom
      ; Name "Aster"
      ; Budget (N.Value.of_float 37. |> ok, N.Draft.of_string "1e-" |> ok)
      ; Model 249
      ; Toggle_feature 47
      ]
  in
  assert (M.notifications changed);
  assert (M.custom changed = 0);
  let filtered = apply changed [ Navigate (Search (query "budget")) ] in
  print_s
    [%sexp (M.reset_targets filtered (Matching_page (page "workspace")) : M.Field.t list)];
  print_s
    [%sexp (M.reset_targets filtered (Whole_page (page "workspace")) : M.Field.t list)];
  let revisited =
    apply
      changed
      [ Navigate (Select (S.Selection.create (page "advanced")))
      ; Navigate (Search (query "no such preference"))
      ; Navigate (Search S.Query.empty)
      ; Navigate (Select (S.Selection.create (page "workspace")))
      ]
  in
  assert (String.equal (M.name revisited) "Aster");
  assert (M.feature revisited 47);
  assert (String.equal (N.Draft.to_string (M.budget_draft revisited)) "1e-");
  let custom = apply changed [ Toggle_lock; Custom; Custom; Toggle_lock; Reset Custom ] in
  assert (M.custom custom = 2);
  assert (not (M.can_reset custom Custom));
  let reset = apply custom [ Toggle_lock; Reset Custom; Reset Budget ] in
  assert (M.custom reset = 0);
  assert (N.Value.equal (M.budget reset) (M.budget M.initial));
  assert (String.equal (N.Draft.to_string (M.budget_draft reset)) "25");
  assert (not (String.is_substring (M.encode changed |> ok) ~substring:"1e-"));
  print_endline
    "values and drafts survive navigation; disabled resets stay inert; exports contain \
     committed values";
  [%expect
    {|
    (Budget)
    (Name Budget Model)
    values and drafts survive navigation; disabled resets stay inert; exports contain committed values
    |}]
;;

let%expect_test "invalid typed choices, numeric values and stale reset destinations" =
  let changed = apply M.initial [ Name "Changed"; Toggle_reports ] in
  let moved =
    apply changed [ Navigate (Select (S.Selection.create (page "advanced"))) ]
  in
  assert (List.is_empty (M.reset_targets moved (Matching_page (page "workspace"))));
  List.iter
    [ M.Action.Region (-1)
    ; Region 3
    ; Model 250
    ; Toggle_feature 48
    ; Budget (N.Value.of_float 1001. |> ok, M.budget_draft M.initial)
    ; Budget (N.Value.empty, M.budget_draft M.initial)
    ; Name "bad\nname"
    ]
    ~f:(fun action -> assert (Or_error.is_error (M.apply changed action)));
  let fields =
    List.concat_map
      (S.pages (M.catalog changed))
      ~f:(fun page -> List.concat_map (S.Page.groups page) ~f:S.Group.items)
  in
  List.iter fields ~f:(fun item ->
    let field = M.Field.of_id (S.Item.id item) |> Option.value_exn in
    assert (S.Item_id.equal (M.Field.id field) (S.Item.id item)));
  assert (List.length fields = 56);
  let empty_name = apply changed [ Name "   " ] in
  assert (Or_error.is_error (M.encode empty_name));
  assert (String.equal (M.name empty_name) "   ");
  print_endline
    "invalid values rejected; stale matching-page reset has no targets; 56 stable typed \
     fields";
  [%expect
    {| invalid values rejected; stale matching-page reset has no targets; 56 stable typed fields |}]
;;
