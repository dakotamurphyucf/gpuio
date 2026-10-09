open Core
module S = Gpuio.Settings

let ok = Or_error.ok_exn
let page_id = Fn.compose ok S.Page_id.of_string
let group_id = Fn.compose ok S.Group_id.of_string
let item_id = Fn.compose ok S.Item_id.of_string

let item ?disabled ?(reset = S.Reset.Dirty) id title =
  S.Item.create ~id:(item_id id) ~title ?disabled ~reset () |> ok
;;

let group id items = S.Group.create ~id:(group_id id) items |> ok

let page ?default_open ?resettable id groups =
  S.Page.create ~id:(page_id id) ~title:id ?default_open ?resettable groups |> ok
;;

let appearance =
  page
    ~default_open:true
    "appearance"
    [ group
        "visual"
        [ item "theme" "Theme"
        ; item ~disabled:true "locked" "Theme policy"
        ; item ~reset:S.Reset.Clean "language" "Language"
        ; S.Item.custom ~id:(item_id "custom") ~keywords:[ "Advanced" ] ~reset:Dirty ()
          |> ok
        ]
    ]
;;

let network = page "network" [ group "connection" [ item "proxy" "Proxy" ] ]
let selected = S.Selection.create ~group:(group_id "connection") (page_id "network")
let initial () = S.create ~selected [ appearance; network ] |> ok
let search t text = S.apply_request t (Search (S.Query.of_string text |> ok))

let visible t =
  List.map (S.filtered_pages t) ~f:(fun p ->
    ( S.Page_id.to_string (S.Page.id p)
    , List.concat_map (S.Page.groups p) ~f:(fun g ->
        List.map (S.Group.items g) ~f:(fun i -> S.Item_id.to_string (S.Item.id i))) ))
;;

let print_selection t = print_s [%sexp (S.selection t : S.Selection.t option)]
let resets t scope = S.reset_targets t ~scope |> List.map ~f:S.Item_id.to_string

let%expect_test "search preserves preferred identity and ignores obsolete navigation" =
  let initial = initial () in
  let filtered = search initial "theme" in
  print_s [%sexp (visible filtered : (string * string list) list)];
  print_selection filtered;
  let filtered = S.apply_request filtered (Select selected) in
  assert (Option.equal S.Selection.equal (S.preferred_selection filtered) (Some selected));
  print_selection (search filtered "no matches");
  print_selection (search filtered "");
  let reordered = S.with_pages initial [ network; appearance ] |> ok in
  print_selection reordered;
  let removed = S.with_pages reordered [ appearance ] |> ok in
  print_s [%sexp (S.preferred_selection removed : S.Selection.t option)];
  print_selection removed;
  [%expect
    {|
    ((appearance (theme locked)))
    (((page appearance) (group ())))
    ()
    (((page network) (group (connection))))
    (((page network) (group (connection))))
    ()
    (((page appearance) (group ())))
    |}]
;;

let%expect_test "Unicode substring matching is per field and custom keywords are explicit"
  =
  let sample =
    S.Item.create
      ~id:(item_id "unicode")
      ~title:"Écran İstanbul"
      ~description:"Local proxy"
      ~keywords:[ "MFA"; "two words" ]
      ()
    |> ok
  in
  let matches query = S.Item.matches sample (S.Query.of_string query |> ok) in
  print_s
    [%sexp
      (List.map
         [ "éCRAN"; "i̇stan"; "MFA"; "two words"; "proxyMFA"; "ecran"; " " ]
         ~f:matches
       : bool list)];
  let custom = S.Item.custom ~id:(item_id "empty") () |> ok in
  print_s
    [%sexp
      (S.Item.matches custom S.Query.empty : bool)
    , (S.Item.matches custom (S.Query.of_string "empty" |> ok) : bool)];
  print_s [%sexp (visible (search (initial ()) "advanced") : (string * string list) list)];
  print_s
    [%sexp (visible (search (initial ()) "connection") : (string * string list) list)];
  [%expect
    {|
    (true true true true false false true)
    (true false)
    ((appearance (custom)))
    ()
    |}]
;;

let%expect_test
    "reset intents resolve latest state with explicit scope and disabled policy"
  =
  let t = search (initial ()) "theme" in
  let requested = S.Reset_scope.Matching_page (page_id "appearance") in
  print_s [%sexp (resets t requested : string list)];
  print_s [%sexp (resets t (Whole_page (page_id "appearance")) : string list)];
  print_s [%sexp (resets t (Matching_group (group_id "visual")) : string list)];
  print_s [%sexp (resets t (Item (item_id "custom")) : string list)];
  (* The user navigates while confirmation is pending: never reset the new page. *)
  print_s [%sexp (resets (search t "proxy") requested : string list)];
  let changed =
    page
      "appearance"
      [ group
          "visual"
          [ item ~reset:Clean "theme" "Theme"; item ~disabled:true "custom" "Custom" ]
      ]
  in
  let changed = S.with_pages t [ changed; network ] |> ok in
  print_s [%sexp (resets changed requested : string list)];
  let prohibited =
    page ~resettable:false "appearance" [ group "visual" [ item "theme" "Theme" ] ]
  in
  let prohibited = S.with_pages t [ prohibited ] |> ok in
  print_s
    [%sexp
      (resets prohibited (Whole_page (page_id "appearance")) : string list)
    , (resets prohibited (Item (item_id "theme")) : string list)];
  [%expect
    {|
    (theme)
    (theme custom)
    (theme)
    (custom)
    ()
    ()
    (() ())
    |}]
;;

let%expect_test
    "removed groups downgrade selection and expansion defaults only apply once"
  =
  let t = S.apply_request (initial ()) (Toggle_page (page_id "appearance")) in
  let t =
    S.with_pages
      t
      [ appearance
      ; page "network" [ group "replacement" [ item "proxy" "Proxy" ] ]
      ; page ~default_open:true "new" []
      ]
    |> ok
  in
  print_selection t;
  print_s [%sexp (List.map (S.expanded_pages t) ~f:S.Page_id.to_string : string list)];
  let hidden = search t "no matches" in
  let hidden = S.apply_request hidden (Toggle_page (page_id "appearance")) in
  print_s
    [%sexp (List.map (S.expanded_pages hidden) ~f:S.Page_id.to_string : string list)];
  assert (Option.is_some (S.find_item hidden (item_id "theme")));
  [%expect
    {|
    (((page network) (group ())))
    (new)
    (new)
    |}]
;;

let%expect_test "catalog bounds and identity validation reject invalid replacements" =
  let rejected result = Or_error.is_error result in
  print_s
    [%sexp
      (List.map
         [ ""; "bad\000id"; "\255"; String.make 257 'x' ]
         ~f:(fun id -> rejected (S.Item_id.of_string id))
       : bool list)];
  print_s [%sexp (rejected (S.Query.of_string (String.make 1025 'q')) : bool)];
  let duplicate_item = page "second" [ group "other" [ item "theme" "Repeated ID" ] ] in
  print_s
    [%sexp
      (rejected (S.create [ appearance; appearance ]) : bool)
    , (rejected (S.create [ appearance; duplicate_item ]) : bool)];
  let bad_selection = S.Selection.create ~group:(group_id "visual") (page_id "network") in
  print_s
    [%sexp (rejected (S.create ~selected:bad_selection [ appearance; network ]) : bool)];
  print_s
    [%sexp
      (rejected (S.create (List.init 129 ~f:(fun i -> page (Int.to_string i) []))) : bool)];
  let many = List.init 1024 ~f:(fun i -> item (Int.to_string i) (String.make 4096 'x')) in
  print_s [%sexp (rejected (S.create [ page "large" [ group "large" many ] ]) : bool)];
  assert (List.is_empty (S.filtered_pages (S.create [ page "empty" [] ] |> ok)));
  [%expect
    {|
    (true true true true)
    true
    (true true)
    true
    true
    true
    |}]
;;
